//! `vba`: the stable public facade of vba-rs, a strict, embeddable Rust
//! implementation of Microsoft VBA.
//!
//! The common flow is load, analyse, create an untrusted session, trust it,
//! run a procedure and collect the `Debug.Print` output:
//!
//! ```
//! use vba::{Engine, Project};
//!
//! let project = Project::from_source(
//!     "Module1",
//!     "Public Function Fact(ByVal n As Long) As Long\n\
//!      \x20 If n <= 1 Then Fact = 1 Else Fact = n * Fact(n - 1)\n\
//!      End Function\n\
//!      Public Sub Main()\n\
//!      \x20 Debug.Print \"5! =\"; Fact(5)\n\
//!      End Sub\n",
//! );
//! let engine = Engine::new();
//!
//! // 1. Analyse: diagnostics, nothing runs.
//! assert!(!engine.analyze(&project).has_errors());
//!
//! // 2. Load: compiled into an untrusted instance; no user code has run.
//! let mut instance = engine.load(&project).unwrap();
//! assert!(instance.run("Main", vec![]).is_err()); // NotTrusted
//!
//! // 3. The host decides to trust the code, then runs it.
//! instance.trust();
//! let outcome = instance.run("Main", vec![]).unwrap();
//! assert_eq!(outcome.output, "5! = 120 \n");
//!
//! let fact = instance.run("Module1.Fact", vec![vba::Variant::Long(6)]).unwrap();
//! assert_eq!(fact.value, vba::Variant::Long(720));
//! ```
//!
//! # Embedding a host object
//!
//! A host object is any [`automation::Dispatch`] implementation. Register it
//! under a ProgID and the program can create it with `CreateObject`, which
//! needs the [`Capability::ComCreate`] grant like every other way out of the
//! interpreter.
//!
//! ```
//! use vba::automation::{
//!     DispError, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ObjectTable, ParamInfo,
//! };
//! use vba::{Capability, Engine, Project, Variant};
//!
//! /// `Set g = CreateObject("Demo.Greeter"): g.Greet("Ada")`
//! struct Greeter;
//!
//! impl Dispatch for Greeter {
//!     fn class_name(&self) -> String {
//!         "Greeter".into()
//!     }
//!     fn member_id(&self, name: &str) -> Option<DispId> {
//!         name.eq_ignore_ascii_case("Greet").then_some(DispId(1))
//!     }
//!     fn member_info(&self, id: DispId) -> Option<MemberInfo> {
//!         (id == DispId(1)).then(|| MemberInfo {
//!             name: "Greet".into(),
//!             id,
//!             supports: InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
//!             params: vec![ParamInfo::required("Who").by_val()],
//!         })
//!     }
//!     fn invoke(
//!         &self,
//!         _table: &ObjectTable,
//!         id: DispId,
//!         _flags: InvokeFlags,
//!         params: &mut DispParams,
//!     ) -> Result<Variant, DispError> {
//!         assert_eq!(id, DispId(1));
//!         let who = match &params.args[0].value {
//!             Variant::String(s) => s.to_string_lossy(),
//!             _ => return Err(DispError::TYPE_MISMATCH),
//!         };
//!         Ok(Variant::from(format!("Hello, {who}!").as_str()))
//!     }
//! }
//!
//! let project = Project::from_source(
//!     "Module1",
//!     "Public Sub Main()\n\
//!      \x20 Dim g As Object\n\
//!      \x20 Set g = CreateObject(\"Demo.Greeter\")\n\
//!      \x20 Debug.Print g.Greet(\"Ada\")\n\
//!      End Sub\n",
//! );
//! let engine = Engine::new()
//!     .grant(Capability::ComCreate)
//!     .dispatch_class("Demo.Greeter", || Greeter);
//! let out = engine.run(&project, "Main", vec![]).unwrap();
//! assert_eq!(out.output, "Hello, Ada!\n");
//! ```
//!
//! # Denied permissions
//!
//! Without a grant the request fails as VBA error 70 (Permission denied); if
//! the program does not handle it the run ends with
//! [`runtime::Error::PermissionDenied`].
//!
//! ```
//! use vba::{Engine, Error, Project};
//! use vba::runtime::Error as RunError;
//!
//! let project = Project::from_source(
//!     "Module1",
//!     "Public Sub Main()\n  Dim o As Object\n  Set o = CreateObject(\"Scripting.Dictionary\")\nEnd Sub\n",
//! );
//! match Engine::new().run(&project, "Main", vec![]) {
//!     Err(Error::Run(RunError::PermissionDenied(report))) => assert_eq!(report.number, 70),
//!     other => panic!("expected a denial, got {other:?}"),
//! }
//!
//! // A program can handle the denial itself.
//! let handled = Project::from_source(
//!     "Module1",
//!     "Public Function Try() As Long\n  On Error Resume Next\n  CreateObject \"X.Y\"\n  Try = Err.Number\nEnd Function\n",
//! );
//! let out = Engine::new().run(&handled, "Try", vec![]).unwrap();
//! assert_eq!(out.value, vba::Variant::Long(70));
//! ```
//!
//! # Resource limits and cancellation
//!
//! A step budget stops runaway loops; a [`CancelToken`] stops a program the
//! next time the interpreter polls it (before each call, at `DoEvents` and
//! every 1024 steps). Both end the run with a classified error that
//! `On Error` cannot swallow.
//!
//! ```
//! use vba::runtime::Error as RunError;
//! use vba::{Engine, Error, Project};
//!
//! let spin = Project::from_source("Module1", "Public Sub Spin()\n  Do\n  Loop\nEnd Sub\n");
//! let err = Engine::new().max_steps(10_000).run(&spin, "Spin", vec![]).unwrap_err();
//! assert!(matches!(err, Error::Run(RunError::Limit(_))));
//!
//! let polite = Project::from_source(
//!     "Module1",
//!     "Public Sub Wait()\n  Do\n    DoEvents\n  Loop\nEnd Sub\n",
//! );
//! let engine = Engine::new();
//! engine.cancel_token().cancel(); // e.g. from another thread
//! let err = engine.run(&polite, "Wait", vec![]).unwrap_err();
//! assert!(matches!(err, Error::Run(RunError::Cancelled(_))));
//! ```
//!
//! # Hosting forms
//!
//! Applications that run event handlers of forms create one instance per form
//! with a host object as its `Me`, raise events with `ByRef` arguments (such as
//! `Cancel`) and read the result back:
//! [`Instance::create_object`], [`Instance::raise_event_on`],
//! [`Instance::call`], [`Instance::get_global`], [`HostContext`] for host
//! members that call back into the running session, and [`Error`] with call
//! stacks. The whole flow, with compiled examples and the semantics of
//! re-entrancy, limits and cancellation, is in `docs/architecture/embedding-guide.md`;
//! a runnable simulation is `examples/form_events.rs`.
//!
//! # Reading and editing a project with preservation
//!
//! ```
//! use vba::{Project, ModuleKind};
//!
//! // Build a project and write it as a vbaProject.bin.
//! let mut project = Project::new();
//! project.add_module("Module1", ModuleKind::Standard, "Sub A()\r\nEnd Sub\r\n").unwrap();
//! project.add_module("Module2", ModuleKind::Standard, "Sub B()\r\nEnd Sub\r\n").unwrap();
//! let bytes = project.to_bytes().unwrap();
//!
//! // Read it back, edit one module's source, write it again.
//! let mut loaded = Project::from_bytes(&bytes).unwrap();
//! assert_eq!(loaded.module_source("module1").unwrap(), "Sub A()\r\nEnd Sub\r\n");
//! loaded
//!     .set_module_source("Module1", "Sub A()\r\n  Debug.Print 1\r\nEnd Sub\r\n")
//!     .unwrap();
//! let edited = Project::from_bytes(&loaded.to_bytes().unwrap()).unwrap();
//!
//! assert!(edited.module_source("Module1").unwrap().contains("Debug.Print 1"));
//! // The untouched module and the project metadata are unchanged.
//! assert_eq!(edited.module_source("Module2"), project.module_source("Module2"));
//! assert_eq!(edited.name(), project.name());
//! ```
//!
//! # Layers
//!
//! The crate re-exports a curated surface at the top level and in the
//! [`prelude`]. Everything else stays reachable in [`raw`], one module per
//! workspace crate, with the semver promises of that crate (the facade's own
//! items are the stable ones). See `docs/architecture/public-api.md` for the embedder
//! contract: threading, authority, determinism and the error taxonomy.
//!
//! # Features
//!
//! None is enabled by default and none grants authority:
//!
//! * `winapi`: the Win32 behavior catalog and portable shims (`raw::winapi`).
//! * `native-call`: the dynamic native call stage of `Declare` (the only
//!   `unsafe`), still behind the `NativeDeclare` capability.
//! * `wine`: the opt-in Wine bridge client (`raw::wine`), implies `winapi`.

mod call;
mod context;
mod engine;
mod host;
mod objects;
mod project;
mod state;

pub mod prelude;
pub mod raw;

pub use call::{Arg, CallLimits, CallResult, CallTarget};
pub use context::HostContext;
pub use engine::{Engine, Instance, Outcome};
pub use host::{CancelToken, Factory};
pub use objects::EventOutcome;
pub use project::{DEFAULT_PROJECT_NAME, Project, Report};

// Profiles.
pub use vba_profile::{
    Bitness, HostApplication, LanguageGeneration, OperatingSystem, Profile, Target,
    default_profile, find_profile, profiles,
};

// Project model and format.
pub use vba_ast::ProcKind;
pub use vba_format::{
    Limits as ReadLimits, ProjectStreams, ReadMode, StaleCache, WriteOptions, WriteReport,
};
pub use vba_project::{
    Module, ModuleKind, ProjectVersion, Reference, ReferenceHint, SysKind, VbaProject,
};

// Diagnostics.
pub use vba_diagnostics::{Diagnostic, Severity};

// Sessions and runtime.
pub use vba_runtime::{
    Error as RunError, ErrorReport, MemFs, ModuleSummary, NoShell, ParamSummary, ProcedureSummary,
    RealFs, Session, SessionBuilder, ShellHook, StackFrame, StdShell, SystemHost, VariableSummary,
    VbaErrorInfo, Vfs,
};
pub use vba_vm::{BufferSink, DebugSink, LimitKind, RunStatus, VmConfig};

// Values.
pub use vba_value::{Currency, Date, Decimal, VarType, Variant, VbaArray, VbaString};

// Host contract and policy.
pub use vba_host::{
    Capability, HeadlessHost, Host, HostMetadata, HostResult, Policy, ResourceLimits, ScriptedHost,
};

/// The automation traits an embedder implements to expose host objects.
pub mod automation {
    pub use vba_automation::dispatch::{
        Arg, Bound, DispId, DispParams, Dispatch, DispatchRef, InvokeFlags, MemberInfo, ParamInfo,
        bind,
    };
    pub use vba_automation::testing;
    pub use vba_automation::{
        AutoResult, CallArgs, DispError, ErrInfo, Final, ObjectHandle, ObjectRef, ObjectTable,
        Step, VariantEnum,
    };
    pub use vba_automation::{ConnectionPoint, Cookie, EventSink, EventSource};
}

/// The runtime layer (sessions, file systems, errors) in full.
pub mod runtime {
    pub use vba_runtime::*;
}

/// The facade's error type.
///
/// Marked non-exhaustive: match with a wildcard arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A `vbaProject.bin` could not be read or written.
    #[error("project format: {0}")]
    Format(vba_format::Error),
    /// A project edit was rejected (duplicate or unknown module).
    #[error("project model: {0}")]
    Model(vba_project::ModelError),
    /// A standalone source file could not be parsed.
    #[error("source file: {0}")]
    Source(String),
    /// A file could not be read.
    #[error("i/o: {0}")]
    Io(String),
    /// Loading or running failed; see [`runtime::Error`] for the taxonomy.
    #[error(transparent)]
    Run(vba_runtime::Error),
    /// The instance hit an internal error and refuses further calls until
    /// [`Instance::reset`] (see [`Instance::is_usable`]).
    #[error("the instance is poisoned by an internal error ({0}); call reset()")]
    Poisoned(String),
}

impl Error {
    /// A short stable name of the family, for logs and exit codes.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Format(_) => "format",
            Error::Model(_) => "model",
            Error::Source(_) => "source",
            Error::Io(_) => "io",
            Error::Run(e) => e.kind(),
            Error::Poisoned(_) => "poisoned",
        }
    }
}

/// Compiles and runs the code blocks of the embedding guide as doctests.
#[cfg(doctest)]
#[doc = include_str!("../../../../docs/architecture/embedding-guide.md")]
pub struct EmbeddingGuide;
