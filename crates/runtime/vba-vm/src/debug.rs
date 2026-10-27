//! Debugger support: breakpoints, stepping, the pause callback and the
//! read-only view of a paused program.
//!
//! The debugger is opt-in and costs nothing when unused: with no hook and no
//! breakpoint installed the interpreter only tests one `Option` where a new
//! statement starts, and no step is ever charged for debugging. Attaching a
//! debugger changes neither the step count nor any behaviour of the program
//! (pausing is invisible to it, apart from the time it takes).
//!
//! # Where the interpreter pauses
//!
//! At the first instruction of a statement whose source line differs from the
//! line of the statement that ran before it in the same frame ("line
//! events"; several statements on one line, `a = 1: b = 2`, pause once). A
//! pause happens for
//!
//! * a **breakpoint** on that line,
//! * a **step** request that is due (`StepInto`: the next line event of any
//!   frame; `StepOver`: the next one of the same or an outer frame;
//!   `StepOut`: the next one of an outer frame),
//! * a `Stop` statement or a failed `Debug.Assert` (these halt the run when
//!   no hook is attached; with a hook they pause and the program continues
//!   after the hook returns `Continue`).
//!
//! The callback runs on the interpreter's thread, inside the run, and returns
//! what to do next. While it runs the wall-clock deadline of the call is
//! stopped (see `Budget::suspend_deadline`), so a person thinking at a
//! breakpoint does not use up the call timeout. Steps are not charged for the
//! pause.
//!
//! Statements of module initialisers and of code run by [`Vm::eval`] never
//! pause.

use std::collections::HashSet;

use vba_ast::ProcKind;
use vba_ir::FuncKind;
use vba_sema::{LocalKind, ModuleId, Type};
use vba_value::{ObjectHandle, ObjectRef, Variant};

use crate::error::{Fault, VResult};
use crate::state::{ObjKind, Slot, vm_id};
use crate::vm::Vm;

/// Why the program paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PauseReason {
    /// A breakpoint on the line about to run.
    Breakpoint,
    /// A step request completed (including `request_step`).
    Step,
    /// A `Stop` statement.
    Stop,
    /// A `Debug.Assert` whose condition was false.
    AssertFailed,
}

/// What the interpreter does when the pause callback returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResumeAction {
    /// Run until the next breakpoint, `Stop` or failed assertion.
    Continue,
    /// Pause at the next line event, entering called procedures.
    StepInto,
    /// Pause at the next line event of this frame or an outer one.
    StepOver,
    /// Pause at the next line event of an outer frame.
    StepOut,
    /// End the run now. The run fails with a cancellation error (untrappable
    /// by `On Error`), like a cancelled call.
    Abort,
}

/// The receiver of pauses. Implement it on your own type, or pass a closure.
///
/// The callback runs on the interpreter thread while the run is suspended; the
/// [`DebugContext`] it gets is only valid for the duration of the call.
pub trait DebugHook {
    /// The program is paused; inspect it through `ctx` and say how to go on.
    fn paused(&mut self, ctx: &mut DebugContext<'_>) -> ResumeAction;
}

impl<F> DebugHook for F
where
    F: FnMut(&mut DebugContext<'_>) -> ResumeAction,
{
    fn paused(&mut self, ctx: &mut DebugContext<'_>) -> ResumeAction {
        self(ctx)
    }
}

/// The outcome of resolving one requested breakpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BreakpointInfo {
    /// The module as requested.
    pub module: String,
    /// The line as requested (1-based).
    pub requested: u32,
    /// The line the breakpoint sits on: the requested line when it holds a
    /// statement, otherwise the next line with a statement in the same
    /// procedure. `None` when no statement can be found (a comment between
    /// procedures, a declaration, an unknown module); nothing is installed.
    pub line: Option<u32>,
}

/// One frame of the call stack of a paused program.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FrameInfo {
    /// The module.
    pub module: String,
    /// The procedure (`"(initialiser)"` for a module initialiser).
    pub procedure: String,
    /// The kind of the procedure, `None` for an initialiser.
    pub kind: Option<ProcKind>,
    /// The 1-based source line of the statement the frame is at (the call
    /// for a frame that is waiting for its callee); 0 when unknown.
    pub line: u32,
    /// `Erl`: the last line-number label the frame passed, 0 when none.
    pub erl: u64,
    /// Whether the frame has a `Me` (a method of a class-like module).
    pub has_me: bool,
}

/// How a variable came to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VariableKind {
    /// A parameter.
    Parameter,
    /// A local variable (declared or implicit).
    Local,
    /// The implicit variable that holds a function's result.
    Result,
    /// A `Static` local.
    Static,
    /// A module-level variable of a standard module.
    ModuleLevel,
    /// A field of a class instance.
    Field,
}

/// A variable, read-only. Objects are `Variant::Object` handles; ask the
/// context to describe them ([`DebugContext::describe`]).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Variable {
    /// The declared name.
    pub name: String,
    /// The declared type, spelled like VBA (`Long`, `String * 10`,
    /// `Long()`, a class or type name); `Variant` when undeclared.
    pub type_name: String,
    /// The current value. An omitted `Optional` argument reads as the
    /// "missing" error value, as `IsMissing` sees it.
    pub value: Variant,
    /// Where the variable comes from.
    pub kind: VariableKind,
}

#[derive(Debug, Clone, Copy)]
enum StepMode {
    Run,
    Into,
    Over(usize),
    Out(usize),
}

pub(crate) struct DebugState {
    hook: Option<Box<dyn DebugHook>>,
    /// `(function index, source line)` pairs that hold a breakpoint.
    breakpoints: HashSet<(usize, u32)>,
    /// What the host asked for, to resolve again when the program changes.
    wanted: Vec<(String, u32)>,
    mode: StepMode,
    /// `(depth, function, line)` of the last statement seen.
    last: Option<(usize, usize, u32)>,
    /// Greater than zero while code run by `eval` executes.
    pub(crate) suppress: u32,
}

impl DebugState {
    pub(crate) fn wanted_breakpoints(&self) -> Vec<(String, u32)> {
        self.wanted.clone()
    }

    fn new() -> DebugState {
        DebugState {
            hook: None,
            breakpoints: HashSet::new(),
            wanted: Vec::new(),
            mode: StepMode::Run,
            last: None,
            suppress: 0,
        }
    }
}

// ---- installation -----------------------------------------------------------------

impl Vm {
    /// Attaches (`Some`) or detaches (`None`) the pause callback. Breakpoints
    /// stay installed across the change.
    pub fn set_debug_hook(&mut self, hook: Option<Box<dyn DebugHook>>) {
        match hook {
            Some(h) => {
                self.debugger
                    .get_or_insert_with(|| Box::new(DebugState::new()))
                    .hook = Some(h);
            }
            None => {
                if let Some(d) = self.debugger.as_mut() {
                    d.hook = None;
                    d.mode = StepMode::Run;
                    d.last = None;
                    if d.breakpoints.is_empty() && d.wanted.is_empty() {
                        self.debugger = None;
                    }
                }
            }
        }
    }

    /// Whether a pause callback is attached.
    pub fn debug_hook_attached(&self) -> bool {
        self.debugger.as_ref().is_some_and(|d| d.hook.is_some())
    }

    /// Replaces the breakpoints. `wanted` are `(module, line)` pairs (module
    /// name case-insensitive, line 1-based). Returns where each one ended up;
    /// see [`BreakpointInfo::line`].
    pub fn set_breakpoints(&mut self, wanted: &[(String, u32)]) -> Vec<BreakpointInfo> {
        let mut out = Vec::with_capacity(wanted.len());
        let mut set = HashSet::new();
        for (module, line) in wanted {
            let resolved = self.resolve_breakpoint(module, *line);
            if let Some((func, l)) = resolved {
                // Every function holding a statement on that line (a line
                // can belong to one procedure only, but be exact).
                set.insert((func, l));
            }
            out.push(BreakpointInfo {
                module: module.clone(),
                requested: *line,
                line: resolved.map(|(_, l)| l),
            });
        }
        if set.is_empty() && wanted.is_empty() && !self.debug_hook_attached() {
            self.debugger = None;
            return out;
        }
        let d = self
            .debugger
            .get_or_insert_with(|| Box::new(DebugState::new()));
        d.breakpoints = set;
        d.wanted = wanted.to_vec();
        out
    }

    /// Resolves a requested breakpoint to `(function, line)`.
    fn resolve_breakpoint(&self, module: &str, line: u32) -> Option<(usize, u32)> {
        let m = self.ir.program.module_by_name(module)?.id;
        for (fi, f) in self.ir.functions.iter().enumerate() {
            let FuncKind::Proc(p) = f.kind else { continue };
            if self.ir.program.proc(p).module != m {
                continue;
            }
            let &(start, end) = self.proc_lines.get(fi)?;
            if line < start || line > end {
                continue;
            }
            let lines = self.source_lines.get(fi)?;
            let best = lines
                .iter()
                .copied()
                .filter(|&l| l >= line && l <= end)
                .min();
            return best.map(|l| (fi, l));
        }
        None
    }

    /// The source lines of `module` that hold a statement a breakpoint can
    /// sit on, ascending.
    pub fn breakable_lines(&self, module: &str) -> Vec<u32> {
        let Some(m) = self.ir.program.module_by_name(module).map(|m| m.id) else {
            return Vec::new();
        };
        let mut lines: Vec<u32> = Vec::new();
        for (fi, f) in self.ir.functions.iter().enumerate() {
            let FuncKind::Proc(p) = f.kind else { continue };
            if self.ir.program.proc(p).module != m {
                continue;
            }
            if let Some(ls) = self.source_lines.get(fi) {
                lines.extend(ls.iter().copied().filter(|&l| l > 0));
            }
        }
        lines.sort_unstable();
        lines.dedup();
        lines
    }

    /// Tells the VM the first and last source line of every function
    /// (indexed like [`Vm::set_source_lines`]); used to place breakpoints.
    pub fn set_procedure_lines(&mut self, lines: Vec<(u32, u32)>) {
        self.proc_lines = lines;
        if let Some(d) = self.debugger.as_ref() {
            let wanted = d.wanted.clone();
            if !wanted.is_empty() {
                self.set_breakpoints(&wanted);
            }
        }
    }

    /// Makes the next line event pause, as if a step had been requested
    /// (the way to "step into" the first statement of a run). Needs a hook.
    pub fn request_step(&mut self) {
        if let Some(d) = self.debugger.as_mut() {
            d.mode = StepMode::Into;
            d.last = None;
        }
    }

    /// Forgets stepping state and the last line seen; the facade calls it at
    /// the start of each run so a run begins with `Continue` semantics.
    pub fn clear_stepping(&mut self) {
        if let Some(d) = self.debugger.as_mut() {
            d.mode = StepMode::Run;
            d.last = None;
        }
    }
}

// ---- the interpreter side -------------------------------------------------------------

impl Vm {
    /// Called when a statement begins (the frame's `cur_stmt` just changed).
    #[cold]
    #[inline(never)]
    pub(crate) fn debug_boundary(&mut self, stmt: vba_ir::StmtId) -> VResult<()> {
        let depth = self.frames.len();
        let func = self.frames[depth - 1].func;
        let Some(d) = self.debugger.as_mut() else {
            return Ok(());
        };
        if d.suppress > 0 || d.hook.is_none() {
            return Ok(());
        }
        if matches!(self.ir.functions[func].kind, FuncKind::ModuleInit(_)) {
            return Ok(());
        }
        let line = self
            .source_lines
            .get(func)
            .and_then(|l| l.get(stmt.0 as usize))
            .copied()
            .unwrap_or(0);
        if line == 0 {
            return Ok(());
        }
        let loc = (depth, func, line);
        if d.last == Some(loc) {
            return Ok(());
        }
        d.last = Some(loc);
        let due = match d.mode {
            StepMode::Run => false,
            StepMode::Into => true,
            StepMode::Over(at) => depth <= at,
            StepMode::Out(at) => depth < at,
        };
        let hit = d.breakpoints.contains(&(func, line));
        if due || hit {
            // A breakpoint wins the label when both apply.
            let reason = if hit {
                PauseReason::Breakpoint
            } else {
                PauseReason::Step
            };
            return self.debug_pause(reason);
        }
        Ok(())
    }

    /// Whether a hook is attached and not suppressed (so `Stop` and failed
    /// assertions pause instead of halting).
    #[inline]
    pub(crate) fn debugger_listening(&self) -> bool {
        self.debugger
            .as_ref()
            .is_some_and(|d| d.hook.is_some() && d.suppress == 0)
    }

    /// Runs the callback and applies its answer.
    pub(crate) fn debug_pause(&mut self, reason: PauseReason) -> VResult<()> {
        let depth = self.frames.len();
        let mut hook = match self.debugger.as_mut().and_then(|d| d.hook.take()) {
            Some(h) => h,
            None => return Ok(()),
        };
        let action = {
            let host = self.host.clone();
            // A person is looking at the program: the wall-clock allowance
            // of the call does not run meanwhile.
            let _stopped = host.budget().suspend_deadline();
            let mut ctx = DebugContext { vm: self, reason };
            hook.paused(&mut ctx)
        };
        if let Some(d) = self.debugger.as_mut() {
            d.hook = Some(hook);
            d.mode = match action {
                ResumeAction::Continue | ResumeAction::Abort => StepMode::Run,
                ResumeAction::StepInto => StepMode::Into,
                ResumeAction::StepOver => StepMode::Over(depth),
                ResumeAction::StepOut => StepMode::Out(depth),
            };
        }
        // A fatal fault raised by code the callback ran (`eval`) ends the
        // run, like in any nested run.
        if let Some(f) = self.pending_fatal.take() {
            return Err(f);
        }
        if action == ResumeAction::Abort {
            return Err(Fault::Cancelled(
                "the run was aborted by the debugger".into(),
            ));
        }
        Ok(())
    }
}

// ---- the view a paused host gets ----------------------------------------------------------

/// A paused program, seen from the pause callback. Everything it offers is
/// read-only except [`eval`](DebugContext::eval) (which runs code under the
/// same capabilities and budgets as the program itself).
pub struct DebugContext<'a> {
    pub(crate) vm: &'a mut Vm,
    reason: PauseReason,
}

impl DebugContext<'_> {
    /// Why the program paused.
    pub fn reason(&self) -> PauseReason {
        self.reason
    }

    /// Steps charged to the budget so far (the pause itself charges none).
    pub fn steps_used(&self) -> u64 {
        self.vm.host.budget().steps_used()
    }

    /// The call stack, innermost frame first (index 0).
    pub fn frames(&self) -> Vec<FrameInfo> {
        let n = self.vm.frames.len();
        (0..n).map(|i| self.vm.frame_info(n - 1 - i)).collect()
    }

    /// Parameters, locals, the function-result slot and `Static` locals of
    /// the frame at `index` (0 = innermost), in declaration order. Compiler
    /// temporaries are not listed. Empty for an unknown frame.
    pub fn locals(&self, index: usize) -> Vec<Variable> {
        match self.vm.frames.len().checked_sub(index + 1) {
            Some(fi) => self.vm.frame_variables(fi),
            None => Vec::new(),
        }
    }

    /// The module-level variables of `module` (a standard module's
    /// variables, or the declared variables of a class-like module with
    /// their current default values omitted: use [`me`](DebugContext::me)
    /// for the instance). Empty for an unknown module.
    pub fn module_variables(&self, module: &str) -> Vec<Variable> {
        let vm = &*self.vm;
        let Some(m) = vm.ir.program.module_by_name(module).map(|m| m.id) else {
            return Vec::new();
        };
        if vm.ir.program.module(m).is_class_like() {
            return Vec::new();
        }
        vm.ir
            .program
            .globals
            .iter()
            .filter(|g| g.module == m && g.static_of.is_none())
            .filter_map(|g| {
                Some(Variable {
                    name: g.name.to_string(),
                    type_name: vm.type_name(&g.ty),
                    value: vm.globals.get(g.id.index())?.clone(),
                    kind: VariableKind::ModuleLevel,
                })
            })
            .collect()
    }

    /// `Me` of the frame at `index`: the object and the variables of its
    /// class (public and private fields). `None` when the frame has no `Me`.
    pub fn me(&self, index: usize) -> Option<(Variant, Vec<Variable>)> {
        let fi = self.vm.frames.len().checked_sub(index + 1)?;
        let id = self.vm.frames[fi].me?;
        let vm = &*self.vm;
        let ObjKind::Class { module, fields, .. } = &vm.objects.get(&id)?.kind else {
            return None;
        };
        let info = vm.ir.classes.get(module.index())?.as_ref()?;
        let vars = info
            .fields
            .iter()
            .zip(fields)
            .map(|(g, v)| {
                let gi = vm.ir.program.global(*g);
                Variable {
                    name: gi.name.to_string(),
                    type_name: vm.type_name(&gi.ty),
                    value: v.clone(),
                    kind: VariableKind::Field,
                }
            })
            .collect();
        Some((
            Variant::Object(ObjectRef::Handle(crate::state::vm_handle(id))),
            vars,
        ))
    }

    /// A short description of an object value for display: its class name
    /// (`Collection`, the class module's name, the host object's class), or
    /// `Nothing`. `None` for a value that is not an object.
    pub fn describe(&self, value: &Variant) -> Option<String> {
        match value {
            Variant::Object(ObjectRef::Nothing) => Some("Nothing".to_owned()),
            Variant::Object(ObjectRef::Handle(h)) => Some(self.vm.object_type_name(*h)),
            _ => None,
        }
    }

    /// Evaluates `text` in the context of the frame at `index` (0 =
    /// innermost): its parameters and locals are visible and, for a
    /// statement, assignable. See [`Vm::eval`].
    pub fn eval(
        &mut self,
        index: usize,
        text: &str,
    ) -> Result<crate::eval::EvalOutput, crate::eval::EvalError> {
        let Some(fi) = self.vm.frames.len().checked_sub(index + 1) else {
            return Err(crate::eval::EvalError::NoSuchFrame);
        };
        self.vm.eval_in_frame(fi, text)
    }
}

impl Vm {
    pub(crate) fn frame_info(&self, fi: usize) -> FrameInfo {
        let f = &self.frames[fi];
        let func = &self.ir.functions[f.func];
        let (module, procedure, kind) = match func.kind {
            FuncKind::Proc(p) => {
                let pi = self.ir.program.proc(p);
                (
                    self.ir.program.module(pi.module).name.to_string(),
                    pi.name.to_string(),
                    Some(pi.kind),
                )
            }
            FuncKind::ModuleInit(m) => (
                self.ir.program.module(m).name.to_string(),
                "(initialiser)".to_owned(),
                None,
            ),
        };
        let line = if f.cur_stmt == u32::MAX {
            0
        } else {
            self.source_lines
                .get(f.func)
                .and_then(|l| l.get(f.cur_stmt as usize))
                .copied()
                .unwrap_or(0)
        };
        FrameInfo {
            module,
            procedure,
            kind,
            line,
            erl: f.line,
            has_me: f.me.is_some(),
        }
    }

    pub(crate) fn frame_variables(&self, fi: usize) -> Vec<Variable> {
        let f = &self.frames[fi];
        let FuncKind::Proc(p) = self.ir.functions[f.func].kind else {
            return Vec::new();
        };
        let pi = self.ir.program.proc(p);
        let mut out = Vec::new();
        for (i, l) in pi.locals.iter().enumerate() {
            let name = l.name.to_string();
            if name.is_empty() || matches!(l.kind, LocalKind::Temp) {
                continue;
            }
            let Some(slot) = f.slots.get(i) else { continue };
            let value = match slot {
                Slot::Val(v) => v.clone(),
                Slot::Bound(loc) => self.load(loc).unwrap_or(Variant::Empty),
            };
            out.push(Variable {
                name,
                type_name: self.type_name(&l.ty),
                value,
                kind: match l.kind {
                    LocalKind::Param(_) => VariableKind::Parameter,
                    LocalKind::Result => VariableKind::Result,
                    _ => VariableKind::Local,
                },
            });
        }
        for g in &self.ir.program.globals {
            if g.static_of == Some(p)
                && let Some(v) = self.globals.get(g.id.index())
            {
                out.push(Variable {
                    name: g.name.to_string(),
                    type_name: self.type_name(&g.ty),
                    value: v.clone(),
                    kind: VariableKind::Static,
                });
            }
        }
        out
    }

    /// A declared type spelled like VBA source (class, type and enum names
    /// instead of ids).
    pub fn type_name_of(&self, ty: &Type) -> String {
        self.type_name(ty)
    }

    pub(crate) fn type_name(&self, ty: &Type) -> String {
        let p = &self.ir.program;
        match ty {
            Type::FixedString(n) => format!("String * {n}"),
            Type::Class(m) => p.module(*m).name.to_string(),
            Type::Udt(u) => p.udts[u.index()].name.to_string(),
            Type::Enum(e) => p.enums[e.index()].name.to_string(),
            Type::Array { elem, .. } => format!("{}()", self.type_name(elem)),
            Type::Library(l) | Type::LibEnum(l) | Type::LibRecord(l) => l.to_string(),
            other => other.to_string(),
        }
    }

    /// The module of a VM-owned object handle.
    #[allow(dead_code)]
    pub(crate) fn module_of_handle(&self, h: ObjectHandle) -> Option<ModuleId> {
        match self.objects.get(&vm_id(h)?)?.kind {
            ObjKind::Class { module, .. } => Some(module),
            _ => None,
        }
    }
}
