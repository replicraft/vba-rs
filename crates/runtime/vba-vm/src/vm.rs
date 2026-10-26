//! The [`Vm`] type: state, construction and the public entry points.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;

use vba_host::Host;
use vba_ir::IrProgram;
use vba_sema::{ModuleId, ProcId};
use vba_value::{ObjectRef, Variant};

use crate::config::{DebugSink, VmConfig};
use crate::declare::DeclareState;
use crate::error::{Fault, RtErr, RuntimeError, VResult, VmError};
use crate::services::Services;
use crate::state::*;

/// How a run finished when it did not fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct RunOutput {
    /// The function result (`Empty` for a `Sub`, or if the run did not
    /// complete).
    pub value: Variant,
    /// How the run finished.
    pub status: RunStatus,
    /// The final value of every argument passed as [`Arg::ByRef`], in the
    /// order of the arguments (which is the order of the parameters they
    /// bound to). A parameter declared `ByVal` keeps the value the caller
    /// passed; a `ByRef` parameter holds what the procedure left in it,
    /// converted to the declared type. Empty when the run did not complete.
    pub byref: Vec<Variant>,
}

/// An argument for a procedure called from outside the VM.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Arg {
    /// A value, passed as if `ByVal` (a `ByRef` parameter receives a copy).
    Value(Variant),
    /// The omitted-argument marker for an `Optional` parameter.
    Missing,
    /// A caller-owned variable passed by reference: a `ByRef` parameter
    /// writes through to it and its final value is returned in
    /// [`RunOutput::byref`]. The value is first converted to the declared
    /// type of the parameter (error 13 when it cannot be). A `ByVal`
    /// parameter, or the elements of a `ParamArray`, receive a copy and do
    /// not write back.
    ByRef(Variant),
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
    /// Emptied slot and register vectors of popped frames, reused by the
    /// next call so a call does not allocate.
    pub(crate) slot_pool: Vec<Vec<Slot>>,
    pub(crate) reg_pool: Vec<Vec<RegVal>>,
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
    /// The same procedures as `procs_by_name`, per module (by index) and
    /// looked up by a borrowed lower-case name, without allocating.
    pub(crate) procs_lc: Vec<HashMap<String, Vec<ProcId>>>,
    /// Public-field candidates per module (by index): the first variable of
    /// each lower-case name that is not a hoisted `Static` local.
    pub(crate) fields_lc: Vec<HashMap<String, (vba_sema::GlobalId, vba_ast::Visibility)>>,
    pub(crate) procs_by_name: HashMap<(ModuleId, String), Vec<ProcId>>,
    pub(crate) pending_events: Rc<RefCell<VecDeque<PendingEvent>>>,
    pub(crate) debug_col: usize,
    pub(crate) declare: DeclareState,
    pub(crate) run_depth: u32,
    /// The stack address seen by the outermost active run; nested runs
    /// measure their distance from it.
    pub(crate) native_base: usize,
    /// How many host calls the VM is suspended in (nested embedder runs).
    pub(crate) nesting: u32,
    /// The place this VM waits in while a re-entrant host object runs.
    pub(crate) slot: Rc<crate::reentry::ReentrySlot>,
    /// A fatal fault (limit, cancellation, `End`, `Stop`...) raised inside a
    /// nested run; the outer run ends with it when the host call returns.
    pub(crate) pending_fatal: Option<Fault>,
    pub(crate) stop_reason: Option<String>,
    pub(crate) last_site: Option<Site>,
    /// Source line of each statement, per function (see `set_source_lines`).
    pub(crate) source_lines: Vec<Vec<u32>>,
    pub(crate) push_request: Option<Option<vba_ir::Reg>>,
    pub(crate) push_done: bool,
    /// The library interface type a late-bound access was written against,
    /// for the duration of that access.
    pub(crate) iface_hint: Option<vba_sema::LibraryType>,
    pub(crate) cascade: Option<crate::state::Cascade>,
    pub(crate) settings: vba_host::MemorySettings,
    pub(crate) bind_stats: BindStats,
    /// Step count at the last poll of the host for cancellation.
    pub(crate) polled_at: u64,
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
            slot_pool: Vec::new(),
            reg_pool: Vec::new(),
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
            procs_lc: Vec::new(),
            fields_lc: Vec::new(),
            pending_events: Rc::new(RefCell::new(VecDeque::new())),
            debug_col: 0,
            declare: DeclareState::new(),
            run_depth: 0,
            native_base: 0,
            nesting: 0,
            slot: crate::reentry::ReentrySlot::new(),
            pending_fatal: None,
            stop_reason: None,
            last_site: None,
            source_lines: Vec::new(),
            push_request: None,
            push_done: false,
            iface_hint: None,
            settings: vba_host::MemorySettings::new(),
            bind_stats: BindStats::default(),
            cascade: None,
            polled_at: 0,
        };
        vm.index_program();
        vm.host.objects().set_link(vm.slot.clone());
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
        self.procs_lc = vec![HashMap::new(); p.modules.len()];
        self.fields_lc = vec![HashMap::new(); p.modules.len()];
        for g in &p.globals {
            if g.static_of.is_none() {
                self.fields_lc[g.module.index()]
                    .entry(g.name.to_string().to_ascii_lowercase())
                    .or_insert((g.id, g.visibility));
            }
        }
        for proc_ in &p.procs {
            self.procs_lc[proc_.module.index()]
                .entry(proc_.name.to_string().to_ascii_lowercase())
                .or_default()
                .push(proc_.id);
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
        // Every `AddressOf` token of the old session stops being valid and
        // timers set with SetTimer are dropped.
        self.declare.callbacks.invalidate_all();
        if let Some(ctx) = self.declare.ctx.as_mut() {
            ctx.timers.clear();
        }
        let ir = self.ir.clone();
        self.objects.insert(
            ERR_ID,
            Obj {
                kind: ObjKind::Err,
                refs: 1 << 30,
                terminating: false,
                sinks: Vec::new(),
                host_base: None,
            },
        );
        self.objects.insert(
            DEBUG_ID,
            Obj {
                kind: ObjKind::Debug,
                refs: 1 << 30,
                terminating: false,
                sinks: Vec::new(),
                host_base: None,
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
            Fault::Err(mut e) => {
                // Like `Err.Source`: the project name unless the error
                // names a source.
                if e.source.is_empty() {
                    e.source = self.cfg.project_name.clone();
                }
                let site = self.last_site.take().unwrap_or_default();
                VmError::Runtime(Box::new(RuntimeError {
                    error: *e,
                    procedure: site.procedure,
                    line: site.line,
                    stack: site.stack,
                    module: site.module,
                    source_line: site.source_line,
                    frames: site.frames,
                    last_dll_error: self.err.last_dll_error,
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

    /// Tells the VM the 1-based source line of every statement, indexed by
    /// function and then by statement, so that errors can report lines. The
    /// VM does not own the source text; the embedding layer computes this
    /// table (missing entries read as unknown).
    pub fn set_source_lines(&mut self, lines: Vec<Vec<u32>>) {
        self.source_lines = lines;
    }

    /// The program the VM runs.
    pub fn program(&self) -> &IrProgram {
        &self.ir
    }

    /// A shared handle to the program the VM runs.
    pub fn program_rc(&self) -> Rc<IrProgram> {
        self.ir.clone()
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
            return Err(VmError::Runtime(Box::new(RuntimeError::new(
                RtErr::standard(91),
            ))));
        };
        let Some(id) = vm_id(h) else {
            return Err(VmError::Unsupported(
                "run_method needs an object created by the VM".into(),
            ));
        };
        self.run_top(proc_, Some(id), args)
    }

    pub(crate) fn run_top(
        &mut self,
        proc_: ProcId,
        me: Option<u64>,
        args: Vec<Arg>,
    ) -> Result<RunOutput, VmError> {
        self.last_site = None;
        self.stop_reason = None;
        let _meter = self.host.budget().enter();
        if let Err(f) = self.poll_now() {
            return if self.nesting > 0 {
                self.finish_nested(Err(f), Vec::new())
            } else {
                Err(self.fault_to_error(f))
            };
        }
        let temp_base = self.globals.len();
        let (bound, outs) = match self.prepare_args(proc_, args) {
            Ok(x) => x,
            Err(f) => {
                self.frames.clear();
                return Err(self.fault_to_error(f));
            }
        };
        let r = self.call_nested(proc_, me, bound);
        let byref = self.collect_byref(temp_base, outs, matches!(r, Ok(_) | Err(Fault::Stop)));
        if self.nesting > 0 {
            return self.finish_nested(r, byref);
        }
        match r {
            Ok(v) => Ok(RunOutput {
                value: v,
                status: RunStatus::Completed,
                byref,
            }),
            Err(Fault::End) => {
                self.frames.clear();
                self.services.reset();
                self.reset_state().map_err(|f| self.fault_to_error(f))?;
                Ok(RunOutput {
                    value: Variant::Empty,
                    status: RunStatus::Ended,
                    byref: Vec::new(),
                })
            }
            Err(Fault::Stop) => {
                self.frames.clear();
                Ok(RunOutput {
                    value: Variant::Empty,
                    status: RunStatus::Stopped,
                    byref,
                })
            }
            Err(f) => {
                self.frames.clear();
                Err(self.fault_to_error(f))
            }
        }
    }

    /// The end of a run that was started by a host object while the VM was
    /// suspended in a call of that object. The outer frames must survive,
    /// so nothing is cleared; a fatal fault is remembered for the outer
    /// run (see the module documentation of `reentry`).
    fn finish_nested(
        &mut self,
        r: VResult<Variant>,
        byref: Vec<Variant>,
    ) -> Result<RunOutput, VmError> {
        match r {
            Ok(value) => Ok(RunOutput {
                value,
                status: RunStatus::Completed,
                byref,
            }),
            Err(Fault::End) => {
                self.pending_fatal = Some(Fault::End);
                Ok(RunOutput {
                    value: Variant::Empty,
                    status: RunStatus::Ended,
                    byref: Vec::new(),
                })
            }
            Err(Fault::Stop) => {
                self.pending_fatal = Some(Fault::Stop);
                Ok(RunOutput {
                    value: Variant::Empty,
                    status: RunStatus::Stopped,
                    byref,
                })
            }
            Err(e @ Fault::Err(_)) => Err(self.fault_to_error(e)),
            Err(f) => {
                self.pending_fatal = Some(f.clone());
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
            Err(f) => Err(self.finish_fault(f)),
        }
    }

    /// The module a VM-owned object is an instance of, `None` for `Nothing`,
    /// host objects, stale handles and the built-in objects.
    pub fn object_module(&self, object: &ObjectRef) -> Option<ModuleId> {
        let ObjectRef::Handle(h) = object else {
            return None;
        };
        match self.objects.get(&vm_id(*h)?)?.kind {
            ObjKind::Class { module, .. } => Some(module),
            _ => None,
        }
    }

    /// The reason of the last `Stop`, for example a failed `Debug.Assert`.
    pub fn stop_reason(&self) -> Option<&str> {
        self.stop_reason.as_deref()
    }

    /// The error `Err` currently holds, with where it was raised, or `None`
    /// when `Err.Number` is 0. This is how the embedder sees an error the
    /// program handled (`On Error Resume Next`, `On Error GoTo`); an
    /// unhandled one ends the call with a [`VmError`].
    pub fn current_error(&self) -> Option<RuntimeError> {
        if self.err.number == 0 {
            return None;
        }
        let site = self.err.site.clone().unwrap_or_default();
        Some(RuntimeError {
            error: RtErr {
                number: self.err.number,
                description: self.err.description.clone(),
                source: self.err.source.clone(),
                help_file: self.err.help_file.clone(),
                help_context: self.err.help_context,
                origin: crate::error::ErrOrigin::Vba,
            },
            procedure: site.procedure,
            line: if site.line > 0 { site.line } else { self.erl },
            stack: site.stack,
            module: site.module,
            source_line: site.source_line,
            frames: site.frames,
            last_dll_error: self.err.last_dll_error,
        })
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
