//! Declarative description of an embedding application's object model.
//!
//! An application describes its classes, enumerations, constants, global
//! functions and predeclared objects once, as a [`HostLibrary`]. After
//! [`HostLibrary::check`] the same [`CheckedLibrary`] serves
//!
//! * `vba_sema::TypeProvider` (early-bound typing and diagnostics, and the
//!   data a language server lists for completion, hover and signature
//!   help), implemented by `CheckedLibrary` itself;
//! * `vba_automation::Dispatch` objects whose `member_info`, argument
//!   binding and type coercion come from the same signatures
//!   ([`HostObject`], [`Handlers`]);
//! * global functions ([`HostFunctions`]);
//! * a stable JSON form ([`HostLibrary::to_json`],
//!   [`HostLibrary::from_json_str`]) for tools that cannot link the
//!   application.
//!
//! See `docs/architecture/host-model.md`.

mod binding;
mod check;
mod json;
mod model;
mod provider;
mod runtime;

pub use binding::{HostFunctions, HostRuntime};
pub use check::{
    CheckedLibrary, EventSig, MemberSig, ModelError, ModelErrors, ParamSig,
};
pub use json::{FORMAT_NAME, FORMAT_VERSION, JsonError};
pub use model::{
    Access, ConstValue, HostClass, HostConst, HostEnum, HostEvent, HostGlobal, HostLibrary,
    HostMember, HostParam, MemberKind, TypeRef,
};
pub use runtime::{Args, BindError, CallKind, Handlers, HostImpl, HostObject, Invocation};
