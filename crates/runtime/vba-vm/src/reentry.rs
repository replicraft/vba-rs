//! Re-entrancy: a host object that is executing a call made by VBA code
//! can call back into the same session.
//!
//! # How it works
//!
//! The interpreter owns all session state through `&mut Vm`, so a host
//! object cannot simply be handed a second reference. Instead, when VBA
//! code calls a member of an object that declares
//! [`Dispatch::reentrant`](vba_automation::Dispatch::reentrant), the VM
//! *suspends itself*: its whole state is moved into a [`ReentrySlot`] that
//! is reachable from the [`ObjectTable`] the host object receives. The host
//! calls [`ReentrySlot::with_vm`]; that takes the VM out of the slot, runs
//! the nested operation on it (new instances, event handlers, procedure
//! calls, variable access), and puts it back. When the host call returns,
//! the VM moves back to where it was and the interrupted VBA statement
//! continues. No `unsafe` is involved and no state is copied.
//!
//! Rules:
//!
//! * Nested runs share the budget (steps, memory, call depth), the `Err`
//!   object, module state and `Debug.Print` sink of the outer run.
//! * Nesting is bounded by [`VmConfig::max_reentrancy_depth`]; beyond it
//!   [`ReentrySlot::with_vm`] fails with [`ReentryError::TooDeep`].
//! * An unhandled VBA error in a nested run is returned to the host
//!   (`Err`); if the host returns it from its own member it reaches the
//!   outer VBA code as an ordinary, trappable run-time error.
//! * A fatal fault in a nested run (resource limit, cancellation, `End`,
//!   `Stop`, unsupported feature) is also returned to the host, **and**
//!   remembered: when the host call returns, the outer run ends with the
//!   same fault, whatever the host did with the error. `On Error` cannot
//!   swallow it.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use vba_automation::{DispError, ObjectTable};

use crate::config::{NullSink, VmConfig};
use crate::declare::DeclareState;
use crate::error::{LimitKind, VmError};
use crate::services::NoServices;
use crate::state::ErrState;
use crate::vm::{BindStats, Vm};

/// Why a nested operation could not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReentryError {
    /// The interpreter is not suspended in a host call: the handle was kept
    /// past the call that produced it, or the host is running outside any
    /// VBA call.
    NotSuspended,
    /// [`VmConfig::max_reentrancy_depth`] nested runs are already active.
    TooDeep {
        /// The configured limit.
        max: u32,
    },
}

impl std::fmt::Display for ReentryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReentryError::NotSuspended => f.write_str(
                "the session is not suspended in a host call (a re-entrant handle is only valid while the host member that received it runs)",
            ),
            ReentryError::TooDeep { max } => {
                write!(f, "host re-entrancy depth limit ({max}) exceeded")
            }
        }
    }
}

impl std::error::Error for ReentryError {}

/// The place a suspended VM waits in. One per VM, reachable from its
/// [`ObjectTable`] with [`ReentrySlot::from_table`].
pub struct ReentrySlot {
    vm: RefCell<Option<Box<Vm>>>,
    depth: Cell<u32>,
}

impl ReentrySlot {
    pub(crate) fn new() -> Rc<ReentrySlot> {
        Rc::new(ReentrySlot {
            vm: RefCell::new(None),
            depth: Cell::new(0),
        })
    }

    /// The slot of the VM that runs against `table`, if any.
    pub fn from_table(table: &ObjectTable) -> Option<Rc<ReentrySlot>> {
        table.link()?.downcast::<ReentrySlot>().ok()
    }

    /// Whether the VM is suspended in a host call right now.
    pub fn is_suspended(&self) -> bool {
        self.vm.try_borrow().map(|v| v.is_some()).unwrap_or(false)
    }

    /// How many nested runs are active.
    pub fn depth(&self) -> u32 {
        self.depth.get()
    }

    /// Runs `f` on the suspended VM; the VM is back in the slot when this
    /// returns (also if `f` panics).
    pub fn with_vm<R>(&self, f: impl FnOnce(&mut Vm) -> R) -> Result<R, ReentryError> {
        let Some(mut vm) = self.vm.borrow_mut().take() else {
            return Err(ReentryError::NotSuspended);
        };
        let max = vm.cfg.max_reentrancy_depth;
        if self.depth.get() >= max {
            *self.vm.borrow_mut() = Some(vm);
            return Err(ReentryError::TooDeep { max });
        }
        self.depth.set(self.depth.get() + 1);
        vm.nesting += 1;
        struct Restore<'a> {
            slot: &'a ReentrySlot,
            vm: Option<Box<Vm>>,
        }
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                if let Some(mut vm) = self.vm.take() {
                    vm.nesting -= 1;
                    *self.slot.vm.borrow_mut() = Some(vm);
                }
                self.slot.depth.set(self.slot.depth.get() - 1);
            }
        }
        let mut guard = Restore {
            slot: self,
            vm: Some(vm),
        };
        let vm = guard.vm.as_mut().expect("present");
        Ok(f(vm))
    }
}

impl Vm {
    /// A placeholder that takes the place of the VM while it sits in the
    /// slot. It owns nothing and cannot run anything.
    fn hollow(&self) -> Vm {
        Vm {
            ir: self.ir.clone(),
            host: self.host.clone(),
            services: Box::new(NoServices),
            debug: Box::new(NullSink),
            std_host: Box::new(vba_stdlib::FixedHost::default()),
            std_state: vba_stdlib::State::default(),
            cfg: VmConfig {
                project_name: String::new(),
                ..VmConfig::default()
            },
            frames: Vec::new(),
            globals: Vec::new(),
            objects: BTreeMap::new(),
            next_obj: 0,
            udts: BTreeMap::new(),
            next_udt: 0,
            gc_threshold: usize::MAX,
            udt_depth: 0,
            default_instances: HashMap::new(),
            err: ErrState::default(),
            erl: 0,
            field_pos: Vec::new(),
            procs_by_name: HashMap::new(),
            pending_events: self.pending_events.clone(),
            debug_col: 0,
            declare: DeclareState::new(),
            run_depth: 0,
            native_base: 0,
            nesting: 0,
            poll_countdown: vba_host::POLL_INTERVAL,
            stop_reason: None,
            last_site: None,
            source_lines: Vec::new(),
            push_request: None,
            push_done: false,
            cascade: None,
            settings: vba_host::MemorySettings::new(),
            bind_stats: BindStats::default(),
            slot: self.slot.clone(),
            pending_fatal: None,
        }
    }

    /// Runs `f` (a call into a re-entrant host object) with this VM
    /// suspended in its slot.
    pub(crate) fn suspended<R>(&mut self, f: impl FnOnce() -> R) -> R {
        let slot = self.slot.clone();
        let hollow = self.hollow();
        let me = std::mem::replace(self, hollow);
        *slot.vm.borrow_mut() = Some(Box::new(me));
        let r = f();
        let back = slot.vm.borrow_mut().take();
        *self = *back.expect("the suspended VM was lost");
        r
    }
}

/// The error a host gets back from a nested run, as the Automation error it
/// can return from its own member: the VBA number, description, source and
/// help of an unhandled error; a stand-in number for the fatal faults, which
/// the outer run re-raises untrappably anyway.
pub fn vm_error_to_disp(e: &VmError) -> DispError {
    match e {
        VmError::Runtime(r) => DispError::Exception(vba_automation::ExcepInfo {
            code: 0,
            scode: vba_automation::hresult::number_to_hresult(r.error.number),
            source: r.error.source.clone(),
            description: r.error.description.clone(),
            help_file: r.error.help_file.clone(),
            help_context: r.error.help_context as u32,
        }),
        VmError::Limit(l) => {
            let n = match l {
                LimitKind::Steps => 18,
                LimitKind::Memory | LimitKind::StringLength | LimitKind::ArraySize => 7,
                LimitKind::CallDepth => 28,
            };
            DispError::raise(n, l.to_string(), "vba")
        }
        VmError::Cancelled(s) => DispError::raise(18, format!("cancelled: {s}"), "vba"),
        other => DispError::raise(5, other.to_string(), "vba"),
    }
}
