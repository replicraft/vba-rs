//! Errors of the interpreter.
//!
//! Inside the VM an interruption is a [`Fault`]. Only [`Fault::Err`] is a
//! VBA run-time error that `On Error` can trap; every other fault (limits,
//! cancellation, missing features, `End`, `Stop`) always unwinds to the
//! caller of the VM. The final outcome of a run is a [`VmError`].

use std::fmt;

use vba_automation::ErrInfo;
use vba_host::LimitExceeded;
use vba_value::VbaError;

/// Where a run-time error came from. The origin never changes how
/// `On Error` treats the error; it only classifies an *unhandled* error for
/// the embedder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrOrigin {
    /// Raised by the language itself (`Err.Raise`, arithmetic, subscripts).
    Vba,
    /// Reported by a host hook, object or service.
    Host,
    /// A permission check failed (error 70).
    Denied,
    /// The `Declare` bridge (vba-winapi / vba-ffi) reported a failure.
    Bridge,
}

/// The standard description of a VBA run-time error number (the text of
/// `Err.Description` when the error is raised without one).
pub fn describe(number: i32) -> String {
    let s = match number {
        16 => "Expression too complex",
        17 => "Can't perform requested operation",
        18 => "User interrupt occurred",
        20 => "Resume without error",
        52 => "Bad file name or number",
        54 => "Bad file mode",
        55 => "File already open",
        57 => "Device I/O error",
        58 => "File already exists",
        59 => "Bad record length",
        61 => "Disk full",
        62 => "Input past end of file",
        63 => "Bad record number",
        67 => "Too many files",
        68 => "Device unavailable",
        71 => "Disk not ready",
        74 => "Can't rename with different drive",
        75 => "Path/File access error",
        76 => "Path not found",
        321 => "Invalid file format",
        380 => "Invalid property value",
        381 => "Invalid property array index",
        382 => "Set not supported at runtime",
        383 => "Set not supported (read-only property)",
        385 => "Need property array index",
        387 => "Set not permitted",
        393 => "Get not supported at runtime",
        394 => "Get not supported (write-only property)",
        422 => "Property not found",
        423 => "Property or method not found",
        440 => "Automation error",
        443 => "Automation object does not have a default value",
        459 => "Object or class does not support the set of events",
        n => return vba_automation::error::describe(n),
    };
    s.to_owned()
}

/// A VBA run-time error with the contents of the `Err` object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtErr {
    /// `Err.Number`.
    pub number: i32,
    /// `Err.Description`.
    pub description: String,
    /// `Err.Source`; empty means "the project name".
    pub source: String,
    /// `Err.HelpFile`.
    pub help_file: String,
    /// `Err.HelpContext`.
    pub help_context: i32,
    /// Where it came from.
    pub origin: ErrOrigin,
}

impl RtErr {
    /// An error with the standard description of `number`.
    pub fn standard(number: i32) -> RtErr {
        RtErr::new(number, describe(number))
    }

    /// An error with an explicit description.
    pub fn new(number: i32, description: impl Into<String>) -> RtErr {
        RtErr {
            number,
            description: description.into(),
            source: String::new(),
            help_file: String::new(),
            help_context: 0,
            origin: ErrOrigin::Vba,
        }
    }

    /// Sets the origin.
    #[must_use]
    pub fn with_origin(mut self, origin: ErrOrigin) -> RtErr {
        self.origin = origin;
        self
    }
}

impl From<VbaError> for RtErr {
    fn from(e: VbaError) -> RtErr {
        RtErr::standard(e.number())
    }
}

impl From<ErrInfo> for RtErr {
    fn from(e: ErrInfo) -> RtErr {
        let origin = if e.number == 70 {
            ErrOrigin::Denied
        } else {
            ErrOrigin::Host
        };
        RtErr {
            number: e.number,
            description: e.description,
            source: e.source,
            help_file: e.help_file,
            help_context: e.help_context as i32,
            origin,
        }
    }
}

/// Which resource limit stopped the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LimitKind {
    /// The step budget (`ResourceLimits::max_steps`).
    Steps,
    /// The memory budget (`ResourceLimits::max_memory_bytes`).
    Memory,
    /// The call depth (`ResourceLimits::max_call_depth`).
    CallDepth,
    /// A single string longer than `VmConfig::max_string_units`.
    StringLength,
    /// A single array larger than `VmConfig::max_array_elements`.
    ArraySize,
}

impl fmt::Display for LimitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LimitKind::Steps => "step limit exceeded",
            LimitKind::Memory => "memory limit exceeded",
            LimitKind::CallDepth => "call depth limit exceeded",
            LimitKind::StringLength => "string length limit exceeded",
            LimitKind::ArraySize => "array size limit exceeded",
        })
    }
}

impl From<LimitExceeded> for LimitKind {
    fn from(l: LimitExceeded) -> LimitKind {
        match l {
            LimitExceeded::Steps => LimitKind::Steps,
            LimitExceeded::Memory => LimitKind::Memory,
            LimitExceeded::CallDepth => LimitKind::CallDepth,
        }
    }
}

/// An interruption of normal execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// A trappable VBA run-time error.
    Err(RtErr),
    /// `End`.
    End,
    /// `Stop` or a failed `Debug.Assert`.
    Stop,
    /// A resource limit; never trappable.
    Limit(LimitKind),
    /// The host asked to stop (idle hook error).
    Cancelled(String),
    /// A feature this implementation does not provide; never trappable.
    Unsupported(String),
    /// The `Declare` bridge (vba-ffi / vba-winapi) cannot carry out a call;
    /// never trappable.
    Bridge(String),
    /// An internal inconsistency (a bug or malformed IR).
    Internal(String),
}

impl From<VbaError> for Fault {
    fn from(e: VbaError) -> Fault {
        Fault::Err(e.into())
    }
}

impl From<RtErr> for Fault {
    fn from(e: RtErr) -> Fault {
        Fault::Err(e)
    }
}

impl From<ErrInfo> for Fault {
    fn from(e: ErrInfo) -> Fault {
        Fault::Err(e.into())
    }
}

impl From<LimitExceeded> for Fault {
    fn from(l: LimitExceeded) -> Fault {
        Fault::Limit(l.into())
    }
}

impl From<vba_stdlib::StdError> for Fault {
    fn from(e: vba_stdlib::StdError) -> Fault {
        match e {
            vba_stdlib::StdError::Vba(v) => v.into(),
            vba_stdlib::StdError::Unsupported(s) => Fault::Unsupported(format!("stdlib: {s}")),
            vba_stdlib::StdError::WorkExceeded => Fault::Limit(LimitKind::Steps),
            vba_stdlib::StdError::Cancelled(why) => Fault::Cancelled(why),
            vba_stdlib::StdError::VmRequired(s) => {
                Fault::Unsupported(format!("stdlib function needs VM support: {s}"))
            }
        }
    }
}

/// Convenience alias.
pub type VResult<T> = Result<T, Fault>;

/// One frame of the call stack at the point of an error.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct StackFrame {
    /// The module of the procedure.
    pub module: String,
    /// The procedure name (without the module).
    pub procedure: String,
    /// The line number (`Erl`) in effect in that frame, 0 if none.
    pub line: u64,
    /// The 1-based source line of the statement being executed in that
    /// frame (in the module source as it was given to the engine), 0 when
    /// unknown.
    pub source_line: u32,
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.module, self.procedure)?;
        match (self.line, self.source_line) {
            (0, 0) => Ok(()),
            (0, s) => write!(f, " (source line {s})"),
            (l, 0) => write!(f, " (line {l})"),
            (l, s) => write!(f, " (line {l}, source line {s})"),
        }
    }
}

/// An unhandled VBA run-time error that ended a run.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RuntimeError {
    /// The error.
    pub error: RtErr,
    /// `Module.Procedure` where the error was raised.
    pub procedure: String,
    /// The line number (`Erl`) in effect, 0 if none.
    pub line: u64,
    /// The call stack at the time, innermost first (`Module.Procedure`).
    pub stack: Vec<String>,
    /// The module of the procedure that raised the error.
    pub module: String,
    /// The 1-based source line of the failing statement, 0 when unknown.
    pub source_line: u32,
    /// The call stack with lines, innermost first.
    pub frames: Vec<StackFrame>,
    /// `Err.LastDllError` at the time.
    pub last_dll_error: i32,
}

impl RuntimeError {
    /// An error that was not raised inside a procedure.
    pub fn new(error: RtErr) -> RuntimeError {
        RuntimeError {
            error,
            procedure: String::new(),
            line: 0,
            stack: Vec::new(),
            module: String::new(),
            source_line: 0,
            frames: Vec::new(),
            last_dll_error: 0,
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Run-time error '{}': {} (in {}",
            self.error.number, self.error.description, self.procedure
        )?;
        if self.line > 0 {
            write!(f, ", line {}", self.line)?;
        }
        f.write_str(")")
    }
}

/// How a run ended abnormally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    /// An unhandled VBA run-time error.
    Runtime(Box<RuntimeError>),
    /// A resource limit was exceeded.
    Limit(LimitKind),
    /// The host cancelled the run.
    Cancelled(String),
    /// A feature that is not implemented (distinct from a VBA error).
    Unsupported(String),
    /// A `Declare` call that the native bridge cannot carry out: the symbol
    /// has no provider, an argument cannot be marshaled, or the target does
    /// not match.
    Bridge(String),
    /// A broken invariant; indicates a bug.
    Internal(String),
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::Runtime(e) => e.fmt(f),
            VmError::Limit(l) => write!(f, "resource limit: {l}"),
            VmError::Cancelled(s) => write!(f, "cancelled: {s}"),
            VmError::Unsupported(s) => write!(f, "unsupported: {s}"),
            VmError::Bridge(s) => write!(f, "native bridge: {s}"),
            VmError::Internal(s) => write!(f, "internal error: {s}"),
        }
    }
}

impl std::error::Error for VmError {}
