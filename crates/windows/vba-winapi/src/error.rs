//! Errors of the provider framework.

use thiserror::Error;
use vba_ffi::MemoryError;

/// Why a Win32 call could not be carried out.
///
/// Win32 failures that the real function reports through its return value and
/// `GetLastError` are NOT errors here; they are ordinary results. This type is
/// for things the framework itself refuses to do.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WinApiError {
    /// The symbol is not implemented. Never a fake success.
    #[error("Unsupported: {symbol}: {reason}")]
    Unsupported {
        /// `library!symbol` (or the unsupported feature of it).
        symbol: String,
        /// Why.
        reason: String,
    },
    /// The call arguments are structurally wrong for the symbol.
    #[error("{symbol}: bad argument {index}: {reason}")]
    BadArguments {
        /// Symbol called.
        symbol: &'static str,
        /// Zero-based argument index (or the count, for arity errors).
        index: usize,
        /// What is wrong.
        reason: &'static str,
    },
    /// An emulated memory fault (real Windows would crash or corrupt).
    #[error(transparent)]
    Memory(#[from] MemoryError),
    /// Two providers export the same symbol.
    #[error("duplicate export {0}")]
    DuplicateExport(String),
    /// A host identity value is invalid.
    #[error("invalid identity: {0}")]
    InvalidIdentity(&'static str),
}

impl WinApiError {
    pub(crate) fn unsupported(symbol: impl Into<String>, reason: impl Into<String>) -> Self {
        WinApiError::Unsupported {
            symbol: symbol.into(),
            reason: reason.into(),
        }
    }
}
