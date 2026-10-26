//! Bridge errors. Every failure of the bridge is a distinct variant; none is
//! ever turned into a successful result.

use crate::protocol::{ProtocolError, RemoteError, RemoteErrorKind};
use std::time::Duration;
use thiserror::Error;
use vba_ffi::HandleError;

/// Why a bridge operation failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WineError {
    /// Wine or the helper is not configured or not present. Nothing was started.
    #[error("Unavailable: {0}")]
    Unavailable(String),
    /// The pipes failed (spawn, write, unexpected I/O error).
    #[error("transport failure: {0}")]
    Transport(String),
    /// The helper sent something that violates the protocol.
    #[error("protocol violation: {0}")]
    Protocol(String),
    /// Client and helper speak different protocol versions.
    #[error("protocol version mismatch: client {client}, helper {helper}")]
    VersionMismatch {
        /// Client version.
        client: u16,
        /// Helper version (0 when the helper did not say).
        helper: u16,
    },
    /// The helper process ended (or its pipe closed) while a request was pending.
    #[error("helper crashed or exited: {0}")]
    HelperCrashed(String),
    /// The helper did not answer in time; it was killed.
    #[error("helper timed out during {op} after {after:?}; it was killed")]
    Timeout {
        /// The operation that was pending.
        op: &'static str,
        /// The configured timeout.
        after: Duration,
    },
    /// `LoadLibrary` failed in the helper.
    #[error("library not found: {0}")]
    LibraryNotFound(String),
    /// `GetProcAddress` failed in the helper.
    #[error("symbol not found: {0}")]
    SymbolNotFound(String),
    /// The helper could not perform the call.
    #[error("call fault: {0}")]
    CallFault(String),
    /// A policy (host grant or helper) refused the request.
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    /// The helper or the bridge does not support this.
    #[error("Unsupported: {0}")]
    Unsupported(String),
    /// A handle was unknown, closed, or belongs to another session or provider.
    #[error("invalid handle: {0}")]
    InvalidHandle(String),
    /// The request was rejected locally before anything was sent.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// The session already failed and was shut down; start a new one.
    #[error("session is closed after an earlier failure")]
    SessionClosed,
}

impl WineError {
    /// A short stable name for the failure category.
    pub fn kind(&self) -> &'static str {
        match self {
            WineError::Unavailable(_) => "Unavailable",
            WineError::Transport(_) => "Transport",
            WineError::Protocol(_) => "Protocol",
            WineError::VersionMismatch { .. } => "VersionMismatch",
            WineError::HelperCrashed(_) => "HelperCrashed",
            WineError::Timeout { .. } => "Timeout",
            WineError::LibraryNotFound(_) => "LibraryNotFound",
            WineError::SymbolNotFound(_) => "SymbolNotFound",
            WineError::CallFault(_) => "CallFault",
            WineError::PermissionDenied(_) => "PermissionDenied",
            WineError::Unsupported(_) => "Unsupported",
            WineError::InvalidHandle(_) => "InvalidHandle",
            WineError::BadRequest(_) => "BadRequest",
            WineError::SessionClosed => "SessionClosed",
        }
    }

    /// Whether the session must be discarded after this error.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            WineError::Transport(_)
                | WineError::Protocol(_)
                | WineError::VersionMismatch { .. }
                | WineError::HelperCrashed(_)
                | WineError::Timeout { .. }
                | WineError::SessionClosed
        )
    }
}

impl From<ProtocolError> for WineError {
    fn from(e: ProtocolError) -> Self {
        match e {
            ProtocolError::Truncated => {
                WineError::HelperCrashed("stream ended inside a frame".into())
            }
            other => WineError::Protocol(other.to_string()),
        }
    }
}

impl From<HandleError> for WineError {
    fn from(e: HandleError) -> Self {
        WineError::InvalidHandle(e.to_string())
    }
}

impl From<RemoteError> for WineError {
    fn from(e: RemoteError) -> Self {
        let detail = if e.code != 0 {
            format!("{} (code {})", e.message, e.code)
        } else {
            e.message
        };
        match e.kind {
            RemoteErrorKind::ProtocolViolation => WineError::Protocol(format!("helper: {detail}")),
            RemoteErrorKind::LibraryNotFound => WineError::LibraryNotFound(detail),
            RemoteErrorKind::SymbolNotFound => WineError::SymbolNotFound(detail),
            RemoteErrorKind::CallFault => WineError::CallFault(detail),
            RemoteErrorKind::PermissionDenied => WineError::PermissionDenied(detail),
            RemoteErrorKind::Unsupported => WineError::Unsupported(detail),
            RemoteErrorKind::InvalidHandle => WineError::InvalidHandle(detail),
            RemoteErrorKind::VersionMismatch => WineError::VersionMismatch {
                client: crate::protocol::PROTOCOL_VERSION,
                helper: e.code as u16,
            },
        }
    }
}
