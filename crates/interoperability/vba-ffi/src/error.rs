//! Crate-wide error type.

use crate::{CodecError, HandleError, MemoryError, NativeCallError};
use thiserror::Error;

/// Errors raised while validating, marshaling or calling a `Declare`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FfiError {
    /// The declaration breaks a validation rule.
    #[error("invalid Declare signature: {0}")]
    InvalidSignature(String),
    /// A call argument does not fit the declared parameter.
    #[error("argument {index} does not match its declared parameter: {reason}")]
    ArgumentMismatch {
        /// Zero-based argument index.
        index: usize,
        /// What is wrong.
        reason: String,
    },
    /// The construct is valid VBA but marshaling it is not implemented.
    #[error("marshaling not implemented: {0}")]
    Unsupported(&'static str),
    /// Emulated memory fault.
    #[error(transparent)]
    Memory(#[from] MemoryError),
    /// Code page conversion failure.
    #[error(transparent)]
    Codec(#[from] CodecError),
    /// Handle rejected.
    #[error(transparent)]
    Handle(#[from] HandleError),
    /// Native call stage refused or unavailable.
    #[error(transparent)]
    Native(#[from] NativeCallError),
}
