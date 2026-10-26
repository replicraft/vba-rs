//! Win32 behavior catalog and provider framework for VBA `Declare`.
//!
//! * [`catalog`]: the machine-readable catalog of Win32 functions, their
//!   preferred strategy and implementation status;
//! * [`inventory`]: the read-only inventory of every Win32 export known to
//!   Microsoft's metadata (generated, never hand-edited);
//! * [`Provider`] / [`Registry`]: resolution of a `Declare` (library, alias)
//!   to a provider, with explicit `Unsupported` errors for everything else;
//! * [`shims::NativeShims`]: portable shims whose behavior is identical on
//!   every host OS, driven by an injected [`CallContext`].
//!
//! A function that is not implemented never succeeds: it fails with
//! [`WinApiError::Unsupported`] naming the symbol.

pub mod callback;
pub mod catalog;
pub mod context;
pub mod error;
pub mod inventory;
pub mod provider;
pub mod shims;

pub use callback::{
    CallbackError, CallbackHost, CallbackSig, CbRet, CbSlot, QSORT_COMPARE, TIMERPROC, Timer,
    TimerTable, WNDENUMPROC, Window, WindowList, dispatch_timers,
};
pub use context::{CallContext, HostEnv, Identity, ManualClock, SystemClock, WindowsVersion};
pub use error::WinApiError;
pub use provider::{Provider, Registry, Resolved, normalize_library};
pub use shims::{NATIVE_SHIMS, NativeShims};
