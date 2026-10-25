//! The VBA standard library: the intrinsic functions of `VBA.Strings`,
//! `VBA.Math`, `VBA.DateTime`, `VBA.Information`, `VBA.Conversion`,
//! `VBA.Financial` and the pure parts of `VBA.Interaction`, together with the
//! `Like` operator.
//!
//! The crate depends only on `vba-value`. Everything that is not pure (clock,
//! environment, locale) comes from a [`host::Host`]; the random generator
//! state lives in [`call::State`]. Given the same host, state and arguments
//! every call is deterministic.
//!
//! Use [`registry::lookup`] / [`registry::call`] for dynamic dispatch, or the
//! typed functions in each module directly.

pub mod call;
pub mod conversion;
pub mod datetime;
pub(crate) mod digits;
pub mod error;
pub mod financial;
pub mod format;
pub mod host;
pub mod info;
pub mod interaction;
pub mod like;
pub mod math;
pub mod registry;
pub mod strings;

pub use call::{Args, Ctx, State};
pub use error::{StdError, StdResult};
pub use host::{Clock, DateOrder, Environment, FixedHost, Host, Locale};
pub use registry::{FunctionInfo, Kind, ParamInfo, Resolved, call as call_function, lookup};
