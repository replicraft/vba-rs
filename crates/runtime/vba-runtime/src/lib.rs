//! Session orchestration for vba-rs: loading a project, the trust gate, a
//! virtual file system and by-name procedure invocation on top of `vba-vm`.
//!
//! A [`Session`] is built from a `vba-project` project or from source
//! modules: the modules are parsed, analysed, lowered to IR and given a VM
//! of their own (so module variables and default instances are isolated per
//! session). Nothing runs while loading. The embedder calls
//! [`Session::trust`] and then [`Session::run_procedure`] or
//! [`Session::raise_event`]; imported code never runs by itself.
//!
//! Everything the program can reach is injected and denied by default:
//! the [`vba_host::Host`] decides permissions, limits, objects and UI;
//! file statements run over a [`Vfs`] ([`MemFs`] in memory, [`RealFs`] on
//! disk) after the host policy allowed the path; `Shell` needs a
//! [`ShellHook`]; the clock and environment come from a
//! `vba_stdlib::Host` ([`SystemHost`] reads the real ones).
//!
//! ```
//! use vba_runtime::Session;
//! use vba_sema::SourceModule;
//! use vba_project::ModuleKind;
//! use vba_vm::BufferSink;
//!
//! let sink = BufferSink::new();
//! let mut session = Session::builder()
//!     .debug_sink(sink.clone())
//!     .build_sources(&[SourceModule {
//!         name: "Module1".into(),
//!         kind: ModuleKind::Standard,
//!         source: b"Sub Hello()\nDebug.Print \"hi\"\nEnd Sub\n".to_vec(),
//!     }])
//!     .unwrap();
//! assert!(session.run_procedure("Hello", vec![]).is_err()); // not trusted yet
//! session.trust();
//! session.run_procedure("module1.hello", vec![]).unwrap();
//! assert_eq!(sink.contents(), "hi\n");
//! ```

mod error;
mod files;
mod host_context;
mod members;
mod memfs;
mod ops;
mod realfs;
mod records;
mod session;
mod shell;
mod system;
mod vfs;

pub use error::{Error, ErrorReport, FsError, LoadError, StackFrame, VbaErrorInfo};
pub use files::{DEFAULT_MAX_FILE_BYTES, FileSystem, FsServices, ScriptFsAdapter, write_value};
pub use host_context::HostContext;
pub use members::{ModuleSummary, ParamSummary, ProcedureSummary, VariableSummary};
pub use memfs::MemFs;
pub use ops::{Ops, Target};
pub use realfs::RealFs;
pub use session::{ProcedureInfo, Session, SessionBuilder};
pub use shell::{NoShell, ShellHook, StdShell};
pub use system::SystemHost;
pub use vfs::{
    ATTR_ARCHIVE, ATTR_DIRECTORY, ATTR_HIDDEN, ATTR_READONLY, ATTR_SYSTEM, DirEntry, Stat, Vfs,
    VfsError, VfsResult, join, normalize, split_parent, wildcard,
};
