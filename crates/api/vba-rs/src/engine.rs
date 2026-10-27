//! [`Engine`]: the builder-style path from a project to a run.

use std::rc::Rc;
use std::time::Duration;

use vba_automation::{Dispatch, ObjectTable};
use vba_host::{
    Capability, HostMetadata, HostResult, Policy, ResourceLimits, UNTRUSTED_CALL_TIMEOUT, UiHooks,
};
use vba_host_model::HostRuntime;
use vba_profile::{Profile, default_profile};
use vba_runtime::{MemFs, ModuleScope, RealFs, Session, SessionBuilder, SystemHost};
use vba_value::Date;
use vba_value::{ObjectRef, Variant};
use vba_vm::{BufferSink, DebugSink, RunStatus, VmConfig};

use crate::Error;
use crate::call::{Arg, CallLimits, CallResult, CallTarget, split_args};
use crate::host::{CancelToken, EngineHost, Factory};
use crate::project::{Project, Report};

/// Tells the analysis which global names the engine registered, so that
/// `Application.Name` in VBA code is a late-bound member access on a global
/// object and not an undeclared variable.
struct EngineGlobals {
    objects: Vec<String>,
    constants: Vec<(String, Variant)>,
}

/// The static type VBA gives a constant of this value.
fn constant_type(v: &Variant) -> vba_sema::Type {
    use vba_sema::Type;
    match v {
        Variant::Boolean(_) => Type::Boolean,
        Variant::Byte(_) => Type::Byte,
        Variant::Integer(_) => Type::Integer,
        Variant::Long(_) => Type::Long,
        Variant::LongLong(_) => Type::LongLong,
        Variant::Single(_) => Type::Single,
        Variant::Double(_) => Type::Double,
        Variant::Currency(_) => Type::Currency,
        Variant::Date(_) => Type::Date,
        Variant::String(_) => Type::String,
        _ => Type::Variant,
    }
}

impl vba_sema::TypeProvider for EngineGlobals {
    fn resolve_type(&self, _: Option<&str>, _: &str) -> Option<vba_sema::LibraryType> {
        None
    }

    fn global(&self, name: &str) -> Option<vba_sema::LibGlobal> {
        if let Some((_, v)) = self
            .constants
            .iter()
            .find(|(g, _)| g.eq_ignore_ascii_case(name))
        {
            return Some(vba_sema::LibGlobal::Constant(v.clone(), constant_type(v)));
        }
        self.objects
            .iter()
            .any(|g| g.eq_ignore_ascii_case(name))
            .then_some(vba_sema::LibGlobal::Object(vba_sema::Type::Object))
    }
}

/// Writes to the instance's buffer and, when set, to the embedder's sink.
struct Tee {
    buffer: BufferSink,
    other: Option<Box<dyn DebugSink>>,
}

impl DebugSink for Tee {
    fn write(&mut self, text: &str) {
        self.buffer.write(text);
        if let Some(o) = self.other.as_mut() {
            o.write(text);
        }
    }
}

#[derive(Clone)]
enum Files {
    Memory(Vec<(String, Vec<u8>)>),
    Real,
}

/// Configuration shared by the sessions it creates. Nothing is ambient: the
/// default engine has a deny-all [`Policy`], no host objects, an in-memory
/// file system, a fixed clock (2000-01-01) and an empty environment.
///
/// ```
/// use vba::{Engine, Project};
///
/// let project = Project::from_source(
///     "Module1",
///     "Public Function Twice(ByVal n As Long) As Long\n  Twice = n * 2\nEnd Function\n",
/// );
/// let outcome = Engine::new().run(&project, "Twice", vec![21.into()]).unwrap();
/// assert_eq!(outcome.value, vba::Variant::Long(42));
/// ```
#[derive(Clone)]
pub struct Engine {
    profile: Profile,
    policy: Policy,
    limits: ResourceLimits,
    call_timeout: Option<Duration>,
    vm: VmConfig,
    cancel: CancelToken,
    globals: Vec<(String, Factory)>,
    constants: Vec<(String, Variant)>,
    classes: Vec<(String, Factory)>,
    libraries: Vec<HostRuntime>,
    files: Files,
    system_environment: bool,
    command: String,
    host_members: Vec<String>,
    scopes: Vec<ModuleScope>,
    strict_spec: bool,
    fixed_clock: Option<Date>,
    ui: Option<Rc<dyn UiHooks>>,
    debug_sinks: Option<Rc<dyn Fn() -> Box<dyn DebugSink>>>,
}

impl std::fmt::Debug for Engine {
    /// Shows the configuration; the registered factories are closures and
    /// appear by name only.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("profile", &self.profile)
            .field("policy", &self.policy)
            .field("limits", &self.limits)
            .field("call_timeout", &self.call_timeout)
            .field("vm", &self.vm)
            .field("cancelled", &self.cancel.is_cancelled())
            .field(
                "globals",
                &self.globals.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            )
            .field(
                "classes",
                &self.classes.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            )
            .field("real_files", &matches!(self.files, Files::Real))
            .field("system_environment", &self.system_environment)
            .field("strict_spec", &self.strict_spec)
            .finish_non_exhaustive()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

impl Engine {
    /// An engine with the default profile and no authority at all.
    #[must_use]
    pub fn new() -> Engine {
        Engine {
            profile: *default_profile(),
            policy: Policy::deny_all(),
            limits: ResourceLimits::untrusted(),
            call_timeout: Some(UNTRUSTED_CALL_TIMEOUT),
            vm: VmConfig::default(),
            cancel: CancelToken::new(),
            globals: Vec::new(),
            constants: Vec::new(),
            classes: Vec::new(),
            libraries: Vec::new(),
            files: Files::Memory(Vec::new()),
            system_environment: false,
            command: String::new(),
            host_members: Vec::new(),
            scopes: Vec::new(),
            strict_spec: false,
            fixed_clock: None,
            ui: None,
            debug_sinks: None,
        }
    }

    /// The VBA implementation the program is written for (`#If`, `VBA7`,
    /// `Win64`...). It never reads the machine vba-rs runs on.
    #[must_use]
    pub fn profile(mut self, profile: Profile) -> Engine {
        self.profile = profile;
        self
    }

    /// Replaces the whole permission policy.
    #[must_use]
    pub fn policy(mut self, policy: Policy) -> Engine {
        self.policy = policy;
        self
    }

    /// Grants one capability for every resource.
    #[must_use]
    pub fn grant(mut self, cap: Capability) -> Engine {
        self.policy = self.policy.grant(cap);
        self
    }

    /// Grants a capability for resources starting with one of the prefixes
    /// (normalized, forward-slash paths for files).
    #[must_use]
    pub fn grant_prefixes<I, S>(mut self, cap: Capability, prefixes: I) -> Engine
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.policy = self.policy.grant_prefixes(cap, prefixes);
        self
    }

    /// Sets the resource limits (steps, memory, call depth). A new engine
    /// starts from [`ResourceLimits::untrusted`]: 100 million steps, 256 MiB
    /// and 1000 nested calls, plus a wall-clock allowance per call (see
    /// [`Engine::call_timeout`]).
    #[must_use]
    pub fn limits(mut self, limits: ResourceLimits) -> Engine {
        self.limits = limits;
        self
    }

    /// Shorthand for a step budget.
    #[must_use]
    pub fn max_steps(mut self, steps: u64) -> Engine {
        self.limits.max_steps = Some(steps);
        self
    }

    /// Shorthand for a memory budget in bytes (strings, arrays, collections,
    /// dictionaries, in-memory files and output all count).
    #[must_use]
    pub fn max_memory(mut self, bytes: u64) -> Engine {
        self.limits.max_memory_bytes = Some(bytes);
        self
    }

    /// Removes the step, memory and wall-clock limits (the call depth stays
    /// bounded). Only for code you trust; the cancellation token still works.
    #[must_use]
    pub fn unlimited(mut self) -> Engine {
        self.limits = ResourceLimits::unlimited();
        self.call_timeout = None;
        self
    }

    /// The wall-clock allowance of every call from the embedder (`run`,
    /// `call`, `call_on`, `raise_event`...): one minute by default, checked
    /// cooperatively every 1024 steps and inside long library calls. When
    /// it passes, the call ends with `Error::Run(Cancelled)`; the
    /// [`CancelToken`] is not touched. A [`CallLimits::timeout`] set by
    /// [`Instance::with_call_limits`] replaces it for that scope. `None`
    /// removes the allowance; [`Engine::unlimited`] does too.
    #[must_use]
    pub fn call_timeout(mut self, timeout: Option<Duration>) -> Engine {
        self.call_timeout = timeout;
        self
    }

    /// The wall-clock allowance per call this engine gives its instances.
    #[must_use]
    pub fn configured_call_timeout(&self) -> Option<Duration> {
        self.call_timeout
    }

    /// Replaces the VM settings (string length, array size, nesting).
    #[must_use]
    pub fn vm_config(mut self, config: VmConfig) -> Engine {
        self.vm = config;
        self
    }

    /// A token that stops sessions of this engine at their next `DoEvents`.
    #[must_use]
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    /// Replaces the token (to share one across engines).
    #[must_use]
    pub fn with_cancel_token(mut self, token: CancelToken) -> Engine {
        self.cancel = token;
        self
    }

    /// Registers a host class for `CreateObject(progid)` / `GetObject(, progid)`.
    /// The factory runs once per call and returns a value that carries one
    /// reference (see [`Engine::dispatch_class`] for the usual case).
    /// `CreateObject` additionally needs [`Capability::ComCreate`].
    #[must_use]
    pub fn class(
        mut self,
        progid: &str,
        factory: impl Fn(&ObjectTable) -> HostResult<Variant> + 'static,
    ) -> Engine {
        self.classes.push((progid.to_lowercase(), Rc::new(factory)));
        self
    }

    /// Registers a host class implemented as a [`Dispatch`] object: each
    /// `CreateObject(progid)` gets a fresh object from `make`.
    #[must_use]
    pub fn dispatch_class<T: Dispatch + 'static>(
        self,
        progid: &str,
        make: impl Fn() -> T + 'static,
    ) -> Engine {
        self.class(progid, move |table| {
            Ok(Variant::Object(ObjectRef::Handle(table.insert_obj(make()))))
        })
    }

    /// Registers a global name (like `Application`) that resolves to a
    /// value built on first use and shared afterwards, per session.
    #[must_use]
    pub fn global(
        mut self,
        name: &str,
        factory: impl Fn(&ObjectTable) -> HostResult<Variant> + 'static,
    ) -> Engine {
        self.globals.push((name.to_lowercase(), Rc::new(factory)));
        self
    }

    /// Registers a global name bound to a [`Dispatch`] object.
    #[must_use]
    pub fn dispatch_global<T: Dispatch + 'static>(
        self,
        name: &str,
        make: impl Fn() -> T + 'static,
    ) -> Engine {
        self.global(name, move |table| {
            Ok(Variant::Object(ObjectRef::Handle(table.insert_obj(make()))))
        })
    }

    /// Registers a named constant of the embedding application
    /// (`acViewNormal`, `dbOpenDynaset`, `vbYes`-like members of its
    /// libraries). It is a real constant for the analysis: it can be used in
    /// `Const` declarations, `Case` labels and array bounds, and it folds at
    /// compile time. The static type follows the value (`Long` for an enum
    /// member). A constant shadows a [`global`](Engine::global) or
    /// intrinsic of the same name.
    #[must_use]
    pub fn constant(mut self, name: &str, value: impl Into<Variant>) -> Engine {
        self.constants.push((name.to_owned(), value.into()));
        self
    }

    /// Registers several constants at once, for example an enum.
    #[must_use]
    pub fn constants<I, S, V>(mut self, items: I) -> Engine
    where
        I: IntoIterator<Item = (S, V)>,
        S: Into<String>,
        V: Into<Variant>,
    {
        self.constants
            .extend(items.into_iter().map(|(n, v)| (n.into(), v.into())));
        self
    }

    /// Registers a described host library: its predeclared objects, creatable
    /// classes (as `CreateObject("Library.Class")` and `New`) and global
    /// functions run, and the same description types the program for the
    /// analysis (`Dim rs As Recordset`, `Nz(...)`, `Application.Name`), so
    /// [`analyze`](Engine::analyze), [`load`](Engine::load) and a language
    /// server fed the same library all agree. See [`host_model`](crate::host_model).
    ///
    /// `CreateObject` still needs [`Capability::ComCreate`]; the library
    /// grants nothing by itself.
    #[must_use]
    pub fn host_library(mut self, library: HostRuntime) -> Engine {
        let lib_name = library.library().name().to_owned();
        for (name, make) in library.globals() {
            let source = lib_name.clone();
            self.globals.push((
                name.to_lowercase(),
                Rc::new(move |table| make(table).map_err(|e| e.into_err_info(&source))),
            ));
        }
        for (class, make) in library.classes() {
            let source = lib_name.clone();
            self.classes.push((
                format!("{lib_name}.{class}").to_lowercase(),
                Rc::new(move |table| make(table).map_err(|e| e.into_err_info(&source))),
            ));
        }
        self.libraries.push(library);
        self
    }

    /// Seeds the in-memory file system with a file (switches back to the
    /// in-memory file system if [`Engine::real_files`] was chosen).
    #[must_use]
    pub fn with_file(mut self, path: &str, contents: &[u8]) -> Engine {
        match &mut self.files {
            Files::Memory(v) => v.push((path.to_owned(), contents.to_vec())),
            Files::Real => self.files = Files::Memory(vec![(path.to_owned(), contents.to_vec())]),
        }
        self
    }

    /// Uses the real file system for the file statements. Still gated by
    /// `FileRead` / `FileWrite` grants on the symlink-resolved path.
    #[must_use]
    pub fn real_files(mut self) -> Engine {
        self.files = Files::Real;
        self
    }

    /// Reads the real clock and environment variables instead of the fixed
    /// ones. This makes runs non-deterministic; it needs no capability
    /// because it only reads.
    #[must_use]
    pub fn system_environment(mut self) -> Engine {
        self.system_environment = true;
        self
    }

    /// Fixes the clock (`Now`, `Date`, `Time`, `Timer`, and the year given to
    /// date literals that name none, such as `#3/4#`) at `now` instead of
    /// the default 2000-01-01 00:00:00. Ignored after
    /// [`system_environment`](Engine::system_environment).
    #[must_use]
    pub fn fixed_clock(mut self, now: Date) -> Engine {
        self.fixed_clock = Some(now);
        self
    }

    /// Names the embedding application provides as members of document and
    /// designer modules (the controls and base-class properties of a form).
    /// Inside such a module an unqualified use of one of them (`Dirty`,
    /// `CustomerName = "x"`) is bound late to the host base object of the
    /// instance, even under `Option Explicit`; other undeclared names keep
    /// the normal rules, so typos are still reported. `Me.Name`, `Me!Name`,
    /// `Me("Name")` and calls `Name(args)` need no listing.
    #[must_use]
    pub fn host_members<I, S>(mut self, names: I) -> Engine
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.host_members = names.into_iter().map(Into::into).collect();
        self
    }

    /// Declares the described class that is the `Me` of one document,
    /// designer or class module: its controls, properties, methods and
    /// events. In that module bare names, `Me.X`, `Me!X` and the handlers
    /// (`Ctl_Click`, `Form_Load`) resolve against that class only, early
    /// bound and checked; a control the form does not have is a diagnostic.
    /// Modules without a scope keep the engine-wide
    /// [`host_members`](Engine::host_members) list. Replaces the scope the
    /// module already has. See
    /// [`set_module_scope`](Engine::set_module_scope) for design changes.
    #[must_use]
    pub fn module_scope(mut self, scope: ModuleScope) -> Engine {
        self.set_module_scope(scope);
        self
    }

    /// [`module_scope`](Engine::module_scope) on an engine you keep: the way
    /// to follow a form whose design changed (a control was added or
    /// removed). Replace the scope with a description of the new design,
    /// then [`analyze`](Engine::analyze) and [`load`](Engine::load) again;
    /// nothing else of the engine is rebuilt. A running
    /// [`Instance`] follows with
    /// [`Instance::set_module_scope`].
    pub fn set_module_scope(&mut self, scope: ModuleScope) {
        match self
            .scopes
            .iter_mut()
            .find(|s| s.host.module.eq_ignore_ascii_case(&scope.host.module))
        {
            Some(old) => *old = scope,
            None => self.scopes.push(scope),
        }
    }

    /// Removes the scope of a module, so that the engine-wide
    /// [`host_members`](Engine::host_members) apply to it again. Returns
    /// whether the module had one.
    pub fn remove_module_scope(&mut self, module: &str) -> bool {
        let before = self.scopes.len();
        self.scopes
            .retain(|s| !s.host.module.eq_ignore_ascii_case(module));
        self.scopes.len() != before
    }

    /// The modules that have a described host class.
    #[must_use]
    pub fn module_scopes(&self) -> &[ModuleScope] {
        &self.scopes
    }

    /// Installs the `MsgBox` / `InputBox` implementation of the embedding
    /// application. Without it both fail with error 70, as in a headless
    /// host; the engine never invents an answer.
    #[must_use]
    pub fn ui(mut self, ui: impl UiHooks + 'static) -> Engine {
        self.ui = Some(Rc::new(ui));
        self
    }

    /// Treats the spec-strict diagnostics as errors. These are static rules
    /// taken from the MS-VBAL text (overlapping `Def<Type>` ranges, empty
    /// `Type`/`Enum`, `ByVal` array parameters...) that real VBA is not
    /// known to enforce; by default they are warnings and the project loads
    /// and runs. See `docs/language/spec-coverage.md` for the list.
    #[must_use]
    pub fn strict_spec(mut self, on: bool) -> Engine {
        self.strict_spec = on;
        self
    }

    /// The text `Command$` returns.
    #[must_use]
    pub fn command_line(mut self, command: &str) -> Engine {
        self.command = command.to_owned();
        self
    }

    /// Analyses the project with this engine's profile; nothing runs.
    #[must_use]
    pub fn analyze(&self, project: &Project) -> Report {
        let globals = self.provider();
        let mut layers: Vec<&dyn vba_sema::TypeProvider> = self
            .scopes
            .iter()
            .map(|s| &*s.provider)
            .chain(
                self.libraries
                    .iter()
                    .map(|l| l.library() as &dyn vba_sema::TypeProvider),
            )
            .collect();
        layers.push(&globals);
        project.analyze_with(
            &self.profile,
            &vba_sema::Layers(layers),
            &self.host_members,
            &self
                .scopes
                .iter()
                .map(|s| s.host.clone())
                .collect::<Vec<_>>(),
            self.strict_spec,
        )
    }

    fn provider(&self) -> EngineGlobals {
        EngineGlobals {
            objects: self.globals.iter().map(|(n, _)| n.clone()).collect(),
            constants: self.constants.clone(),
        }
    }

    /// Also sends everything `Debug.Print` writes to a sink of your own, one
    /// per instance (`make` is called when an instance is built). The
    /// in-memory buffer behind [`Instance::output`] and [`Outcome::output`]
    /// keeps receiving the text too, so existing code is unaffected. The
    /// sink sees raw text as the program writes it: a `Debug.Print` statement
    /// ends with `\n` unless its list ends with `;` or `,`.
    ///
    /// ```
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use vba::{DebugSink, Engine, Project};
    ///
    /// struct Lines(Rc<RefCell<Vec<String>>>);
    /// impl DebugSink for Lines {
    ///     fn write(&mut self, text: &str) {
    ///         self.0.borrow_mut().push(text.to_owned());
    ///     }
    /// }
    ///
    /// let seen = Rc::new(RefCell::new(Vec::new()));
    /// let sink = seen.clone();
    /// let engine = Engine::new().debug_sink(move || Box::new(Lines(sink.clone())));
    /// let project = Project::from_source("M", "Sub Main()\n  Debug.Print 1\nEnd Sub\n");
    /// engine.run(&project, "Main", vec![]).unwrap();
    /// assert_eq!(seen.borrow().concat(), " 1 \n");
    /// ```
    #[must_use]
    pub fn debug_sink(mut self, make: impl Fn() -> Box<dyn DebugSink> + 'static) -> Engine {
        self.debug_sinks = Some(Rc::new(make));
        self
    }

    /// A session builder configured like this engine, plus the sink that
    /// collects `Debug.Print` text. Use it to add what the engine does not
    /// expose (a `Shell` hook, a custom `TypeProvider`...) and build the
    /// session yourself.
    #[must_use]
    pub fn session_builder(&self) -> (SessionBuilder, BufferSink) {
        let sink = BufferSink::new();
        let mut host = EngineHost::new(
            HostMetadata::new("vba-rs", env!("CARGO_PKG_VERSION"), self.profile),
            self.policy.clone(),
            self.limits,
            self.cancel.clone(),
        );
        host.ui = self.ui.clone();
        for (n, f) in &self.globals {
            host.globals.insert(n.clone(), f.clone());
        }
        for (n, f) in &self.classes {
            host.classes.insert(n.clone(), f.clone());
        }
        for l in &self.libraries {
            host.functions
                .push((l.library().name().to_owned(), l.functions().clone()));
        }
        let mut b = Session::builder()
            .provider(self.provider())
            .host(Rc::new(host))
            .debug_sink(Tee {
                buffer: sink.clone(),
                other: self.debug_sinks.as_ref().map(|make| make()),
            })
            .command_line(&self.command)
            .host_members(self.host_members.clone())
            .strict_spec(self.strict_spec)
            .vm_config(self.vm.clone());
        for l in &self.libraries {
            b = b.extra_provider(l.library().clone());
        }
        for s in &self.scopes {
            b = b.module_scope(s.clone());
        }
        b = match &self.files {
            Files::Memory(files) => {
                let mut fs = MemFs::new();
                for (p, d) in files {
                    fs = fs.with_file(p, d);
                }
                b.vfs(fs)
            }
            Files::Real => b.vfs(RealFs::new()),
        };
        if let Some(now) = self.fixed_clock {
            b = b.std_host(vba_stdlib::FixedHost::new(now));
        }
        if self.system_environment {
            b = b.std_host(SystemHost::new());
        }
        (b, sink)
    }

    /// Loads the project into a new, **untrusted** [`Instance`]: it is parsed,
    /// analysed and compiled, and no user code has run.
    pub fn load(&self, project: &Project) -> Result<Instance, Error> {
        let (builder, sink) = self.session_builder();
        let session = builder.build_project(project.model()).map_err(Error::Run)?;
        Ok(Instance::new(
            session,
            sink,
            self.cancel.clone(),
            self.call_timeout,
        ))
    }

    /// Like [`load`](Engine::load), after `configure` adjusted the session
    /// builder: the way to add what the engine does not expose, such as a
    /// `TypeProvider` that describes host types for early binding and
    /// `WithEvents` (add it with `SessionBuilder::extra_provider`, which
    /// keeps the engine's own globals visible), a `Shell` hook or a
    /// different file system.
    pub fn load_with(
        &self,
        project: &Project,
        configure: impl FnOnce(SessionBuilder) -> SessionBuilder,
    ) -> Result<Instance, Error> {
        let (builder, sink) = self.session_builder();
        let session = configure(builder)
            .build_project(project.model())
            .map_err(Error::Run)?;
        Ok(Instance::new(
            session,
            sink,
            self.cancel.clone(),
            self.call_timeout,
        ))
    }

    /// Loads, trusts, runs `procedure` and shuts the session down. Calling
    /// this is the embedder's explicit decision to run the project's code.
    pub fn run(
        &self,
        project: &Project,
        procedure: &str,
        args: Vec<Variant>,
    ) -> Result<Outcome, Error> {
        let mut inst = self.load(project)?;
        inst.trust();
        let result = inst.run(procedure, args);
        let shutdown = inst.shutdown();
        let out = result?;
        shutdown?;
        Ok(out)
    }
}

/// What one run produced.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Outcome {
    /// The function result (`Empty` for a `Sub`).
    pub value: Variant,
    /// How the procedure finished.
    pub status: RunStatus,
    /// The `Debug.Print` text written during this call.
    pub output: String,
}

/// A loaded project: one [`Session`] and its `Debug.Print` buffer.
/// Single-threaded: an instance is not `Send`, so it cannot leave the thread
/// that created it.
///
/// ```compile_fail
/// fn assert_send<T: Send>() {}
/// assert_send::<vba::Instance>();
/// ```
pub struct Instance {
    pub(crate) session: Session,
    pub(crate) sink: BufferSink,
    cancel: CancelToken,
    call_timeout: Option<Duration>,
    poisoned: Option<String>,
    pub(crate) step_armed: bool,
}

impl std::fmt::Debug for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instance")
            .field("trusted", &self.is_trusted())
            .field("usable", &self.is_usable())
            .field("poisoned", &self.poisoned)
            .field("call_timeout", &self.call_timeout)
            .finish_non_exhaustive()
    }
}

impl Instance {
    pub(crate) fn new(
        session: Session,
        sink: BufferSink,
        cancel: CancelToken,
        call_timeout: Option<Duration>,
    ) -> Instance {
        Instance {
            session,
            sink,
            cancel,
            call_timeout,
            poisoned: None,
            step_armed: false,
        }
    }

    /// Wraps a session you built yourself (see
    /// [`Engine::session_builder`]). `cancel` is the token the session's
    /// host polls (the engine's own, or a fresh one if your host has none);
    /// it only feeds [`is_usable`](Instance::is_usable).
    ///
    /// Every call from the embedder gets the same wall-clock allowance that
    /// [`Engine::new`] gives, [`UNTRUSTED_CALL_TIMEOUT`] (60 seconds). The
    /// session brings its own host, and a host built without limits has
    /// neither a step budget nor a deadline; an instance that quietly
    /// inherited that would let a hostile loop run forever, so the safe
    /// value is the default and the unlimited one is an explicit choice.
    /// Change it with [`Instance::default_call_timeout`]
    /// (`Some(d)` for another allowance, `None` for none).
    #[must_use]
    pub fn from_session(session: Session, sink: BufferSink, cancel: CancelToken) -> Instance {
        Instance::new(session, sink, cancel, Some(UNTRUSTED_CALL_TIMEOUT))
    }

    /// The wall-clock allowance of every call from the embedder that is not
    /// inside a [`CallLimits::timeout`] scope; `None` means none.
    #[must_use]
    pub fn call_timeout(&self) -> Option<Duration> {
        self.call_timeout
    }

    /// Sets the wall-clock allowance of every call from the embedder that
    /// is not inside a [`CallLimits::timeout`] scope (see
    /// [`Engine::call_timeout`]); `None` removes it.
    pub fn default_call_timeout(&mut self, timeout: Option<Duration>) {
        self.call_timeout = timeout;
    }

    /// Refuses to run on a poisoned instance.
    pub(crate) fn guard(&self) -> Result<(), Error> {
        match &self.poisoned {
            Some(why) => Err(Error::Poisoned(why.clone())),
            None => Ok(()),
        }
    }

    /// Maps a session result; an internal error poisons the instance.
    pub(crate) fn conv<T>(&mut self, r: Result<T, vba_runtime::Error>) -> Result<T, Error> {
        r.map_err(|e| {
            if let vba_runtime::Error::Internal(msg) = &e {
                self.poisoned = Some(msg.clone());
            }
            Error::Run(e)
        })
    }

    /// Runs `f` on the session: refuses a poisoned instance, maps the error.
    pub(crate) fn guarded<T>(
        &mut self,
        f: impl FnOnce(&mut Session) -> Result<T, vba_runtime::Error>,
    ) -> Result<T, Error> {
        self.guard()?;
        self.prepare_debugger();
        let budget = self.session.host().budget();
        // The default allowance applies to a call that no scope bounded in
        // time, and only to the outermost one.
        let token = match self.call_timeout {
            Some(timeout) if !budget.has_deadline() => {
                Some(budget.enter_scope(vba_host::ScopeLimits::new().with_timeout(Some(timeout))))
            }
            _ => None,
        };
        let r = f(&mut self.session);
        if let Some(t) = token {
            self.session.host().budget().leave_scope(t);
        }
        self.conv(r)
    }

    /// Whether the instance can take another call right now.
    ///
    /// `false` when
    ///
    /// * the step allowance is spent (`ResourceLimits::max_steps` counts
    ///   steps over the whole life of the instance, nested calls included):
    ///   every call would fail at once. [`reset_step_counter`] gives a new
    ///   allowance (per handler, say), [`reset`] starts over;
    /// * the [`CancelToken`] is still cancelled: calls fail at once until
    ///   `token.reset()`;
    /// * the instance is *poisoned* by an internal error (a bug in vba-rs,
    ///   never a mistake of the VBA program): calls are refused with
    ///   [`Error::Poisoned`] until [`reset`](Instance::reset).
    ///
    /// Any other failure leaves the instance usable. An unhandled VBA error
    /// unwinds its frames properly (locals are released, `Class_Terminate`
    /// runs). A fatal fault (resource limit, cancellation, `End`) stops at
    /// once and discards the frames of the run **without running code**:
    /// objects only those frames held are never terminated, and module
    /// state may be half updated, exactly as after any aborted procedure.
    /// Call [`reset`](Instance::reset) when that matters.
    ///
    /// [`reset_step_counter`]: Instance::reset_step_counter
    /// [`reset`]: Instance::reset
    #[must_use]
    pub fn is_usable(&self) -> bool {
        if self.poisoned.is_some() || self.cancel.is_cancelled() {
            return false;
        }
        let budget = self.session.host().budget();
        !budget
            .limits()
            .max_steps
            .is_some_and(|max| budget.steps_used() >= max)
    }

    /// Runs `f` with the allowances of `limits` in force, then restores what
    /// was in force before (scopes nest, and an inner scope never widens an
    /// outer one). The way to bound one handler in CPU, memory and time
    /// without spending the instance's own budget:
    ///
    /// ```
    /// use vba::{CallLimits, Engine, Error, Project};
    /// use vba::runtime::Error as RunError;
    ///
    /// let project = Project::from_source("Module1", "Public Sub Spin()\n  Do\n  Loop\nEnd Sub\n");
    /// let mut inst = Engine::new().load(&project).unwrap();
    /// inst.trust();
    /// let err = inst
    ///     .with_call_limits(CallLimits::new().max_steps(10_000), |i| i.run("Spin", vec![]))
    ///     .unwrap_err();
    /// assert!(matches!(err, Error::Run(RunError::Limit(_))));
    /// assert!(inst.is_usable()); // the allowance was the call's
    /// ```
    pub fn with_call_limits<T>(
        &mut self,
        limits: CallLimits,
        f: impl FnOnce(&mut Instance) -> T,
    ) -> T {
        let token = self.session.host().budget().enter_scope(limits.to_scope());
        // The guard restores the scope even if `f` unwinds.
        struct Restore(Rc<dyn vba_host::Host>, Option<vba_host::ScopeToken>);
        impl Drop for Restore {
            fn drop(&mut self) {
                if let Some(t) = self.1.take() {
                    self.0.budget().leave_scope(t);
                }
            }
        }
        let _guard = Restore(self.session.host().clone(), Some(token));
        f(self)
    }

    /// The token the instance's host polls: cancelling it stops the running
    /// call and every later one until [`CancelToken::reset`]. It is the
    /// engine's token, shared by all instances of that engine; give each
    /// instance its own engine (or [`Engine::with_cancel_token`]) to cancel
    /// them independently.
    #[must_use]
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    /// Starts a fresh step allowance: the counter of
    /// `ResourceLimits::max_steps` goes back to zero, module state is kept.
    /// The way to give every event handler its own budget.
    pub fn reset_step_counter(&mut self) {
        self.session.host().budget().reset_steps();
    }

    /// Discards all state and makes the instance usable again: module
    /// variables return to their initial values (standard-module
    /// initialisers run again), every object, including instances from
    /// [`create_object`](Instance::create_object), becomes invalid
    /// **without** `Class_Terminate`, `Err`, open files, the budget
    /// counters and the poison flag are cleared. References VBA variables
    /// held to host objects are released; what the *host* keeps (its own
    /// state, event connections it still holds) is the host's to clean up.
    /// The output buffer, the trust decision and the [`CancelToken`] (whose
    /// request, if any, stays until you `reset()` the token) are untouched.
    pub fn reset(&mut self) -> Result<(), Error> {
        let r = self.session.reset();
        self.poisoned = None;
        self.conv(r)
    }

    /// The embedder's decision to let this project's code run.
    pub fn trust(&mut self) {
        self.session.trust();
    }

    /// Whether [`Instance::trust`] was called.
    #[must_use]
    pub fn is_trusted(&self) -> bool {
        self.session.is_trusted()
    }

    /// Compiler warnings from loading.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        self.session.warnings()
    }

    /// Runs a public procedure by `Proc`, `Module.Proc` or `Project.Module.Proc`.
    pub fn run(&mut self, procedure: &str, args: Vec<Variant>) -> Result<Outcome, Error> {
        let before = self.output_mark();
        let r = self.guarded(|s| s.run_procedure(procedure, args));
        let output = self.output_since(before);
        let out = r?;
        Ok(Outcome {
            value: out.value,
            status: out.status,
            output,
        })
    }

    /// Calls a procedure with full control over the arguments.
    ///
    /// * `target` selects the project (`Proc` / `Module.Proc`, like
    ///   [`run`](Instance::run)) or one object instance.
    /// * Arguments may be values, [`Arg::by_ref`] variables (their final
    ///   values come back in [`CallResult::byref`]), [`Arg::missing`] and
    ///   [`Arg::named`] ones. Only parameters declared `ByRef` (the VBA
    ///   default) write back; the value is converted to the declared type
    ///   first (error 13 when it cannot be). A wrong argument count is
    ///   error 450, a missing required argument 449, an unknown parameter
    ///   name 448 and a parameter given twice 446.
    /// * An unhandled error ends the call with an `Err` that carries the
    ///   error, the failing procedure and the call stack.
    ///
    /// ```
    /// use vba::{Arg, Engine, Project, CallTarget, Variant};
    ///
    /// let project = Project::from_source(
    ///     "Module1",
    ///     "Public Sub BeforeUpdate(Cancel As Integer)\n  Cancel = True\nEnd Sub\n",
    /// );
    /// let mut inst = Engine::new().load(&project).unwrap();
    /// inst.trust();
    /// let r = inst
    ///     .call(CallTarget::Project, "BeforeUpdate", vec![Arg::by_ref(Variant::Integer(0))])
    ///     .unwrap();
    /// assert_eq!(r.byref, vec![Variant::Integer(-1)]);
    /// ```
    pub fn call(
        &mut self,
        target: CallTarget,
        procedure: &str,
        args: Vec<Arg>,
    ) -> Result<CallResult, Error> {
        let before = self.output_mark();
        let (positional, named) = split_args(args).map_err(|why| {
            Error::Run(vba_runtime::Error::NotCallable(procedure.to_owned(), why))
        })?;
        let r = self.guarded(|s| s.call(&target.to_runtime(), procedure, positional, named));
        let output = self.output_since(before);
        let out = r?;
        Ok(CallResult {
            value: out.value,
            status: out.status,
            byref: out.byref,
            output,
        })
    }

    /// Delivers a host event (for example `"Workbook"`, `"Open"`).
    pub fn raise_event(
        &mut self,
        source: &str,
        event: &str,
        args: Vec<Variant>,
    ) -> Result<Option<Outcome>, Error> {
        let before = self.output_mark();
        let r = self.guarded(|s| s.raise_event(source, event, args));
        let output = self.output_since(before);
        Ok(r?.map(|out| Outcome {
            value: out.value,
            status: out.status,
            output,
        }))
    }

    /// The length of the output buffer, to be read back with
    /// [`output_since`](Instance::output_since).
    pub(crate) fn output_mark(&self) -> usize {
        self.sink.buffer().borrow().len()
    }

    /// The text printed after `mark`. Never panics, even when a sink the
    /// caller kept (see [`Instance::from_session`]) was truncated meanwhile.
    pub(crate) fn output_since(&self, mark: usize) -> String {
        self.sink
            .buffer()
            .borrow()
            .get(mark..)
            .unwrap_or_default()
            .to_owned()
    }

    /// Everything printed so far, as an owned copy. For a large buffer prefer
    /// [`with_output`](Instance::with_output), which borrows it.
    #[must_use]
    pub fn output(&self) -> String {
        self.sink.contents()
    }

    /// Calls `f` with everything printed so far, without copying the buffer.
    /// `f` must not call back into this instance (the buffer is borrowed
    /// while it runs; printing from a host callback inside `f` would panic).
    pub fn with_output<R>(&self, f: impl FnOnce(&str) -> R) -> R {
        let buffer = self.sink.buffer();
        let text = buffer.borrow();
        f(text.as_str())
    }

    /// Releases module-level objects (running `Class_Terminate`) and closes
    /// files.
    pub fn shutdown(&mut self) -> Result<(), Error> {
        self.guarded(|s| s.shutdown())
    }

    /// The underlying session.
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// The underlying session, mutably.
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}
