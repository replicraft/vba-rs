//! Moving values between Rust and VBA: constructors, conversions with
//! `From` and `TryFrom`, arrays, dates and a `Display` for logs.
//!
//! Conversions *to* a `Variant` never fail and keep the natural subtype
//! (`i32` is `Long`, `f64` is `Double`, `&str` is `String`, `bool` is
//! `Boolean`). Conversions *from* a `Variant` follow the VBA conversion
//! functions (`CInt`, `CLng`, `CDbl`, `CBool`, `CStr`...): `"12"` becomes
//! 12, a `Boolean` is -1 or 0 as a number, `Empty` is 0 or `""`, `Null` is
//! error 94, an overflow is error 6, text that is not a number is error 13.
//!
//! ```
//! use vba_value::Variant;
//!
//! let v = Variant::from(41_i32);
//! assert_eq!(i64::try_from(&v), Ok(41));
//! assert_eq!(String::try_from(Variant::from(2.5_f64)), Ok("2.5".to_string()));
//! assert!(i16::try_from(&Variant::from(70_000_i32)).is_err());
//!
//! // Arrays: zero-based like `Array(...)`, or any lower bound.
//! let a = Variant::from(vec![1_i32, 2, 3]);
//! assert_eq!(a.array_bounds(), Some((0, 2)));
//! assert_eq!(Variant::array_with_lower_bound(vec![Variant::from("x")], 1).array_bounds(), Some((1, 1)));
//! assert_eq!(a.to_vec().unwrap()[2], Variant::Long(3));
//! ```

use std::fmt;

use vba_core::{VbaError, VbaResult};

use crate::array::{Bounds, VbaArray};
use crate::convert;
use crate::currency::Currency;
use crate::date::Date;
use crate::decimal::Decimal;
use crate::variant::{ObjectRef, VarType, Variant};

impl Variant {
    /// `Empty`: an uninitialised `Variant`.
    pub const fn empty() -> Variant {
        Variant::Empty
    }

    /// `Null`: no valid data (a database field without a value).
    pub const fn null() -> Variant {
        Variant::Null
    }

    /// `Nothing`: an object variable that refers to no object.
    pub const fn nothing() -> Variant {
        Variant::Object(ObjectRef::Nothing)
    }

    /// Whether this is an object reference that refers to nothing.
    pub fn is_nothing(&self) -> bool {
        matches!(self, Variant::Object(ObjectRef::Nothing))
    }

    /// A `Date` from an OLE Automation serial number (days since
    /// 1899-12-30; the fraction is the time of day). Out-of-range or
    /// non-finite serials are error 6.
    pub fn date_from_serial(serial: f64) -> VbaResult<Variant> {
        Ok(Variant::Date(Date::from_serial(serial)?))
    }

    /// A `Date` from calendar fields (proleptic Gregorian, year 100..=9999);
    /// an invalid field is error 5.
    pub fn date_from_ymd_hms(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> VbaResult<Variant> {
        Ok(Variant::Date(Date::from_ymd_hms(
            year, month, day, hour, minute, second,
        )?))
    }

    /// A one-dimensional `Variant()` array whose first index is `lower`
    /// (0 like `Array(...)`, 1 under `Option Base 1`, or any other).
    pub fn array_with_lower_bound(values: Vec<Variant>, lower: i32) -> Variant {
        if values.is_empty() {
            return Variant::Array(Box::new(VbaArray::new_empty(VarType::Variant, lower)));
        }
        let upper = lower.saturating_add(values.len().min(i32::MAX as usize) as i32 - 1);
        let mut a = VbaArray::new_dynamic(VarType::Variant);
        a.redim(&[Bounds::new(lower, upper)], false)
            .expect("a Vec fits an array");
        for (i, v) in values.into_iter().enumerate() {
            a.set(&[lower + i as i32], v).expect("index in range");
        }
        Variant::Array(Box::new(a))
    }

    /// A zero-based `Variant()` array, like `Array(...)`.
    pub fn array_zero_based(values: Vec<Variant>) -> Variant {
        Variant::array_with_lower_bound(values, 0)
    }

    /// A one-based `Variant()` array.
    pub fn array_one_based(values: Vec<Variant>) -> Variant {
        Variant::array_with_lower_bound(values, 1)
    }

    /// The `(LBound, UBound)` of a one-dimensional array; `None` for other
    /// values, other dimensionalities and unallocated arrays.
    pub fn array_bounds(&self) -> Option<(i32, i32)> {
        match self {
            Variant::Array(a) if a.dimensions() == 1 && a.is_allocated() => {
                Some((a.lbound(1).ok()?, a.ubound(1).ok()?))
            }
            _ => None,
        }
    }

    /// The elements of a one-dimensional array in index order, whatever its
    /// lower bound; `None` for anything else.
    pub fn to_vec(&self) -> Option<Vec<Variant>> {
        match self {
            Variant::Array(a) if a.dimensions() == 1 => Some(a.iter().cloned().collect()),
            _ => None,
        }
    }
}

macro_rules! from_primitive {
    ($($t:ty => $variant:ident as $via:ty),* $(,)?) => {$(
        impl From<$t> for Variant {
            fn from(v: $t) -> Variant {
                Variant::$variant(<$via>::from(v))
            }
        }
    )*};
}

from_primitive! {
    u16 => Long as i32,
    u32 => LongLong as i64,
    i8 => Integer as i16,
}

impl From<String> for Variant {
    fn from(v: String) -> Variant {
        Variant::from_text(&v)
    }
}

impl From<&String> for Variant {
    fn from(v: &String) -> Variant {
        Variant::from_text(v)
    }
}

impl From<VbaError> for Variant {
    /// A `vbError` value, as `CVErr(n)` makes.
    fn from(v: VbaError) -> Variant {
        Variant::Error(v)
    }
}

impl From<ObjectRef> for Variant {
    fn from(v: ObjectRef) -> Variant {
        Variant::Object(v)
    }
}

impl<T: Into<Variant>> From<Option<T>> for Variant {
    /// `None` is `Null` (no value, as in a database field), not `Empty`.
    fn from(v: Option<T>) -> Variant {
        match v {
            Some(v) => v.into(),
            None => Variant::Null,
        }
    }
}

impl<T: Into<Variant>> From<Vec<T>> for Variant {
    /// A zero-based `Variant()` array, like `Array(...)`.
    fn from(v: Vec<T>) -> Variant {
        Variant::array_zero_based(v.into_iter().map(Into::into).collect())
    }
}

macro_rules! try_from_variant {
    ($($t:ty => $conv:path),* $(,)?) => {$(
        impl TryFrom<&Variant> for $t {
            type Error = VbaError;
            fn try_from(v: &Variant) -> Result<$t, VbaError> {
                $conv(v)
            }
        }
        impl TryFrom<Variant> for $t {
            type Error = VbaError;
            fn try_from(v: Variant) -> Result<$t, VbaError> {
                $conv(&v)
            }
        }
    )*};
}

try_from_variant! {
    bool => convert::to_boolean,
    u8 => convert::to_byte,
    i16 => convert::to_integer,
    i32 => convert::to_long,
    i64 => convert::to_longlong,
    f32 => convert::to_single,
    f64 => convert::to_double,
    Currency => convert::to_currency,
    Decimal => convert::to_decimal,
    Date => convert::to_date,
}

impl TryFrom<&Variant> for String {
    type Error = VbaError;
    /// Like `CStr`; unpaired UTF-16 surrogates become U+FFFD.
    fn try_from(v: &Variant) -> Result<String, VbaError> {
        Ok(convert::to_vba_string(v)?.to_string_lossy())
    }
}

impl TryFrom<Variant> for String {
    type Error = VbaError;
    fn try_from(v: Variant) -> Result<String, VbaError> {
        String::try_from(&v)
    }
}

impl TryFrom<&Variant> for Vec<Variant> {
    type Error = VbaError;
    /// The elements of a one-dimensional array; anything else is error 13.
    fn try_from(v: &Variant) -> Result<Vec<Variant>, VbaError> {
        v.to_vec().ok_or(VbaError::TYPE_MISMATCH)
    }
}

impl TryFrom<Variant> for Vec<Variant> {
    type Error = VbaError;
    fn try_from(v: Variant) -> Result<Vec<Variant>, VbaError> {
        Vec::<Variant>::try_from(&v)
    }
}

impl TryFrom<&Variant> for ObjectRef {
    type Error = VbaError;
    /// An object reference (`Nothing` included); anything else is error 13.
    fn try_from(v: &Variant) -> Result<ObjectRef, VbaError> {
        match v {
            Variant::Object(o) => Ok(*o),
            _ => Err(VbaError::TYPE_MISMATCH),
        }
    }
}

impl TryFrom<Variant> for ObjectRef {
    type Error = VbaError;
    fn try_from(v: Variant) -> Result<ObjectRef, VbaError> {
        ObjectRef::try_from(&v)
    }
}

impl Date {
    /// Seconds between 1970-01-01 00:00:00 and 1899-12-30 00:00:00.
    const UNIX_OFFSET_SECS: i64 = 25_569 * 86_400;

    /// A date from seconds since the Unix epoch (UTC, no leap seconds); no
    /// time-zone handling. Out of range is error 6.
    pub fn from_unix_seconds(secs: i64) -> VbaResult<Date> {
        let total = secs
            .checked_add(Self::UNIX_OFFSET_SECS)
            .ok_or(VbaError::OVERFLOW)?;
        Date::from_serial(total as f64 / 86_400.0)
    }

    /// Seconds since the Unix epoch (rounded to the second); no time-zone
    /// handling.
    pub fn unix_seconds(self) -> i64 {
        (self.serial() * 86_400.0).round() as i64 - Self::UNIX_OFFSET_SECS
    }
}

impl fmt::Display for Variant {
    /// A debugging form: strings are quoted, `Empty` and `Null` are named,
    /// dates and numbers print as `CStr` does, an array lists its elements
    /// with its bounds, objects print their handle.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Variant::Empty => f.write_str("Empty"),
            Variant::Null => f.write_str("Null"),
            Variant::Boolean(b) => f.write_str(if *b { "True" } else { "False" }),
            Variant::Byte(v) => write!(f, "{v}"),
            Variant::Integer(v) => write!(f, "{v}"),
            Variant::Long(v) => write!(f, "{v}"),
            Variant::LongLong(v) => write!(f, "{v}"),
            Variant::Single(_)
            | Variant::Double(_)
            | Variant::Currency(_)
            | Variant::Decimal(_) => match convert::to_vba_string(self) {
                Ok(s) => write!(f, "{s}"),
                Err(_) => f.write_str("?"),
            },
            Variant::Date(d) => write!(f, "#{d}#"),
            Variant::String(s) => write!(f, "\"{s}\""),
            Variant::Error(e) => write!(f, "Error {}", e.number()),
            Variant::Object(ObjectRef::Nothing) => f.write_str("Nothing"),
            Variant::Object(ObjectRef::Handle(h)) => write!(f, "<object {}>", h.0),
            Variant::UserDefined(h) => write!(f, "<type value {}>", h.0),
            Variant::Array(a) => {
                if !a.is_allocated() {
                    return f.write_str("{unallocated array}");
                }
                if a.dimensions() == 1 {
                    let (lo, hi) = (a.lbound(1).unwrap_or(0), a.ubound(1).unwrap_or(-1));
                    write!(f, "({lo} To {hi}){{")?;
                } else {
                    write!(f, "({} dimensions){{", a.dimensions())?;
                }
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_str("}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_keep_their_subtype() {
        assert_eq!(Variant::from(true), Variant::Boolean(true));
        assert_eq!(Variant::from(7_u8), Variant::Byte(7));
        assert_eq!(Variant::from(-3_i8), Variant::Integer(-3));
        assert_eq!(Variant::from(7_u16), Variant::Long(7));
        assert_eq!(Variant::from(7_i32), Variant::Long(7));
        assert_eq!(Variant::from(7_u32), Variant::LongLong(7));
        assert_eq!(Variant::from(7_i64), Variant::LongLong(7));
        assert_eq!(Variant::from(1.5_f32), Variant::Single(1.5));
        assert_eq!(Variant::from(1.5_f64), Variant::Double(1.5));
        assert_eq!(Variant::from("a"), Variant::from_text("a"));
        assert_eq!(Variant::from(String::from("a")), Variant::from_text("a"));
        assert_eq!(Variant::from(Some(3_i32)), Variant::Long(3));
        assert_eq!(Variant::from(None::<i32>), Variant::Null);
        assert!(Variant::nothing().is_nothing());
        assert!(Variant::empty().is_empty() && Variant::null().is_null());
        assert_eq!(
            Variant::from(VbaError::new(2042)),
            Variant::Error(VbaError::new(2042))
        );
    }

    #[test]
    fn conversions_back_follow_the_vba_functions() {
        assert_eq!(i32::try_from(Variant::from("12")), Ok(12));
        assert_eq!(i32::try_from(&Variant::from(true)), Ok(-1));
        assert_eq!(bool::try_from(&Variant::from(0_i32)), Ok(false));
        assert_eq!(bool::try_from(&Variant::from("True")), Ok(true));
        assert_eq!(f64::try_from(&Variant::from(3_i32)), Ok(3.0));
        assert_eq!(i16::try_from(&Variant::from(2.5_f64)), Ok(2)); // banker's rounding
        assert_eq!(i16::try_from(&Variant::from(3.5_f64)), Ok(4));
        assert_eq!(u8::try_from(&Variant::from(255_i32)), Ok(255));
        assert_eq!(
            i16::try_from(&Variant::from(70_000_i32)),
            Err(VbaError::OVERFLOW)
        );
        assert_eq!(
            i32::try_from(&Variant::from("abc")),
            Err(VbaError::TYPE_MISMATCH)
        );
        assert_eq!(i32::try_from(&Variant::Empty), Ok(0));
        assert_eq!(String::try_from(&Variant::Empty), Ok(String::new()));
        assert_eq!(String::try_from(&Variant::from(12_i32)), Ok("12".into()));
        assert!(i32::try_from(&Variant::Null).is_err());
        assert!(String::try_from(&Variant::Null).is_err());
    }

    #[test]
    fn arrays_round_trip_with_any_lower_bound() {
        let a = Variant::from(vec![1_i32, 2, 3]);
        assert_eq!(a.array_bounds(), Some((0, 2)));
        assert_eq!(
            Vec::<Variant>::try_from(&a).unwrap(),
            vec![Variant::Long(1), Variant::Long(2), Variant::Long(3)]
        );
        let b = Variant::array_one_based(vec![Variant::from("x"), Variant::from("y")]);
        assert_eq!(b.array_bounds(), Some((1, 2)));
        assert_eq!(b.to_vec().unwrap()[1], Variant::from("y"));
        let c = Variant::array_with_lower_bound(vec![Variant::Null], -5);
        assert_eq!(c.array_bounds(), Some((-5, -5)));
        let empty = Variant::array_zero_based(vec![]);
        assert_eq!(empty.to_vec().unwrap(), Vec::<Variant>::new());
        assert_eq!(Variant::from(1_i32).to_vec(), None);
        assert_eq!(
            Vec::<Variant>::try_from(&Variant::from(1_i32)),
            Err(VbaError::TYPE_MISMATCH)
        );
    }

    #[test]
    fn dates_without_a_calendar_crate() {
        let d = Variant::date_from_ymd_hms(2024, 2, 29, 13, 30, 15).unwrap();
        let Variant::Date(date) = d.clone() else {
            panic!("a date")
        };
        let p = date.parts();
        assert_eq!(
            (p.year, p.month, p.day, p.hour, p.minute, p.second),
            (2024, 2, 29, 13, 30, 15)
        );
        assert!(Variant::date_from_ymd_hms(2023, 2, 29, 0, 0, 0).is_err());
        assert_eq!(
            Variant::date_from_serial(0.0).unwrap().to_string(),
            "#12:00:00 AM#"
        );
        assert!(Variant::date_from_serial(f64::NAN).is_err());
        // Unix epoch.
        let epoch = Date::from_unix_seconds(0).unwrap();
        assert_eq!(epoch.serial(), 25_569.0);
        assert_eq!(epoch.parts().year, 1970);
        assert_eq!(date.unix_seconds(), 1_709_213_415);
        assert_eq!(Date::from_unix_seconds(1_709_213_415).unwrap(), date);
        assert_eq!(Date::try_from(&d), Ok(date));
    }

    #[test]
    fn display_is_for_logs() {
        let a = Variant::array_one_based(vec![
            Variant::from("x"),
            Variant::Null,
            Variant::Boolean(true),
            Variant::Long(3),
        ]);
        assert_eq!(a.to_string(), "(1 To 4){\"x\", Null, True, 3}");
        assert_eq!(Variant::Empty.to_string(), "Empty");
        assert_eq!(Variant::nothing().to_string(), "Nothing");
        assert_eq!(Variant::from(2.5_f64).to_string(), "2.5");
        assert_eq!(
            Variant::Error(VbaError::new(2007)).to_string(),
            "Error 2007"
        );
        assert_eq!(
            Variant::date_from_ymd_hms(2020, 1, 2, 0, 0, 0)
                .unwrap()
                .to_string(),
            "#1/2/2020#"
        );
    }
}
