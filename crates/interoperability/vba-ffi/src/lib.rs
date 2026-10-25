//! The VBA `Declare` ABI layer.
//!
//! This crate models everything that happens between a VBA `Declare`
//! statement and a native entry point, without ever touching the host:
//!
//! * [`signature`]: a validated descriptor of the declaration (MS-VBAL §5.2.3.5)
//!   whose 32/64-bit semantics come from the `vba-profile` target, never from
//!   the machine running vba-rs;
//! * [`layout`]: user-defined type layout and byte encoding;
//! * [`codec`]: exact ANSI/UTF-8 code page conversion;
//! * [`memory`]: a bounds-checked, VBA-visible address space (no host pointers);
//! * [`marshal`]: the argument marshaling plan (prepare before, write back after);
//! * [`handle`]: opaque, provider-affine handles;
//! * [`last_error`]: emulated per-logical-thread `GetLastError`;
//! * [`native`]: the permission-gated, feature-gated dynamic call stage.

pub mod codec;
pub mod error;
pub mod handle;
pub mod last_error;
pub mod layout;
pub mod marshal;
pub mod memory;
pub mod native;
pub mod signature;

pub use codec::{CodecError, Codepage, Encoded, Unmappable};
pub use error::FfiError;
pub use handle::{Handle, HandleError, HandleTable, ProviderId};
pub use last_error::{LastErrorStore, LogicalThread};
pub use layout::{FieldValue, UdtFieldType, UdtLayout};
pub use marshal::{ArgValue, CallOutcome, NativeArg, NativeReturn, PreparedCall};
pub use memory::{Memory, MemoryError};
pub use native::{NativeCallError, NativePermission, call_native};
pub use signature::{
    CallConv, Charset, DeclareSpec, EntryCandidate, EntryName, ParamKind, ParamSpec, Passing,
    ValidatedDeclare,
};
