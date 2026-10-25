//! The dynamic native call stage.
//!
//! Loading a library and calling into it needs `unsafe` code that cannot be
//! exercised safely and portably in this repository's test environment, so
//! this stage validates the request completely and then reports
//! [`NativeCallError::Unavailable`]. It never loads anything. The `native-call`
//! cargo feature reserves the future real backend; enabling it changes only the
//! reported reason. A [`NativePermission`] issued by the host is required in
//! every configuration, and no VBA-reachable API creates one.

use crate::{NativeArg, NativeReturn, ValidatedDeclare};
use thiserror::Error;
use vba_profile::Target;

/// Why the native call stage did not run a call.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NativeCallError {
    /// The permission token does not allow this library.
    #[error("host grant does not allow native library `{library}`")]
    PermissionDenied {
        /// Requested library.
        library: String,
    },
    /// The declaration's target is not the machine's, so it cannot be run.
    #[error("declaration targets {declared:?} but the host is {host:?}")]
    TargetMismatch {
        /// Declared target.
        declared: Target,
        /// Host target.
        host: Target,
    },
    /// No native backend is available: `NativeCallUnavailable`.
    #[error("NativeCallUnavailable: {reason}")]
    Unavailable {
        /// Human-readable reason.
        reason: &'static str,
    },
}

/// A host-issued grant to call native code in specific libraries.
///
/// Only host code constructs this; the `Declare` machinery merely demands one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePermission {
    libraries: Vec<String>,
}

impl NativePermission {
    /// Grants calls into exactly these libraries (case-insensitive names).
    pub fn grant<I: IntoIterator<Item = String>>(libraries: I) -> Self {
        NativePermission {
            libraries: libraries
                .into_iter()
                .map(|l| l.to_ascii_lowercase())
                .collect(),
        }
    }

    /// Whether `library` is covered.
    pub fn allows(&self, library: &str) -> bool {
        let l = library.to_ascii_lowercase();
        self.libraries.contains(&l)
    }
}

/// Attempts a native call. Always an error in this build: see module docs.
pub fn call_native(
    permission: &NativePermission,
    decl: &ValidatedDeclare,
    host: Target,
    _args: &[NativeArg],
) -> Result<NativeReturn, NativeCallError> {
    let library = &decl.spec().library;
    if !permission.allows(library) {
        return Err(NativeCallError::PermissionDenied {
            library: library.clone(),
        });
    }
    if decl.target() != host {
        return Err(NativeCallError::TargetMismatch {
            declared: decl.target(),
            host,
        });
    }
    Err(NativeCallError::Unavailable {
        reason: if cfg!(feature = "native-call") {
            "the native-call backend is reserved but not implemented"
        } else {
            "built without the `native-call` feature"
        },
    })
}
