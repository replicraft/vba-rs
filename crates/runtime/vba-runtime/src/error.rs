//! The error taxonomy of a session.
//!
//! Failures are kept apart by *who* is responsible: the VBA program
//! ([`Error::Vba`]), the host application ([`Error::Host`],
//! [`Error::PermissionDenied`]), the native `Declare` bridge
//! ([`Error::Bridge`]), a configured resource limit ([`Error::Limit`]), the
//! embedder asking to stop ([`Error::Cancelled`]), or this implementation
//! ([`Error::Unsupported`], [`Error::Internal`]). Only the first four
//! families can be produced by a *running* program; project loading problems
//! are [`Error::Load`], embedder mistakes are [`Error::NotTrusted`],
//! [`Error::NoSuchProcedure`], [`Error::AmbiguousProcedure`] and
//! [`Error::NotCallable`].

use std::fmt;

use vba_automation::ErrInfo;
use vba_vm::{ErrOrigin, LimitKind, VmError};

/// An unhandled run-time error with the place it was raised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorReport {
    /// `Err.Number`.
    pub number: i32,
    /// `Err.Description`.
    pub description: String,
    /// `Err.Source`; empty if the error did not name one.
    pub source: String,
    /// `Module.Procedure` that raised it.
    pub procedure: String,
    /// The `Erl` line in effect, 0 if none.
    pub line: u64,
    /// Call stack, innermost first.
    pub stack: Vec<String>,
}

impl fmt::Display for ErrorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Run-time error '{}': {} (in {}",
            self.number, self.description, self.procedure
        )?;
        if self.line > 0 {
            write!(f, ", line {}", self.line)?;
        }
        f.write_str(")")
    }
}

/// A project that could not be turned into a runnable program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    /// The rendered error diagnostics (syntax and semantic).
    pub diagnostics: Vec<String>,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the project has {} error(s)", self.diagnostics.len())?;
        for d in &self.diagnostics {
            write!(f, "\n{d}")?;
        }
        Ok(())
    }
}

fn compiled_only_message(project: &str, modules: &[String]) -> String {
    format!(
        "project {project:?} has no source for module(s) {}; compiled-only or stripped \
         modules cannot be run (their compiled form is never executed)",
        modules
            .iter()
            .map(|m| format!("{m:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Everything that can go wrong in a [`Session`](crate::Session).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The project did not compile.
    #[error("{0}")]
    Load(LoadError),
    /// `run_procedure` / `raise_event` before [`Session::trust`](crate::Session::trust).
    #[error("execution is not enabled for this session; the embedder must call `trust` first")]
    NotTrusted,
    /// The project (or the module a call needs) has no source text: it is
    /// compiled-only or stripped. Compiled state is never executed.
    #[error("{}", compiled_only_message(.project, .modules))]
    CompiledOnly {
        /// The project name.
        project: String,
        /// The modules lacking source, in project order.
        modules: Vec<String>,
    },
    /// No procedure with that name.
    #[error("no procedure named `{0}`")]
    NoSuchProcedure(String),
    /// A bare name that exists in more than one module.
    #[error("procedure name `{0}` is ambiguous; qualify it as Module.Name")]
    AmbiguousProcedure(String),
    /// The procedure cannot be run by name (private, or in a class module).
    #[error("procedure `{0}` cannot be run by name: {1}")]
    NotCallable(String, &'static str),
    /// An unhandled error raised by the VBA program or the language.
    #[error("{0}")]
    Vba(Box<ErrorReport>),
    /// An unhandled error reported by a host hook or host object.
    #[error("host error: {0}")]
    Host(Box<ErrorReport>),
    /// A capability was refused by the host policy (error 70).
    #[error("permission denied: {0}")]
    PermissionDenied(Box<ErrorReport>),
    /// The native `Declare` bridge could not carry out a call.
    #[error("native bridge: {0}")]
    Bridge(String),
    /// A resource limit stopped the run.
    #[error("resource limit: {0}")]
    Limit(LimitKind),
    /// The host cancelled the run.
    #[error("cancelled: {0}")]
    Cancelled(String),
    /// A feature this implementation does not provide.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// A broken invariant (a bug).
    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    /// A short stable name of the family, for exit codes and logs.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Load(_) => "load",
            Error::CompiledOnly { .. } => "compiled-only",
            Error::NotTrusted => "not-trusted",
            Error::NoSuchProcedure(_) => "no-such-procedure",
            Error::AmbiguousProcedure(_) => "ambiguous-procedure",
            Error::NotCallable(..) => "not-callable",
            Error::Vba(_) => "vba",
            Error::Host(_) => "host",
            Error::PermissionDenied(_) => "permission-denied",
            Error::Bridge(_) => "bridge",
            Error::Limit(_) => "limit",
            Error::Cancelled(_) => "cancelled",
            Error::Unsupported(_) => "unsupported",
            Error::Internal(_) => "internal",
        }
    }
}

impl From<VmError> for Error {
    fn from(e: VmError) -> Error {
        match e {
            VmError::Runtime(r) => {
                let report = Box::new(ErrorReport {
                    number: r.error.number,
                    description: r.error.description,
                    source: r.error.source,
                    procedure: r.procedure,
                    line: r.line,
                    stack: r.stack,
                });
                match r.error.origin {
                    ErrOrigin::Vba => Error::Vba(report),
                    ErrOrigin::Host => Error::Host(report),
                    ErrOrigin::Denied => Error::PermissionDenied(report),
                    ErrOrigin::Bridge => Error::Bridge(report.to_string()),
                }
            }
            VmError::Limit(l) => Error::Limit(l),
            VmError::Cancelled(s) => Error::Cancelled(s),
            VmError::Unsupported(s) => Error::Unsupported(s),
            VmError::Bridge(s) => Error::Bridge(s),
            VmError::Internal(s) => Error::Internal(s),
        }
    }
}

/// An internal file system failure before it becomes a VBA error.
#[derive(Debug, Clone, PartialEq)]
pub enum FsError {
    /// A VBA error number with its standard description.
    Vba(i32),
    /// A full VBA error (for example a permission denial from the host).
    Info(ErrInfo),
    /// A feature that is not implemented.
    Unsupported(String),
    /// A resource limit.
    Limit(LimitKind),
}

impl From<ErrInfo> for FsError {
    fn from(e: ErrInfo) -> FsError {
        FsError::Info(e)
    }
}

impl From<FsError> for vba_vm::ServiceError {
    fn from(e: FsError) -> vba_vm::ServiceError {
        match e {
            FsError::Vba(n) => vba_vm::ServiceError::Vba(ErrInfo::new(n, vba_vm::describe(n))),
            FsError::Info(i) => vba_vm::ServiceError::Vba(i),
            FsError::Unsupported(s) => vba_vm::ServiceError::Unsupported(s),
            FsError::Limit(l) => vba_vm::ServiceError::Limit(l),
        }
    }
}
