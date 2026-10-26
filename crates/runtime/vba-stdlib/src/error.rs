//! Error type of the standard library.

use std::fmt;
use vba_value::VbaError;

/// Error 449, "Argument not optional".
pub const ARGUMENT_NOT_OPTIONAL: VbaError = VbaError::new(449);
/// Error 450, "Wrong number of arguments or invalid property assignment".
pub const WRONG_ARGUMENT_COUNT: VbaError = VbaError::new(450);
/// Error 93, "Invalid pattern string".
pub const INVALID_PATTERN: VbaError = VbaError::new(93);

/// Failure of a standard-library call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdError {
    /// A VBA run-time error that user code can trap with `On Error`.
    Vba(VbaError),
    /// A documented behavior that this implementation does not provide (for
    /// example behavior that depends on a Windows code page). This is never a
    /// VBA error: the host should report it as an implementation limit.
    Unsupported(&'static str),
    /// The function needs information only the VM has (`IsMissing`, object
    /// class names, ...). The VM must implement it itself.
    VmRequired(&'static str),
    /// The caller's work budget (steps) ran out while the function was
    /// scanning its arguments. Never a VBA error: the host ends the run.
    WorkExceeded,
    /// The host cancelled the run (or its time allowance ran out) while the
    /// function was working. Never a VBA error: the host ends the run.
    Cancelled(String),
}

/// Result alias used throughout the crate.
pub type StdResult<T> = Result<T, StdError>;

impl From<VbaError> for StdError {
    fn from(e: VbaError) -> Self {
        StdError::Vba(e)
    }
}

impl StdError {
    /// The VBA error, if this is one.
    pub fn vba(&self) -> Option<VbaError> {
        match self {
            StdError::Vba(e) => Some(*e),
            _ => None,
        }
    }
}

impl fmt::Display for StdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StdError::Vba(e) => write!(f, "{e}"),
            StdError::Unsupported(what) => write!(f, "unsupported: {what}"),
            StdError::VmRequired(what) => write!(f, "requires the VM: {what}"),
            StdError::WorkExceeded => f.write_str("work budget exceeded"),
            StdError::Cancelled(why) => write!(f, "cancelled: {why}"),
        }
    }
}

impl std::error::Error for StdError {}
