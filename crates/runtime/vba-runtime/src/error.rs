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

pub use vba_vm::StackFrame;

/// The contents of the `Err` object together with where the error was
/// raised: what a host logs or shows when VBA code fails.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct VbaErrorInfo {
    /// `Err.Number`.
    pub number: i32,
    /// `Err.Description`.
    pub description: String,
    /// `Err.Source`; empty if the error did not name one.
    pub source: String,
    /// `Err.HelpFile`.
    pub help_file: String,
    /// `Err.HelpContext`.
    pub help_context: i32,
    /// `Err.LastDllError`.
    pub last_dll_error: i32,
    /// The line to blame: the `Erl` line number when the procedure uses
    /// line numbers, else the 1-based source line of the failing statement,
    /// else 0.
    pub line: u64,
    /// The `Erl` line number in effect, 0 if none.
    pub erl: u64,
    /// The module of the failing procedure.
    pub module: String,
    /// The failing procedure (without the module).
    pub procedure: String,
}

/// An unhandled run-time error with the place it was raised.
///
/// `{}` prints one line; `{:#}` adds the error source and the call stack:
///
/// ```text
/// Run-time error '11': Division by zero (in Form_Orders.Form_BeforeUpdate, source line 12)
///   source: VBAProject
///   at Form_Orders.Form_BeforeUpdate (source line 12)
///   at Module1.Helper (source line 3)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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
    /// Call stack, innermost first (`Module.Procedure`).
    pub stack: Vec<String>,
    /// `Err.HelpFile`.
    pub help_file: String,
    /// `Err.HelpContext`.
    pub help_context: i32,
    /// `Err.LastDllError`.
    pub last_dll_error: i32,
    /// The module of the procedure that raised the error.
    pub module: String,
    /// The 1-based source line of the failing statement, 0 when unknown.
    pub source_line: u32,
    /// The call stack with lines, innermost first. Host code has no
    /// frames: a call that went through a host object shows the VBA frames
    /// only.
    pub frames: Vec<StackFrame>,
}

impl ErrorReport {
    /// A report for an error that was not raised inside a procedure.
    pub fn plain(number: i32, description: impl Into<String>, source: impl Into<String>) -> Self {
        ErrorReport {
            number,
            description: description.into(),
            source: source.into(),
            procedure: String::new(),
            line: 0,
            stack: Vec::new(),
            help_file: String::new(),
            help_context: 0,
            last_dll_error: 0,
            module: String::new(),
            source_line: 0,
            frames: Vec::new(),
        }
    }

    /// The error as the `Err` object shows it, with its location.
    pub fn info(&self) -> VbaErrorInfo {
        let procedure = self
            .procedure
            .split_once('.')
            .map_or(self.procedure.as_str(), |(_, p)| p)
            .to_owned();
        VbaErrorInfo {
            number: self.number,
            description: self.description.clone(),
            source: self.source.clone(),
            help_file: self.help_file.clone(),
            help_context: self.help_context,
            last_dll_error: self.last_dll_error,
            line: if self.line > 0 {
                self.line
            } else {
                u64::from(self.source_line)
            },
            erl: self.line,
            module: self.module.clone(),
            procedure,
        }
    }
}

impl From<vba_vm::RuntimeError> for ErrorReport {
    fn from(r: vba_vm::RuntimeError) -> ErrorReport {
        ErrorReport {
            number: r.error.number,
            description: r.error.description,
            source: r.error.source,
            procedure: r.procedure,
            line: r.line,
            stack: r.stack,
            help_file: r.error.help_file,
            help_context: r.error.help_context,
            last_dll_error: r.last_dll_error,
            module: r.module,
            source_line: r.source_line,
            frames: r.frames,
        }
    }
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
        } else if self.source_line > 0 {
            write!(f, ", source line {}", self.source_line)?;
        }
        f.write_str(")")?;
        if f.alternate() {
            if !self.source.is_empty() {
                write!(f, "\n  source: {}", self.source)?;
            }
            if !self.help_file.is_empty() {
                write!(f, "\n  help: {} #{}", self.help_file, self.help_context)?;
            }
            for fr in &self.frames {
                write!(f, "\n  at {fr}")?;
            }
        }
        Ok(())
    }
}

/// A project that could not be turned into a runnable program.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LoadError {
    /// The rendered error diagnostics (syntax and semantic).
    pub diagnostics: Vec<String>,
}

impl std::error::Error for LoadError {}

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
///
/// Marked non-exhaustive: match with a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The project did not compile. The structured [`LoadError`] is also the
    /// `source()` of this error, so `downcast_ref::<LoadError>()` on it works
    /// (its text repeats this error's, which error-chain printers show twice).
    #[error("{0}")]
    Load(#[source] LoadError),
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
    /// No module-level variable, constant or member with that name.
    #[error("no variable, constant or member named `{0}`")]
    NoSuchMember(String),
    /// A bare name that several modules declare.
    #[error("name `{0}` is ambiguous; qualify it as Module.Name")]
    AmbiguousMember(String),
    /// The member exists but cannot be read or written this way.
    #[error("`{0}` cannot be accessed: {1}")]
    NotAccessible(String, &'static str),
    /// A re-entrant call from a host object could not start: the handle
    /// is stale (kept past its host call) or the nesting limit is reached.
    #[error("re-entrancy: {0}")]
    Reentrancy(String),
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
    /// The operation needs an idle session (no procedure running).
    #[error("a procedure is running")]
    Busy,
    /// An edit of the project was refused (unknown or duplicate module).
    #[error("edit refused: {0}")]
    BadEdit(String),
}

impl Error {
    /// The unhandled VBA error behind this failure, for the families that
    /// carry one ([`Error::Vba`], [`Error::Host`] and
    /// [`Error::PermissionDenied`]).
    pub fn report(&self) -> Option<&ErrorReport> {
        match self {
            Error::Vba(r) | Error::Host(r) | Error::PermissionDenied(r) => Some(r),
            _ => None,
        }
    }

    /// [`ErrorReport::info`] of [`Error::report`].
    pub fn error_info(&self) -> Option<VbaErrorInfo> {
        self.report().map(|r| r.info())
    }

    /// The VBA call stack at the failure, innermost first (empty for the
    /// families that carry no report).
    pub fn stack(&self) -> &[StackFrame] {
        self.report().map_or(&[], |r| r.frames.as_slice())
    }

    /// A multi-line description for logs: the one-line message followed by
    /// the error source and the call stack when there is one.
    pub fn detail(&self) -> String {
        match self.report() {
            Some(r) => format!("{r:#}"),
            None => self.to_string(),
        }
    }

    /// A short stable name of the family, for exit codes and logs.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Load(_) => "load",
            Error::CompiledOnly { .. } => "compiled-only",
            Error::NotTrusted => "not-trusted",
            Error::NoSuchProcedure(_) => "no-such-procedure",
            Error::AmbiguousProcedure(_) => "ambiguous-procedure",
            Error::NotCallable(..) => "not-callable",
            Error::NoSuchMember(_) => "no-such-member",
            Error::AmbiguousMember(_) => "ambiguous-member",
            Error::NotAccessible(..) => "not-accessible",
            Error::Reentrancy(_) => "reentrancy",
            Error::Vba(_) => "vba",
            Error::Host(_) => "host",
            Error::PermissionDenied(_) => "permission-denied",
            Error::Bridge(_) => "bridge",
            Error::Limit(_) => "limit",
            Error::Cancelled(_) => "cancelled",
            Error::Unsupported(_) => "unsupported",
            Error::Internal(_) => "internal",
            Error::Busy => "busy",
            Error::BadEdit(_) => "bad-edit",
        }
    }
}

impl From<VmError> for Error {
    fn from(e: VmError) -> Error {
        match e {
            VmError::Runtime(r) => {
                let origin = r.error.origin;
                let report = Box::new(ErrorReport::from(*r));
                match origin {
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
