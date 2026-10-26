//! [`Engine`]: the builder-style path from a project to a run.

use std::rc::Rc;

use vba_automation::{Dispatch, ObjectTable};
use vba_host::{Capability, HostMetadata, HostResult, Policy, ResourceLimits};
use vba_profile::{Profile, default_profile};
use vba_runtime::{MemFs, RealFs, Session, SessionBuilder, SystemHost};
use vba_value::{ObjectRef, Variant};
use vba_vm::{BufferSink, RunStatus, VmConfig};

use crate::Error;
use crate::host::{CancelToken, EngineHost, Factory};
use crate::project::{Project, Report};

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
    vm: VmConfig,
    cancel: CancelToken,
    globals: Vec<(String, Factory)>,
    classes: Vec<(String, Factory)>,
    files: Files,
    system_environment: bool,
    command: String,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

impl Engine {
    /// An engine with the default profile and no authority at all.
    pub fn new() -> Engine {
        Engine {
            profile: *default_profile(),
            policy: Policy::deny_all(),
            limits: ResourceLimits::default(),
            vm: VmConfig::default(),
            cancel: CancelToken::new(),
            globals: Vec::new(),
            classes: Vec::new(),
            files: Files::Memory(Vec::new()),
            system_environment: false,
            command: String::new(),
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

    /// Sets the resource limits (steps, memory, call depth).
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

    /// Replaces the VM settings (string length, array size, nesting).
    #[must_use]
    pub fn vm_config(mut self, config: VmConfig) -> Engine {
        self.vm = config;
        self
    }

    /// A token that stops sessions of this engine at their next `DoEvents`.
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

    /// The text `Command$` returns.
    #[must_use]
    pub fn command_line(mut self, command: &str) -> Engine {
        self.command = command.to_owned();
        self
    }

    /// Analyses the project with this engine's profile; nothing runs.
    pub fn analyze(&self, project: &Project) -> Report {
        project.analyze(&self.profile)
    }

    /// A session builder configured like this engine, plus the sink that
    /// collects `Debug.Print` text. Use it to add what the engine does not
    /// expose (a `Shell` hook, a custom `TypeProvider`...) and build the
    /// session yourself.
    pub fn session_builder(&self) -> (SessionBuilder, BufferSink) {
        let sink = BufferSink::new();
        let mut host = EngineHost::new(
            HostMetadata::new("vba-rs", env!("CARGO_PKG_VERSION"), self.profile),
            self.policy.clone(),
            self.limits,
            self.cancel.clone(),
        );
        for (n, f) in &self.globals {
            host.globals.insert(n.clone(), f.clone());
        }
        for (n, f) in &self.classes {
            host.classes.insert(n.clone(), f.clone());
        }
        let mut b = Session::builder()
            .host(Rc::new(host))
            .debug_sink(sink.clone())
            .command_line(&self.command)
            .vm_config(self.vm.clone());
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
        Ok(Instance { session, sink })
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
pub struct Outcome {
    /// The function result (`Empty` for a `Sub`).
    pub value: Variant,
    /// How the procedure finished.
    pub status: RunStatus,
    /// The `Debug.Print` text written during this call.
    pub output: String,
}

/// A loaded project: one [`Session`] and its `Debug.Print` buffer.
/// Single-threaded (`!Send`).
pub struct Instance {
    session: Session,
    sink: BufferSink,
}

impl Instance {
    /// The embedder's decision to let this project's code run.
    pub fn trust(&mut self) {
        self.session.trust();
    }

    /// Whether [`Instance::trust`] was called.
    pub fn is_trusted(&self) -> bool {
        self.session.is_trusted()
    }

    /// Compiler warnings from loading.
    pub fn warnings(&self) -> &[String] {
        self.session.warnings()
    }

    /// Runs a public procedure by `Proc` or `Module.Proc`.
    pub fn run(&mut self, procedure: &str, args: Vec<Variant>) -> Result<Outcome, Error> {
        let before = self.sink.contents().len();
        let r = self.session.run_procedure(procedure, args);
        let output = self.sink.contents()[before..].to_owned();
        let out = r.map_err(Error::Run)?;
        Ok(Outcome {
            value: out.value,
            status: out.status,
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
        let before = self.sink.contents().len();
        let r = self.session.raise_event(source, event, args);
        let output = self.sink.contents()[before..].to_owned();
        Ok(r.map_err(Error::Run)?.map(|out| Outcome {
            value: out.value,
            status: out.status,
            output,
        }))
    }

    /// Everything printed so far.
    pub fn output(&self) -> String {
        self.sink.contents()
    }

    /// Releases module-level objects (running `Class_Terminate`) and closes
    /// files.
    pub fn shutdown(&mut self) -> Result<(), Error> {
        self.session.shutdown().map_err(Error::Run)
    }

    /// The underlying session.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// The underlying session, mutably.
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}
