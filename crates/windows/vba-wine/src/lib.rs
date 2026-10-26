//! Native-side client of the Wine bridge.
//!
//! * [`protocol`]: the versioned, length-prefixed wire protocol shared with the
//!   helper executable (`vba-wine-host`);
//! * [`helper`]: the helper-side dispatcher ([`helper::Server`] over a
//!   [`helper::Backend`]);
//! * [`config`] / [`transport`] / [`session`]: explicit Wine discovery, process
//!   spawn with timeouts and kill-on-drop, one-request-at-a-time sessions;
//! * [`provider`]: [`WineProvider`], an opt-in `vba-winapi` provider gated on
//!   the `NativeDeclare` capability.
//!
//! Nothing starts automatically; Wine is not a sandbox. See `docs/interop/wine-bridge.md`.

pub mod config;
pub mod error;
pub mod helper;
pub mod protocol;
pub mod provider;
pub mod session;
pub mod transport;

pub use config::{Launch, WineConfig, find_wine_in_env, find_wine_in_path};
pub use error::WineError;
pub use provider::{ParamKind, WineExport, WineProvider};
pub use session::{CallReply, DEFAULT_PROVIDER, Session, SessionOptions};
pub use transport::{ChannelPeer, ChannelTransport, ProcessTransport, Transport};
