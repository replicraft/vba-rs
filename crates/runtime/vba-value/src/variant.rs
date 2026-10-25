//! The `Variant` value and `VarType` numbering (MS-VBAL §2.2, §6.1.2.5.2).
//!
//! Objects and user-defined types are opaque handles: the VM or host decides
//! what they refer to. This crate never dereferences them.

use crate::array::VbaArray;
use crate::currency::Currency;
use crate::date::Date;
use crate::decimal::Decimal;
use crate::string::VbaString;
use vba_core::VbaError;

/// `VT_ARRAY` flag added to the element type by `VarType` for arrays.
pub const VT_ARRAY: i32 = 0x2000;
/// `VT_BYREF` flag of a by-reference variant (not reported by `VarType`; kept
/// so the VM can model OLE variant headers).
pub const VT_BYREF: i32 = 0x4000;

/// VBA `VarType` constants (and declared element types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarType {
    /// `vbEmpty` (0).
    Empty,
    /// `vbNull` (1).
    Null,
    /// `vbInteger` (2).
    Integer,
    /// `vbLong` (3).
    Long,
    /// `vbSingle` (4).
    Single,
    /// `vbDouble` (5).
    Double,
    /// `vbCurrency` (6).
    Currency,
    /// `vbDate` (7).
    Date,
    /// `vbString` (8).
    String,
    /// `vbObject` (9).
    Object,
    /// `vbError` (10).
    Error,
    /// `vbBoolean` (11).
    Boolean,
    /// `vbVariant` (12): only as the element type of a Variant array.
    Variant,
    /// `vbDataObject` (13).
    DataObject,
    /// `vbDecimal` (14).
    Decimal,
    /// `vbByte` (17).
    Byte,
    /// `vbLongLong` (20).
    LongLong,
    /// `vbUserDefinedType` (36).
    UserDefinedType,
}

impl VarType {
    /// The numeric `VarType` code.
    pub fn code(self) -> i32 {
        match self {
            VarType::Empty => 0,
            VarType::Null => 1,
            VarType::Integer => 2,
            VarType::Long => 3,
            VarType::Single => 4,
            VarType::Double => 5,
            VarType::Currency => 6,
            VarType::Date => 7,
            VarType::String => 8,
            VarType::Object => 9,
            VarType::Error => 10,
            VarType::Boolean => 11,
            VarType::Variant => 12,
            VarType::DataObject => 13,
            VarType::Decimal => 14,
            VarType::Byte => 17,
            VarType::LongLong => 20,
            VarType::UserDefinedType => 36,
        }
    }

    /// Looks up a type by its code (flags must already be stripped).
    pub fn from_code(code: i32) -> Option<Self> {
        Some(match code {
            0 => VarType::Empty,
            1 => VarType::Null,
            2 => VarType::Integer,
            3 => VarType::Long,
            4 => VarType::Single,
            5 => VarType::Double,
            6 => VarType::Currency,
            7 => VarType::Date,
            8 => VarType::String,
            9 => VarType::Object,
            10 => VarType::Error,
            11 => VarType::Boolean,
            12 => VarType::Variant,
            13 => VarType::DataObject,
            14 => VarType::Decimal,
            17 => VarType::Byte,
            20 => VarType::LongLong,
            36 => VarType::UserDefinedType,
            _ => return None,
        })
    }

    /// The initial value of a variable or array element of this type.
    ///
    /// `UserDefinedType` elements have no value this crate can construct; they
    /// start as `Empty` and the VM is expected to materialize them.
    pub fn default_value(self) -> Variant {
        match self {
            VarType::Integer => Variant::Integer(0),
            VarType::Long => Variant::Long(0),
            VarType::Single => Variant::Single(0.0),
            VarType::Double => Variant::Double(0.0),
            VarType::Currency => Variant::Currency(Currency::ZERO),
            VarType::Date => Variant::Date(Date::default()),
            VarType::String => Variant::String(VbaString::new()),
            VarType::Object | VarType::DataObject => Variant::Object(ObjectRef::Nothing),
            VarType::Boolean => Variant::Boolean(false),
            VarType::Decimal => Variant::Decimal(Decimal::ZERO),
            VarType::Byte => Variant::Byte(0),
            VarType::LongLong => Variant::LongLong(0),
            VarType::Null => Variant::Null,
            VarType::Error => Variant::Error(VbaError::new(0)),
            VarType::Empty | VarType::Variant | VarType::UserDefinedType => Variant::Empty,
        }
    }
}

/// Opaque identifier of an object owned by the VM or host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectHandle(pub u64);

/// Opaque identifier of a user-defined-type value owned by the VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UdtHandle(pub u64);

/// An object reference: `Nothing` or a handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectRef {
    /// `Nothing`.
    Nothing,
    /// A live object.
    Handle(ObjectHandle),
}

/// Pointer width chosen by the caller for `LongPtr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerWidth {
    /// 32-bit host: `LongPtr` is `Long`.
    Bits32,
    /// 64-bit host: `LongPtr` is `LongLong`.
    Bits64,
}

/// A VBA `Variant`.
#[derive(Debug, Clone, PartialEq)]
pub enum Variant {
    /// Uninitialized (`vbEmpty`).
    Empty,
    /// `Null`.
    Null,
    /// `Boolean`.
    Boolean(bool),
    /// `Byte`.
    Byte(u8),
    /// `Integer`.
    Integer(i16),
    /// `Long`.
    Long(i32),
    /// `LongLong`.
    LongLong(i64),
    /// `Single`.
    Single(f32),
    /// `Double`.
    Double(f64),
    /// `Currency`.
    Currency(Currency),
    /// `Date`.
    Date(Date),
    /// `Decimal`.
    Decimal(Decimal),
    /// `String`.
    String(VbaString),
    /// A `vbError` value (`CVErr`).
    Error(VbaError),
    /// An object reference.
    Object(ObjectRef),
    /// A user-defined type value.
    UserDefined(UdtHandle),
    /// An array.
    Array(Box<VbaArray>),
}

impl Variant {
    /// A `LongPtr` value: `Long` on 32-bit hosts, `LongLong` on 64-bit hosts.
    /// A value that does not fit a 32-bit `Long` is error 6.
    pub fn from_long_ptr(v: i64, width: PointerWidth) -> Result<Self, VbaError> {
        match width {
            PointerWidth::Bits64 => Ok(Variant::LongLong(v)),
            PointerWidth::Bits32 => i32::try_from(v)
                .map(Variant::Long)
                .map_err(|_| VbaError::OVERFLOW),
        }
    }

    /// Creates a String variant from Rust text.
    pub fn from_text(s: &str) -> Self {
        Variant::String(VbaString::from(s))
    }

    /// The subtype as `VarType`. For arrays this is the element type; use
    /// [`Variant::vartype_code`] for the flagged value.
    pub fn vartype(&self) -> VarType {
        match self {
            Variant::Empty => VarType::Empty,
            Variant::Null => VarType::Null,
            Variant::Boolean(_) => VarType::Boolean,
            Variant::Byte(_) => VarType::Byte,
            Variant::Integer(_) => VarType::Integer,
            Variant::Long(_) => VarType::Long,
            Variant::LongLong(_) => VarType::LongLong,
            Variant::Single(_) => VarType::Single,
            Variant::Double(_) => VarType::Double,
            Variant::Currency(_) => VarType::Currency,
            Variant::Date(_) => VarType::Date,
            Variant::Decimal(_) => VarType::Decimal,
            Variant::String(_) => VarType::String,
            Variant::Error(_) => VarType::Error,
            Variant::Object(_) => VarType::Object,
            Variant::UserDefined(_) => VarType::UserDefinedType,
            Variant::Array(a) => a.element_type(),
        }
    }

    /// The value `VarType(v)` returns: the subtype code, or `VT_ARRAY` plus
    /// the element type code for arrays (8204 for `Variant()` arrays).
    pub fn vartype_code(&self) -> i32 {
        match self {
            Variant::Array(a) => VT_ARRAY | a.element_type().code(),
            other => other.vartype().code(),
        }
    }

    /// Like [`Variant::vartype_code`] with the `VT_BYREF` flag set when the
    /// variant models a by-reference variant.
    pub fn vartype_code_with_flags(&self, by_ref: bool) -> i32 {
        self.vartype_code() | if by_ref { VT_BYREF } else { 0 }
    }

    /// `true` for `Empty`.
    pub fn is_empty(&self) -> bool {
        matches!(self, Variant::Empty)
    }

    /// `true` for `Null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Variant::Null)
    }

    /// `true` for subtypes `IsNumeric` accepts without parsing text: Boolean,
    /// Byte, Integer, Long, LongLong, Single, Double, Currency, Decimal and
    /// Date is NOT included (Date is not numeric for arithmetic purposes
    /// here, matching `IsNumeric(Date)` = False).
    pub fn is_numeric_subtype(&self) -> bool {
        matches!(
            self,
            Variant::Boolean(_)
                | Variant::Byte(_)
                | Variant::Integer(_)
                | Variant::Long(_)
                | Variant::LongLong(_)
                | Variant::Single(_)
                | Variant::Double(_)
                | Variant::Currency(_)
                | Variant::Decimal(_)
        )
    }
}

impl From<bool> for Variant {
    fn from(v: bool) -> Self {
        Variant::Boolean(v)
    }
}
impl From<u8> for Variant {
    fn from(v: u8) -> Self {
        Variant::Byte(v)
    }
}
impl From<i16> for Variant {
    fn from(v: i16) -> Self {
        Variant::Integer(v)
    }
}
impl From<i32> for Variant {
    fn from(v: i32) -> Self {
        Variant::Long(v)
    }
}
impl From<i64> for Variant {
    fn from(v: i64) -> Self {
        Variant::LongLong(v)
    }
}
impl From<f32> for Variant {
    fn from(v: f32) -> Self {
        Variant::Single(v)
    }
}
impl From<f64> for Variant {
    fn from(v: f64) -> Self {
        Variant::Double(v)
    }
}
impl From<&str> for Variant {
    fn from(v: &str) -> Self {
        Variant::from_text(v)
    }
}
impl From<VbaString> for Variant {
    fn from(v: VbaString) -> Self {
        Variant::String(v)
    }
}
impl From<Currency> for Variant {
    fn from(v: Currency) -> Self {
        Variant::Currency(v)
    }
}
impl From<Decimal> for Variant {
    fn from(v: Decimal) -> Self {
        Variant::Decimal(v)
    }
}
impl From<Date> for Variant {
    fn from(v: Date) -> Self {
        Variant::Date(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vartype_codes() {
        let cases: Vec<(Variant, i32)> = vec![
            (Variant::Empty, 0),
            (Variant::Null, 1),
            (Variant::Integer(0), 2),
            (Variant::Long(0), 3),
            (Variant::Single(0.0), 4),
            (Variant::Double(0.0), 5),
            (Variant::Currency(Currency::ZERO), 6),
            (Variant::Date(Date::default()), 7),
            (Variant::from("x"), 8),
            (Variant::Object(ObjectRef::Nothing), 9),
            (Variant::Error(VbaError::new(5)), 10),
            (Variant::Boolean(true), 11),
            (Variant::Decimal(Decimal::ZERO), 14),
            (Variant::Byte(1), 17),
            (Variant::LongLong(1), 20),
            (Variant::UserDefined(UdtHandle(1)), 36),
        ];
        for (v, c) in cases {
            assert_eq!(v.vartype_code(), c, "{v:?}");
            assert_eq!(VarType::from_code(c).unwrap().code(), c);
        }
        assert_eq!(VarType::from_code(15), None);
        assert_eq!(VarType::from_code(8192), None);
    }

    #[test]
    fn long_ptr_width() {
        assert_eq!(
            Variant::from_long_ptr(5, PointerWidth::Bits32),
            Ok(Variant::Long(5))
        );
        assert_eq!(
            Variant::from_long_ptr(1 << 40, PointerWidth::Bits32),
            Err(VbaError::OVERFLOW)
        );
        assert_eq!(
            Variant::from_long_ptr(1 << 40, PointerWidth::Bits64),
            Ok(Variant::LongLong(1 << 40))
        );
    }

    #[test]
    fn defaults() {
        assert_eq!(VarType::String.default_value(), Variant::from(""));
        assert_eq!(VarType::Variant.default_value(), Variant::Empty);
        assert_eq!(
            VarType::Object.default_value(),
            Variant::Object(ObjectRef::Nothing)
        );
        assert_eq!(VarType::Boolean.default_value(), Variant::Boolean(false));
    }

    #[test]
    fn byref_flag() {
        assert_eq!(Variant::Long(1).vartype_code_with_flags(true), 0x4003);
    }
}
