//! The [`Vm`] type: state, construction and the public entry points.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;

use vba_host::Host;
use vba_ir::IrProgram;
use vba_sema::{ModuleId, ProcId};
use vba_value::{ObjectRef, Variant};

use crate::calls::BoundArg;
use crate::config::{DebugSink, VmConfig};
use crate::declare::DeclareState;
use crate::error::{Fault, RtErr, RuntimeError, VResult, VmError};
use crate::services::Services;
use crate::state::*;

/// How a run finished when it did not fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// The procedure returned.
    Completed,
    /// `End` ran: module-level state was reset and files closed.
    Ended,
    /// `Stop` (or a failed `Debug.Assert`) halted the run.
    Stopped,
}

/// The result of running a procedure.
#[derive(Debug, Clone, PartialEq)]
pub struct RunOutput {
    /// The function result (`Empty` for a `Sub`, or if the run did not
    /// complete).
    pub value: Variant,
    /// How the run finished.
    pub status: RunStatus,
}

/// An argument for a procedure called from outside the VM.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    /// A value, passed as if `ByVal` (a `ByRef` parameter receives a copy).
    Value(Variant),
    /// The omitted-argument marker for an `Optional` parameter.
    Missing,
}

impl From<Variant> for Arg {
    fn from(v: Variant) -> Arg {
        Arg::Value(v)
    }
}

/// The interpreter. One `Vm` holds the complete state of one program:
/// module variables, objects, `Err`, the random generator. It is single
/// threaded and runs to completion of each call it is given.
pub struct Vm {
    pub(crate) ir: Rc<IrProgram>,
    pub(crate) host: Rc<dyn Host>,
    pub(crate) services: Box<dyn Services>,
    pub(crate) debug: Box<dyn DebugSink>,
    pub(crate) std_host: Box<dyn vba_stdlib::Host>,
    pub(crate) std_state: vba_stdlib::State,
    pub(crate) cfg: VmConfig,
    pub(crate) frames: Vec<Frame>,
    pub(crate) globals: Vec<Variant>,
    pub(crate) objects: BTreeMap<u64, Obj>,
    pub(crate) next_obj: u64,
    pub(crate) udts: BTreeMap<u64, UdtVal>,
    pub(crate) next_udt: u64,
    pub(crate) gc_threshold: usize,
    /// Nesting of `alloc_udt` calls in progress (user-defined types that
    /// contain user-defined types).
    pub(crate) udt_depth: u32,
    pub(crate) default_instances: HashMap<ModuleId, u64>,
    pub(crate) err: ErrState,
    pub(crate) erl: u64,
    pub(crate) field_pos: Vec<u32>,
    pub(crate) procs_by_name: HashMap<(ModuleId, String), Vec<ProcId>>,
    pub(crate) pending_events: Rc<RefCell<VecDeque<PendingEvent>>>,
    pub(crate) debug_col: usize,
    pub(crate) declare: DeclareState,
    pub(crate) run_depth: u32,
    pub(crate) stop_reason: Option<String>,
    pub(crate) last_site: Option<(String, u64, Vec<String>)>,
    pub(crate) push_request: Option<Option<vba_ir::Reg>>,
    pub(crate) push_done: bool,
    pub(crate) cascade: Option<crate::state::Cascade>,
    pub(crate) settings: vba_host::MemorySettings,
    pub(crate) bind_stats: BindStats,
    /// Steps until the host is asked whether the run was cancelled.
    pub(crate) poll_countdown: u32,
}

/// How member accesses on objects outside the VM were dispatched.
///
/// An access is *early* when the analysis resolved the receiver's static type
/// to a library type, so the member id is known, and the object accepted that
/// id; it is *by name* otherwise. A *fallback* is an early-bound access whose
/// object did not accept the id (an object that does not provide the
/// library's ids) and was therefore called by name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BindStats {
    /// Calls made by member id.
    pub early: u64,
    /// Calls made by name, including the fallbacks.
    pub by_name: u64,
    /// Early-bound calls that fell back to the name.
    pub fallbacks: u64,
}

impl Vm {
    /// Creates a VM for `ir`.
    ///
    /// Module variables get their default values and fixed-size arrays are
    /// allocated (the standard-module initialisers run); no user procedure
    /// runs, so creating a VM never executes imported code.
    pub fn new(
        ir: Rc<IrProgram>,
        host: Rc<dyn Host>,
        services: Box<dyn Services>,
        std_host: Box<dyn vba_stdlib::Host>,
        debug: Box<dyn DebugSink>,
        cfg: VmConfig,
    ) -> Result<Vm, VmError> {
        let _meter = host.budget().enter();
        let mut vm = Vm {
            ir: ir.clone(),
            host,
            services,
            debug,
            std_host,
            std_state: vba_stdlib::State::default(),
            cfg,
            frames: Vec::new(),
            globals: Vec::new(),
            objects: BTreeMap::new(),
            next_obj: 3,
            udts: BTreeMap::new(),
            next_udt: 1,
            gc_threshold: 4096,
            udt_depth: 0,
            default_instances: HashMap::new(),
            err: ErrState::default(),
            erl: 0,
            field_pos: Vec::new(),
            procs_by_name: HashMap::new(),
            pending_events: Rc::new(RefCell::new(VecDeque::new())),
            debug_col: 0,
            declare: DeclareState::new(),
            run_depth: 0,
            stop_reason: None,
            last_site: None,
            push_request: None,
            push_done: false,
            settings: vba_host::MemorySettings::new(),
            bind_stats: BindStats::default(),
            cascade: None,
            poll_countdown: vba_host::POLL_INTERVAL,
        };
        vm.index_program();
        vm.reset_state().map_err(|f| vm.fault_to_error(f))?;
        Ok(vm)
    }

    fn index_program(&mut self) {
        let ir = self.ir.clone();
        let p = &ir.program;
        self.field_pos = vec![u32::MAX; p.globals.len()];
        for info in ir.classes.iter().flatten() {
            for (i, g) in info.fields.iter().enumerate() {
                self.field_pos[g.index()] = i as u32;
            }
        }
        for proc_ in &p.procs {
            self.procs_by_name
                .entry((proc_.module, proc_.name.to_string().to_ascii_lowercase()))
                .or_default()
                .push(proc_.id);
        }
    }

    /// Resets all module-level state: the effect of `End`, and the initial
    /// state.
    pub(crate) fn reset_state(&mut self) -> VResult<()> {
        self.frames.clear();
        self.objects.clear();
        self.udts.clear();
        self.default_instances.clear();
        self.pending_events.borrow_mut().clear();
        self.err = ErrState::default();
        self.erl = 0;
        self.debug_col = 0;
        let ir = self.ir.clone();
        self.objects.insert(
            ERR_ID,
            Obj {
                kind: ObjKind::Err,
                refs: 1 << 30,
                terminating: false,
                sinks: Vec::new(),
            },
        );
        self.objects.insert(
            DEBUG_ID,
            Obj {
                kind: ObjKind::Debug,
                refs: 1 << 30,
                terminating: false,
                sinks: Vec::new(),
            },
        );
        self.globals = Vec::with_capacity(ir.program.globals.len());
        for g in &ir.program.globals {
            let standard = !ir.program.module(g.module).is_class_like();
            let v = if standard {
                self.default_for(&g.ty)?
            } else {
                Variant::Empty
            };
            self.globals.push(v);
        }
        for m in &ir.program.modules {
            if m.is_class_like() {
                continue;
            }
            if let Some(f) = ir.module_init[m.id.index()] {
                self.run_init(f.0 as usize, None)?;
            }
        }
        Ok(())
    }

    pub(crate) fn fault_to_error(&mut self, f: Fault) -> VmError {
        match f {
            Fault::Err(e) => {
                let site = self.last_site.take().unwrap_or_default();
                VmError::Runtime(Box::new(RuntimeError {
                    error: e,
                    procedure: site.0,
                    line: site.1,
                    stack: site.2,
                }))
            }
            Fault::End | Fault::Stop => VmError::Internal("End/Stop escaped the VM".into()),
            Fault::Limit(l) => VmError::Limit(l),
            Fault::Cancelled(s) => VmError::Cancelled(s),
            Fault::Unsupported(s) => VmError::Unsupported(s),
            Fault::Bridge(s) => VmError::Bridge(s),
            Fault::Internal(s) => VmError::Internal(s),
        }
    }

    /// The program the VM runs.
    pub fn program(&self) -> &IrProgram {
        &self.ir
    }

    /// Finds a procedure by `Module.Name` or by a bare name (the first match
    /// in module order; case-insensitive).
    pub fn find_procedure(&self, name: &str) -> Option<ProcId> {
        let p = &self.ir.program;
        if let Some((m, n)) = name.split_once('.') {
            let m = p.module_by_name(m)?;
            return self
                .procs_by_name
                .get(&(m.id, n.to_ascii_lowercase()))
                .and_then(|v| v.first().copied());
        }
        let n = name.to_ascii_lowercase();
        p.modules
            .iter()
            .filter_map(|m| self.procs_by_name.get(&(m.id, n.clone())))
            .find_map(|v| v.first().copied())
    }

    /// Runs a procedure of a standard module with the given arguments.
    ///
    /// `End` resets the module state and reports [`RunStatus::Ended`];
    /// `Stop` reports [`RunStatus::Stopped`]. Any other failure is a
    /// [`VmError`]; the VM stays usable afterwards (module variables keep
    /// their values).
    pub fn run_procedure(&mut self, proc_: ProcId, args: Vec<Arg>) -> Result<RunOutput, VmError> {
        let module = self.ir.program.proc(proc_).module;
        if self.ir.program.module(module).is_class_like() {
            return Err(VmError::Unsupported(format!(
                "`{}` belongs to a class module; use run_method with an instance",
                self.ir.program.proc(proc_).name
            )));
        }
        self.run_top(proc_, None, args)
    }

    /// Runs a method of a class instance (a `Property Get` returns its
    /// value).
    pub fn run_method(
        &mut self,
        object: ObjectRef,
        proc_: ProcId,
        args: Vec<Arg>,
    ) -> Result<RunOutput, VmError> {
        let ObjectRef::Handle(h) = object else {
            return Err(VmError::Runtime(Box::new(RuntimeError {
                error: RtErr::standard(91),
                procedure: String::new(),
                line: 0,
                stack: Vec::new(),
            })));
        };
        let Some(id) = vm_id(h) else {
            return Err(VmError::Unsupported(
                "run_method needs an object created by the VM".into(),
            ));
        };
        self.run_top(proc_, Some(id), args)
    }

    fn run_top(
        &mut self,
        proc_: ProcId,
        me: Option<u64>,
        args: Vec<Arg>,
    ) -> Result<RunOutput, VmError> {
        let bound: Vec<BoundArg> = args
            .into_iter()
            .map(|a| match a {
                Arg::Value(v) => BoundArg::Val(v),
                Arg::Missing => BoundArg::Missing,
            })
            .collect();
        self.last_site = None;
        self.stop_reason = None;
        let _meter = self.host.budget().enter();
        let r = self.call_nested(proc_, me, bound);
        match r {
            Ok(v) => Ok(RunOutput {
                value: v,
                status: RunStatus::Completed,
            }),
            Err(Fault::End) => {
                self.frames.clear();
                self.services.reset();
                self.reset_state().map_err(|f| self.fault_to_error(f))?;
                Ok(RunOutput {
                    value: Variant::Empty,
                    status: RunStatus::Ended,
                })
            }
            Err(Fault::Stop) => {
                self.frames.clear();
                Ok(RunOutput {
                    value: Variant::Empty,
                    status: RunStatus::Stopped,
                })
            }
            Err(f) => {
                self.frames.clear();
                Err(self.fault_to_error(f))
            }
        }
    }

    /// The predeclared default instance of a class-like module, created (and
    /// `Class_Initialize` run) on first use.
    pub fn default_instance_object(&mut self, module: ModuleId) -> Result<ObjectRef, VmError> {
        self.last_site = None;
        let _meter = self.host.budget().enter();
        let r = self.default_instance(module);
        match r {
            Ok(id) => Ok(ObjectRef::Handle(vm_handle(id))),
            Err(f) => {
                self.frames.clear();
                Err(self.fault_to_error(f))
            }
        }
    }

    /// The reason of the last `Stop`, for example a failed `Debug.Assert`.
    pub fn stop_reason(&self) -> Option<&str> {
        self.stop_reason.as_deref()
    }

    /// The current `Err` contents `(number, description)`.
    pub fn last_error(&self) -> (i32, &str) {
        (self.err.number, &self.err.description)
    }

    /// Releases every module-level variable and default instance in
    /// declaration order, running `Class_Terminate` of objects that become
    /// unreferenced (the orderly end of a session).
    pub fn shutdown(&mut self) -> Result<(), VmError> {
        let _meter = self.host.budget().enter();
        let r = self.shutdown_inner();
        r.map_err(|f| self.fault_to_error(f))
    }

    /// Number of live objects owned by the VM (the built-in `Err` and
    /// `Debug` objects are not counted).
    pub fn live_objects(&self) -> usize {
        self.objects.len().saturating_sub(2)
    }

    /// How member accesses on objects outside the VM were dispatched so far.
    pub fn bind_stats(&self) -> BindStats {
        self.bind_stats
    }

    /// Number of live user-defined-type values.
    pub fn live_udts(&self) -> usize {
        self.udts.len()
    }

    /// The module-level variables of a standard module that currently hold
    /// a value, as `(name, value)` (for diagnostics and tests).
    pub fn module_variables(&self, module: ModuleId) -> Vec<(String, Variant)> {
        self.ir
            .program
            .globals
            .iter()
            .filter(|g| g.module == module && g.static_of.is_none())
            .filter_map(|g| {
                let v = self.globals.get(g.id.index())?;
                Some((g.name.to_string(), v.clone()))
            })
            .collect()
    }
}
