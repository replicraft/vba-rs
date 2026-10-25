//! Exact VBA value system.
//!
//! Strings are UTF-16 code unit sequences, scalars follow the VBA data types
//! (`Currency` as a scaled `i64`, `Decimal` with a 96-bit mantissa, `Date` as
//! an OLE automation `f64`), and `Variant` carries every subtype. This crate
//! depends on no other vba-rs crate and no host.

pub mod array;
pub mod convert;
pub mod currency;
pub mod date;
pub mod decimal;
pub mod error;
pub mod string;
pub mod text;
pub mod variant;

pub use array::{Bounds, VbaArray};
pub use currency::Currency;
pub use date::Date;
pub use decimal::Decimal;
pub use error::{VbaError, VbaResult};
pub use string::{CompareMode, VbaString};
pub use variant::{ObjectHandle, ObjectRef, PointerWidth, UdtHandle, VarType, Variant};
