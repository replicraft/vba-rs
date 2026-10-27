//! Services for an editor around the interpreter: a debugger, an
//! immediate window and hot edits of a live [`Instance`].
//!
//! Everything here is opt-in. An instance that never attaches a debugger
//! runs exactly as before (same results, same step counts); see
//! `docs/architecture/embedding-guide.md`, "Editor services".

use vba_runtime::{CallableProcedure, EditReport, ModuleEdit};
use vba_vm::{BreakpointInfo, DebugHook};

use crate::{Error, Instance, Variant};

/// Where an [`Instance::eval`] runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalScope<'a> {
    /// Only the project's public names are visible.
    Global,
    /// The names of this module are visible unqualified, private ones
    /// included (variables, constants, procedures, types of a standard
    /// module; constants and types of a class-like module).
    Module(&'a str),
}

/// What an evaluation produced.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct EvalResult {
    /// The value of a `?expression`; `None` for statements.
    pub value: Option<Variant>,
    /// The value as `Debug.Print` writes it, without the line end; empty
    /// for statements.
    pub text: String,
    /// What `Debug.Print` wrote while the text ran.
    pub output: String,
}

impl EvalResult {
    /// The text an immediate window shows for this result: the value of an
    /// expression, or what the statements printed.
    pub fn display(&self) -> String {
        if self.value.is_some() {
            format!("{}{}", self.output, self.text)
        } else {
            self.output.trim_end_matches('\n').to_owned()
        }
    }
}

impl Error {
    /// A message for an immediate window or a log line: the VBA error as
    /// `Run-time error '13': Type mismatch`, compile diagnostics one per
    /// line, any other failure as its message. It carries no procedure or
    /// line, which mean nothing for text typed on one line.
    pub fn immediate_message(&self) -> String {
        match self {
            Error::Run(vba_runtime::Error::Load(l)) => l.diagnostics.join("\n"),
            other => match other.report() {
                Some(r) => format!("Run-time error '{}': {}", r.number, r.description),
                None => other.to_string(),
            },
        }
    }
}

impl Instance {
    /// Evaluates one line of VBA in the context of `scope`, like an
    /// immediate window.
    ///
    /// * `? expression` returns the value (`result.value`, formatted in
    ///   `result.text`);
    /// * anything else is a list of statements separated by `:`
    ///   (`counter = counter + 1`, `Call Reset`, `Debug.Print Total()`).
    ///
    /// The text runs on the live state of the instance under the same trust
    /// requirement, capabilities, step budget and call timeout as any other
    /// call (it is an ordinary `Instance` call: [`NotTrusted`] without
    /// [`trust`](Instance::trust)), so it cannot do anything the program
    /// could not. Code started by it does not stop at breakpoints. A VBA
    /// error in the text is an `Err`; the instance stays usable and `Err`
    /// is as it was. Only one line is accepted. To evaluate in the frame
    /// of a paused program, with its locals visible, use
    /// `DebugContext::eval` from the pause callback.
    ///
    /// ```
    /// use vba::{Engine, EvalScope, Project, Variant};
    ///
    /// let project = Project::from_source(
    ///     "Module1",
    ///     "Private hits As Long\nPublic Sub Hit()\n  hits = hits + 1\nEnd Sub\n",
    /// );
    /// let mut inst = Engine::new().load(&project).unwrap();
    /// inst.trust();
    /// inst.eval("Call Hit: Call Hit", EvalScope::Module("Module1")).unwrap();
    /// let r = inst.eval("? hits * 10", EvalScope::Module("Module1")).unwrap();
    /// assert_eq!(r.value, Some(Variant::Long(20)));
    /// // `hits` is private: outside the module it is not defined.
    /// assert!(inst.eval("? hits", EvalScope::Global).is_err());
    /// ```
    ///
    /// [`NotTrusted`]: vba_runtime::Error::NotTrusted
    pub fn eval(&mut self, text: &str, scope: EvalScope<'_>) -> Result<EvalResult, Error> {
        let module = match scope {
            EvalScope::Global => None,
            EvalScope::Module(m) => Some(m),
        };
        let before = self.sink.contents().len();
        let r = self.guarded(|s| s.eval(module, text));
        let output = self.sink.contents()[before..].to_owned();
        let out = r?;
        Ok(EvalResult {
            value: out.value,
            text: out.text,
            output,
        })
    }

    /// Replaces, adds and removes modules of the live instance, compiling
    /// again, without losing state that does not need to be lost.
    ///
    /// Everything is checked before anything changes: if the new sources do
    /// not compile ([`Error::Run`] with a `Load` error that carries the
    /// diagnostics), if a limit is exceeded, or if a procedure is running
    /// ([`Busy`]), the instance is exactly as it was.
    ///
    /// # State policy
    ///
    /// * An edit that changes nothing (same text) is detected: no
    ///   recompilation, no change ([`EditReport::unchanged`]).
    /// * If the declarations of the new program match the old one (the same
    ///   modules, procedures with their signatures, variables with their
    ///   types, types, enums, events and `Declare`s, in the same order;
    ///   typically, only procedure bodies, comments or constant values
    ///   changed), **all state is kept**: module variables, class instances
    ///   (including those from [`create_object`](Instance::create_object)),
    ///   default instances, `Err`, open files.
    /// * Otherwise the state is rebuilt: a module-level variable of a
    ///   standard module (and a `Static` local) keeps its value when the new
    ///   program has a variable with the same module, name and declared
    ///   type and the value is plain data (no VBA object, no user-defined
    ///   type value inside); every other variable returns to its initial
    ///   value, standard-module initialisers run again, and every object
    ///   owned by the interpreter (class instances, default instances,
    ///   collections held in variables) is dropped **without**
    ///   `Class_Terminate`, exactly as [`reset`](Instance::reset) does;
    ///   references to host objects they held are released and handles of
    ///   dropped instances become invalid. The report lists what was kept and
    ///   reset ([`EditReport::state`]).
    /// * The call is refused while any procedure is running: the interpreter
    ///   is idle exactly when no call into it is active. (From inside a pause
    ///   callback there is no way to call it; stop the run, edit, start
    ///   again. Edit-and-continue is not offered.)
    /// * Trust, budget counters and the output buffer are untouched. The
    ///   edited code runs with the trust given to the old code; only apply
    ///   edits you would trust.
    /// * The whole program is compiled again (name resolution crosses
    ///   modules), so the cost is that of [`Engine::load`](crate::Engine::load);
    ///   the host's source and node limits apply.
    ///
    /// ```
    /// use vba::{Engine, ModuleEdit, Project, Variant};
    ///
    /// let project = Project::from_source(
    ///     "Module1",
    ///     "Public Total As Long\nPublic Function Twice(ByVal n As Long) As Long\n  Twice = n * 2\nEnd Function\n",
    /// );
    /// let mut inst = Engine::new().load(&project).unwrap();
    /// inst.trust();
    /// inst.set_global("Total", Variant::Long(7)).unwrap();
    ///
    /// // A syntax error changes nothing.
    /// let bad = ModuleEdit::Replace { name: "Module1".into(), source: "Sub (".into() };
    /// assert!(inst.apply_edits(&[bad]).is_err());
    /// assert_eq!(inst.run("Twice", vec![4.into()]).unwrap().value, Variant::Long(8));
    ///
    /// // A body edit keeps the state.
    /// let good = ModuleEdit::Replace {
    ///     name: "Module1".into(),
    ///     source: "Public Total As Long\nPublic Function Twice(ByVal n As Long) As Long\n  Twice = n * 3\nEnd Function\n".into(),
    /// };
    /// let report = inst.apply_edits(&[good]).unwrap();
    /// assert!(!report.state.unwrap().layout_changed);
    /// assert_eq!(inst.get_global("Total").unwrap(), Variant::Long(7));
    /// assert_eq!(inst.run("Twice", vec![4.into()]).unwrap().value, Variant::Long(12));
    /// ```
    ///
    /// [`Busy`]: vba_runtime::Error::Busy
    pub fn apply_edits(&mut self, edits: &[ModuleEdit]) -> Result<EditReport, Error> {
        self.guard()?;
        let r = self.session.apply_edits(edits);
        self.conv(r)
    }

    /// The procedures a "Run Macro" list can offer: public `Sub`s and
    /// `Function`s of standard modules, and of class-like modules that have a
    /// default instance. Pass [`CallableProcedure::qualified_name`] to
    /// [`run`](Instance::run) or [`call`](Instance::call).
    ///
    /// Names resolve like `Application.Run`, case-insensitively:
    /// `Module.Proc`, or a bare `Proc` of a standard module when no other
    /// standard module declares one of that name (otherwise
    /// [`AmbiguousProcedure`](vba_runtime::Error::AmbiguousProcedure)). A
    /// procedure of a plain class module is not callable by name; create an
    /// instance and use [`CallTarget::Object`](crate::CallTarget::Object).
    /// The project name may qualify the name as well
    /// (`Project.Module.Proc`, case-insensitive); it is ignored.
    pub fn callable_procedures(&self) -> Vec<CallableProcedure> {
        self.session.callable_procedures()
    }

    // ---- debugger -----------------------------------------------------------------

    /// Attaches a debugger: from now on the interpreter calls
    /// `hook.paused(&mut DebugContext)` at breakpoints, when a step
    /// request completes, at a `Stop` statement and at a failed
    /// `Debug.Assert`, on the thread that runs the program, and acts on the
    /// [`ResumeAction`](crate::ResumeAction) it returns. A closure
    /// `FnMut(&mut DebugContext) -> ResumeAction` works.
    ///
    /// Without a debugger, `Stop` and a failed `Debug.Assert` halt the run
    /// with [`RunStatus::Stopped`](crate::RunStatus::Stopped) (the reason
    /// is available from `session().vm().stop_reason()`). With one they
    /// pause, and the program goes on if the hook says `Continue`.
    ///
    /// The debugger never changes what the program does or how many steps
    /// it uses; pausing does not charge steps, and the wall-clock allowance
    /// of the call (see [`Engine::call_timeout`](crate::Engine::call_timeout))
    /// does not run while the hook does. A hook that never returns blocks
    /// the thread: the cancel token cannot interrupt it, so a hook that
    /// waits for a person should give up (return
    /// [`Abort`](crate::ResumeAction::Abort)) when it is told to.
    ///
    /// ```
    /// use vba::{Engine, Project, ResumeAction};
    ///
    /// let project = Project::from_source(
    ///     "Module1",
    ///     "Public Function Sum(ByVal n As Long) As Long\n  Dim i As Long, total As Long\n  For i = 1 To n\n    total = total + i\n  Next i\n  Sum = total\nEnd Function\n",
    /// );
    /// let mut inst = Engine::new().load(&project).unwrap();
    /// inst.trust();
    /// let lines = inst.set_breakpoints(&[("Module1", 4)]);
    /// assert_eq!(lines[0].line, Some(4));
    /// let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    /// let log = seen.clone();
    /// inst.attach_debugger(move |ctx: &mut vba::DebugContext<'_>| {
    ///     let locals = ctx.locals(0);
    ///     let i = locals.iter().find(|v| v.name == "i").unwrap();
    ///     log.borrow_mut().push(i.value.clone());
    ///     ResumeAction::Continue
    /// });
    /// let out = inst.run("Sum", vec![3.into()]).unwrap();
    /// assert_eq!(out.value, vba::Variant::Long(6));
    /// assert_eq!(seen.borrow().len(), 3);
    /// ```
    pub fn attach_debugger(&mut self, hook: impl DebugHook + 'static) {
        self.session.set_debug_hook(Some(Box::new(hook)));
    }

    /// Detaches the debugger; the breakpoints stay installed, silent.
    pub fn detach_debugger(&mut self) {
        self.session.set_debug_hook(None);
        self.step_armed = false;
    }

    /// Whether a debugger is attached.
    pub fn debugger_attached(&self) -> bool {
        self.session.debugger_attached()
    }

    /// Replaces the breakpoints. A breakpoint is a `(module, line)` pair
    /// (module name case-insensitive, 1-based line of the module's source).
    /// The result says, per request, which line it sits on: a line without a
    /// statement moves to the next line with one in the same procedure, or
    /// is not installed (`line: None`). The breakpoints follow the source
    /// through [`apply_edits`](Instance::apply_edits) by `(module, line)`
    /// (they are resolved again against the new program).
    pub fn set_breakpoints(&mut self, wanted: &[(&str, u32)]) -> Vec<BreakpointInfo> {
        let wanted: Vec<(String, u32)> =
            wanted.iter().map(|(m, l)| ((*m).to_owned(), *l)).collect();
        self.session.set_breakpoints(&wanted)
    }

    /// The lines of a module that can hold a breakpoint, ascending.
    pub fn breakable_lines(&self, module: &str) -> Vec<u32> {
        self.session.breakable_lines(module)
    }

    /// Makes the debugger pause at the first statement of the next call
    /// into the instance ("step into" from idle). Needs a debugger.
    pub fn break_on_next_statement(&mut self) {
        self.step_armed = true;
    }

    /// Applied at the start of every call into the session: a run starts
    /// with `Continue` semantics unless a step was requested.
    pub(crate) fn prepare_debugger(&mut self) {
        let vm = self.session.vm();
        if !vm.debug_hook_attached() {
            return;
        }
        if self.step_armed {
            self.step_armed = false;
            vm.request_step();
        } else {
            vm.clear_stepping();
        }
    }
}
