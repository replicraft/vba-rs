//! The contract between vba-rs and the application that embeds it.
//!
//! An embedder implements [`Host`]: it names itself ([`HostMetadata`], linked
//! to a `vba-profile` profile), resolves global names such as `Application`,
//! creates objects for `CreateObject`/`GetObject`, routes `Declare` calls,
//! and supplies UI and event delivery. Everything the VBA program could use
//! to reach outside the interpreter is gated by a deny-by-default
//! [`Policy`]; the provided `Host` methods check it before calling the
//! embedder's hook, so a hook can never be reached without a grant. Resource
//! limits are plain data ([`ResourceLimits`]) with a small [`Budget`] meter.
//!
//! The crate knows no host application: no embedding-application, Access or Office
//! types. [`HeadlessHost`] is a complete host that grants nothing and fails
//! explicitly; [`ScriptedHost`] is a configurable one for tests.
//!
//! ```
//! use vba_host::{Capability, Host, HeadlessHost, Policy};
//!
//! let host = HeadlessHost::new();
//! // Deny by default: creating a COM object is Permission denied (70).
//! assert_eq!(host.create_object("Scripting.Dictionary", None).unwrap_err().number, 70);
//!
//! let host = HeadlessHost::new().with_policy(Policy::deny_all().grant(Capability::ComCreate));
//! // Granted, but this host has no factories: error 429.
//! assert_eq!(host.create_object("Scripting.Dictionary", None).unwrap_err().number, 429);
//! ```

pub mod capability;
pub mod events;
pub mod headless;
pub mod host;
pub mod limits;
pub mod metadata;
pub mod scripted;
pub mod settings;
pub mod ui;

pub use capability::{Capability, PermissionDenied, Policy};
pub use events::{EventDispatch, EventRouter};
pub use headless::HeadlessHost;
pub use host::{ClockPart, DeclareRequest, Host, HostResult, ProjectReference};
pub use limits::{Budget, LimitExceeded, ResourceLimits};
pub use metadata::HostMetadata;
pub use scripted::ScriptedHost;
pub use settings::{MemorySettings, SettingsStore};
pub use ui::{HeadlessUi, InputBoxRequest, MsgBoxRequest, UiHooks};
