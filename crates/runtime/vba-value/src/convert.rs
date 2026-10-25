//! Explicit and implicit conversions between subtypes (MS-VBAL §5.6.9.x
//! "Let-coercion", §6.1.2.3 conversion functions `CBool` .. `CVar`).
//!
//! Rules implemented (spec text and documented VBA behavior; see the module
//! docs of [`crate::text`] for string syntax):
//!
//! * Conversion of a floating, `Currency` or `Decimal` value to an integer
//!   subtype rounds half to even (banker's rounding), then range-checks:
//!   out of range, NaN and infinities are error 6.
//! * `Null` converts to nothing: error 94. A `vbError` value re-raises its
//!   error number. Objects, user-defined types and arrays are error 13 (the
//!   VM resolves default properties before calling into this crate).
//! * `Empty` is 0, `""` or `False`.
//! * Boolean converts to -1/0 numerically and to `"True"`/`"False"` as text.
//! * String to number: see [`crate::text::parse_numeric`]; non-numeric text is
//!   error 13, out-of-range text error 6. String to Boolean also accepts
//!   `"True"`/`"False"` (case-insensitive).
//! * `CDec` of a Double keeps 15 significant digits, of a Single 7.

use crate::currency::Currency;
use crate::date::Date;
use crate::decimal::Decimal;
use crate::string::VbaString;
use crate::text::{self, Numeric};
use crate::variant::{VarType, Variant};
use vba_core::{VbaError, VbaResult};

/// A source value reduced to a numeric form.
enum Num {
    Int(i64),
    Single(f32),
    Double(f64),
    Cur(Currency),
    Dec(Decimal),
    Str {
        value: f64,
        decimal: Option<Decimal>,
    },
}

fn to_num(v: &Variant) -> VbaResult<Num> {
    Ok(match v {
        Variant::Empty => Num::Int(0),
        Variant::Boolean(b) => Num::Int(if *b { -1 } else { 0 }),
        Variant::Byte(x) => Num::Int(i64::from(*x)),
        Variant::Integer(x) => Num::Int(i64::from(*x)),
        Variant::Long(x) => Num::Int(i64::from(*x)),
        Variant::LongLong(x) => Num::Int(*x),
        Variant::Single(x) => Num::Single(*x),
        Variant::Double(x) => Num::Double(*x),
        Variant::Currency(x) => Num::Cur(*x),
        Variant::Decimal(x) => Num::Dec(*x),
        Variant::Date(d) => Num::Double(d.serial()),
        Variant::String(s) => match text::parse_numeric(s)? {
            Numeric::Integer(i) => Num::Int(i),
            Numeric::Real { value, decimal } => Num::Str { value, decimal },
        },
        Variant::Null => return Err(VbaError::INVALID_USE_OF_NULL),
        Variant::Error(e) => return Err(*e),
        Variant::Object(_) | Variant::UserDefined(_) | Variant::Array(_) => {
            return Err(VbaError::TYPE_MISMATCH);
        }
    })
}

/// Rounds half to even and checks the `i64` range.
fn round_f64_to_i64(v: f64) -> VbaResult<i64> {
    if !v.is_finite() {
        return Err(VbaError::OVERFLOW);
    }
    let r = v.round_ties_even();
    if (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&r) {
        Ok(r as i64)
    } else {
        Err(VbaError::OVERFLOW)
    }
}

fn num_to_i64(n: &Num) -> VbaResult<i64> {
    match n {
        Num::Int(i) => Ok(*i),
        Num::Single(s) => round_f64_to_i64(f64::from(*s)),
        Num::Double(d) => round_f64_to_i64(*d),
        Num::Cur(c) => Ok(c.round_to_i64()),
        Num::Dec(d) => d.to_i64_round(),
        Num::Str { value, .. } => round_f64_to_i64(*value),
    }
}

fn num_to_f64(n: &Num) -> f64 {
    match n {
        Num::Int(i) => *i as f64,
        Num::Single(s) => f64::from(*s),
        Num::Double(d) => *d,
        Num::Cur(c) => c.to_f64(),
        Num::Dec(d) => d.to_f64(),
        Num::Str { value, .. } => *value,
    }
}

/// `CBool`.
pub fn to_boolean(v: &Variant) -> VbaResult<bool> {
    if let Variant::Boolean(b) = v {
        return Ok(*b);
    }
    if let Variant::String(s) = v {
        let l = s.to_string_lossy();
        let l = l.trim();
        if l.eq_ignore_ascii_case("true") {
            return Ok(true);
        }
        if l.eq_ignore_ascii_case("false") {
            return Ok(false);
        }
    }
    Ok(match to_num(v)? {
        Num::Int(i) => i != 0,
        Num::Single(s) => s != 0.0,
        Num::Double(d) => d != 0.0,
        Num::Cur(c) => c != Currency::ZERO,
        Num::Dec(d) => !d.is_zero(),
        Num::Str { value, .. } => value != 0.0,
    })
}

/// `CByte`.
pub fn to_byte(v: &Variant) -> VbaResult<u8> {
    u8::try_from(num_to_i64(&to_num(v)?)?).map_err(|_| VbaError::OVERFLOW)
}

/// `CInt`.
pub fn to_integer(v: &Variant) -> VbaResult<i16> {
    i16::try_from(num_to_i64(&to_num(v)?)?).map_err(|_| VbaError::OVERFLOW)
}

/// `CLng`.
pub fn to_long(v: &Variant) -> VbaResult<i32> {
    i32::try_from(num_to_i64(&to_num(v)?)?).map_err(|_| VbaError::OVERFLOW)
}

/// `CLngLng`.
pub fn to_longlong(v: &Variant) -> VbaResult<i64> {
    num_to_i64(&to_num(v)?)
}

/// `CSng`: values beyond the `Single` range are error 6.
pub fn to_single(v: &Variant) -> VbaResult<f32> {
    let n = to_num(v)?;
    if let Num::Single(s) = n {
        return Ok(s);
    }
    let d = num_to_f64(&n);
    let s = d as f32;
    if !d.is_finite() || s.is_infinite() {
        return Err(VbaError::OVERFLOW);
    }
    Ok(s)
}

/// `CDbl`.
pub fn to_double(v: &Variant) -> VbaResult<f64> {
    Ok(num_to_f64(&to_num(v)?))
}

/// `CCur`: rounds half to even at four decimals.
pub fn to_currency(v: &Variant) -> VbaResult<Currency> {
    match to_num(v)? {
        Num::Int(i) => Currency::from_i64(i),
        Num::Single(s) => Currency::from_f64(f64::from(s)),
        Num::Double(d) => Currency::from_f64(d),
        Num::Cur(c) => Ok(c),
        Num::Dec(d) => d.to_currency(),
        Num::Str { value, decimal } => match decimal {
            Some(d) => d.to_currency(),
            None => Currency::from_f64(value),
        },
    }
}

/// `CDec`.
pub fn to_decimal(v: &Variant) -> VbaResult<Decimal> {
    match to_num(v)? {
        Num::Int(i) => Ok(Decimal::from_i64(i)),
        Num::Single(s) => Decimal::from_f32(s),
        Num::Double(d) => Decimal::from_f64(d),
        Num::Cur(c) => Ok(Decimal::from_currency(c)),
        Num::Dec(d) => Ok(d),
        Num::Str { value, decimal } => match decimal {
            Some(d) => Ok(d),
            None => Decimal::from_f64(value),
        },
    }
}

/// `CDate`: strings are parsed as dates, numbers are serials.
pub fn to_date(v: &Variant) -> VbaResult<Date> {
    match v {
        Variant::Date(d) => Ok(*d),
        Variant::String(s) => text::parse_date(s),
        other => Date::from_serial(num_to_f64(&to_num(other)?)),
    }
}

/// `CStr`.
pub fn to_vba_string(v: &Variant) -> VbaResult<VbaString> {
    let s = match v {
        Variant::Empty => String::new(),
        Variant::Boolean(b) => (if *b { "True" } else { "False" }).to_string(),
        Variant::Byte(x) => x.to_string(),
        Variant::Integer(x) => x.to_string(),
        Variant::Long(x) => x.to_string(),
        Variant::LongLong(x) => x.to_string(),
        Variant::Single(x) => text::format_single(*x),
        Variant::Double(x) => text::format_double(*x),
        Variant::Currency(x) => x.to_string(),
        Variant::Decimal(x) => x.to_string(),
        Variant::Date(x) => x.to_string(),
        Variant::String(s) => return Ok(s.clone()),
        Variant::Error(e) => format!("Error {}", e.number()),
        Variant::Null => return Err(VbaError::INVALID_USE_OF_NULL),
        Variant::Object(_) | Variant::UserDefined(_) | Variant::Array(_) => {
            return Err(VbaError::TYPE_MISMATCH);
        }
    };
    Ok(VbaString::from(s))
}

/// Converts `v` to the given declared type as an assignment or a `Cxxx`
/// function would. `Variant` (`CVar`) returns the value unchanged.
pub fn convert_to(v: &Variant, target: VarType) -> VbaResult<Variant> {
    Ok(match target {
        VarType::Variant => v.clone(),
        VarType::Boolean => Variant::Boolean(to_boolean(v)?),
        VarType::Byte => Variant::Byte(to_byte(v)?),
        VarType::Integer => Variant::Integer(to_integer(v)?),
        VarType::Long => Variant::Long(to_long(v)?),
        VarType::LongLong => Variant::LongLong(to_longlong(v)?),
        VarType::Single => Variant::Single(to_single(v)?),
        VarType::Double => Variant::Double(to_double(v)?),
        VarType::Currency => Variant::Currency(to_currency(v)?),
        VarType::Decimal => Variant::Decimal(to_decimal(v)?),
        VarType::Date => Variant::Date(to_date(v)?),
        VarType::String => Variant::String(to_vba_string(v)?),
        VarType::Object | VarType::DataObject => match v {
            Variant::Object(_) => v.clone(),
            Variant::Null => return Err(VbaError::INVALID_USE_OF_NULL),
            Variant::Error(e) => return Err(*e),
            _ => return Err(VbaError::TYPE_MISMATCH),
        },
        VarType::UserDefinedType => match v {
            Variant::UserDefined(_) => v.clone(),
            _ => return Err(VbaError::TYPE_MISMATCH),
        },
        VarType::Error => match v {
            Variant::Error(_) => v.clone(),
            _ => return Err(VbaError::TYPE_MISMATCH),
        },
        VarType::Null => match v {
            Variant::Null => Variant::Null,
            _ => return Err(VbaError::TYPE_MISMATCH),
        },
        VarType::Empty => match v {
            Variant::Empty => Variant::Empty,
            _ => return Err(VbaError::TYPE_MISMATCH),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Variant {
        Variant::from(s)
    }

    fn e<T: std::fmt::Debug>(r: VbaResult<T>) -> i32 {
        r.unwrap_err().number()
    }

    #[test]
    fn cint_rounding_and_range() {
        for (input, want) in [
            (Variant::Double(0.5), 0),
            (Variant::Double(1.5), 2),
            (Variant::Double(2.5), 2),
            (Variant::Double(-0.5), 0),
            (Variant::Double(-1.5), -2),
            (Variant::Double(-2.5), -2),
            (Variant::Double(32767.4), 32767),
            (Variant::Double(-32768.5), -32768),
            (Variant::Single(2.5), 2),
            (Variant::Currency(Currency::from_scaled(25_000)), 2),
            (Variant::Currency(Currency::from_scaled(35_000)), 4),
            (Variant::Decimal(Decimal::parse("2.5").unwrap().unwrap()), 2),
            (Variant::Boolean(true), -1),
            (Variant::Empty, 0),
            (v("12"), 12),
            (v(" 3.5 "), 4),
            (v("&HFFFF"), -1),
            (Variant::Byte(255), 255),
            (Variant::Long(-32768), -32768),
        ] {
            assert_eq!(to_integer(&input), Ok(want), "{input:?}");
        }
        for input in [
            Variant::Double(32767.5),
            Variant::Double(-32768.6),
            Variant::Double(f64::NAN),
            Variant::Double(f64::INFINITY),
            Variant::Double(f64::NEG_INFINITY),
            Variant::Long(32768),
            Variant::Long(-32769),
            Variant::LongLong(1 << 40),
            v("32768"),
            v("1e30"),
        ] {
            assert_eq!(e(to_integer(&input)), 6, "{input:?}");
        }
    }

    #[test]
    fn type_mismatch_and_null() {
        assert_eq!(e(to_integer(&v("abc"))), 13);
        assert_eq!(e(to_integer(&v(""))), 13);
        assert_eq!(e(to_integer(&Variant::Null)), 94);
        assert_eq!(e(to_vba_string(&Variant::Null)), 94);
        assert_eq!(e(to_boolean(&Variant::Null)), 94);
        assert_eq!(e(to_double(&Variant::Error(VbaError::new(5)))), 5);
        assert_eq!(
            e(to_long(&Variant::Object(
                crate::variant::ObjectRef::Nothing
            ))),
            13
        );
        let arr = crate::array::VbaArray::new_dynamic(VarType::Long);
        assert_eq!(e(to_long(&Variant::Array(Box::new(arr)))), 13);
    }

    #[test]
    fn clng_boundaries() {
        assert_eq!(to_long(&Variant::Double(2147483647.4)), Ok(i32::MAX));
        assert_eq!(e(to_long(&Variant::Double(2147483647.5))), 6);
        assert_eq!(to_long(&Variant::Double(-2147483648.5)), Ok(i32::MIN));
        assert_eq!(e(to_long(&Variant::Double(-2147483648.6))), 6);
        assert_eq!(e(to_long(&Variant::LongLong(i64::MAX))), 6);
        assert_eq!(
            to_longlong(&Variant::Double(9.0e18)),
            Ok(9_000_000_000_000_000_000)
        );
        assert_eq!(e(to_longlong(&Variant::Double(9.3e18))), 6);
        assert_eq!(to_longlong(&v("9223372036854775807")), Ok(i64::MAX));
    }

    #[test]
    fn cbyte() {
        assert_eq!(to_byte(&Variant::Double(255.4)), Ok(255));
        assert_eq!(to_byte(&Variant::Double(-0.4)), Ok(0));
        assert_eq!(e(to_byte(&Variant::Double(255.5))), 6);
        assert_eq!(to_byte(&Variant::Double(-0.5)), Ok(0));
        assert_eq!(e(to_byte(&Variant::Integer(-1))), 6);
        assert_eq!(e(to_byte(&Variant::Boolean(true))), 6);
        assert_eq!(to_byte(&Variant::Boolean(false)), Ok(0));
    }

    #[test]
    fn cbool() {
        for (input, want) in [
            (Variant::Long(0), false),
            (Variant::Long(5), true),
            (Variant::Double(0.0), false),
            (Variant::Double(-0.0), false),
            (Variant::Double(0.1), true),
            (Variant::Empty, false),
            (v("True"), true),
            (v(" false "), false),
            (v("0"), false),
            (v("-1"), true),
            (v("0.0"), false),
            (Variant::Currency(Currency::from_scaled(1)), true),
            (Variant::Decimal(Decimal::ZERO), false),
        ] {
            assert_eq!(to_boolean(&input), Ok(want), "{input:?}");
        }
        assert_eq!(e(to_boolean(&v("yes"))), 13);
        assert_eq!(e(to_boolean(&v(""))), 13);
    }

    #[test]
    fn csng_cdbl() {
        assert_eq!(to_single(&Variant::Double(1e39)).unwrap_err().number(), 6);
        assert_eq!(to_single(&Variant::Double(3.4e38)), Ok(3.4e38f32));
        assert_eq!(to_single(&Variant::Double(1e-50)), Ok(0.0));
        assert_eq!(e(to_single(&Variant::Double(f64::NAN))), 6);
        assert_eq!(to_double(&Variant::Single(0.1)), Ok(f64::from(0.1f32)));
        assert_eq!(to_double(&v("1.5")), Ok(1.5));
        assert_eq!(to_double(&Variant::Boolean(true)), Ok(-1.0));
        assert_eq!(
            to_double(&Variant::Currency(Currency::from_scaled(12_345))),
            Ok(1.2345)
        );
        assert_eq!(
            to_double(&Variant::Date(Date::from_serial(2.5).unwrap())),
            Ok(2.5)
        );
    }

    #[test]
    fn ccur() {
        let c = |x: &Variant| to_currency(x).map(Currency::scaled);
        assert_eq!(c(&Variant::Double(1.5)), Ok(15_000));
        assert_eq!(c(&v("1.23456")), Ok(12_346));
        assert_eq!(c(&v("0.00005")), Ok(0));
        assert_eq!(c(&v("0.00015")), Ok(2));
        assert_eq!(c(&Variant::Long(5)), Ok(50_000));
        assert_eq!(c(&Variant::Boolean(true)), Ok(-10_000));
        assert_eq!(e(to_currency(&Variant::Double(1e15))), 6);
        assert_eq!(e(to_currency(&Variant::LongLong(922_337_203_685_478))), 6);
        assert_eq!(
            c(&Variant::LongLong(922_337_203_685_477)),
            Ok(9_223_372_036_854_770_000)
        );
        assert_eq!(c(&Variant::Single(0.1)), Ok(1_000));
    }

    #[test]
    fn cdec() {
        let d = |x: &Variant| to_decimal(x).map(|d| d.to_string());
        assert_eq!(d(&Variant::Double(0.1)), Ok("0.1".to_string()));
        assert_eq!(d(&v("1.10")), Ok("1.10".to_string()));
        assert_eq!(d(&Variant::Long(-5)), Ok("-5".to_string()));
        assert_eq!(d(&Variant::Boolean(true)), Ok("-1".to_string()));
        assert_eq!(d(&Variant::Single(0.1)), Ok("0.1".to_string()));
        assert_eq!(
            d(&Variant::Currency(Currency::from_scaled(15_000))),
            Ok("1.5000".to_string())
        );
        assert_eq!(e(to_decimal(&Variant::Double(1e29))), 6);
        assert_eq!(e(to_decimal(&v("1e29"))), 6);
        assert_eq!(
            d(&v("79228162514264337593543950335")),
            Ok("79228162514264337593543950335".to_string())
        );
    }

    #[test]
    fn cdate() {
        let d = |x: &Variant| to_date(x).map(Date::serial);
        assert_eq!(d(&Variant::Double(43831.5)), Ok(43831.5));
        assert_eq!(d(&Variant::Long(1)), Ok(1.0));
        assert_eq!(d(&v("1/1/2020")), Ok(43831.0));
        assert_eq!(d(&Variant::Empty), Ok(0.0));
        assert_eq!(d(&Variant::Boolean(true)), Ok(-1.0));
        assert_eq!(e(to_date(&Variant::Double(3e6))), 6);
        assert_eq!(e(to_date(&Variant::Double(f64::NAN))), 6);
        assert_eq!(e(to_date(&v("not a date"))), 13);
    }

    #[test]
    fn cstr() {
        let s = |x: &Variant| to_vba_string(x).map(|s| s.to_string_lossy());
        for (input, want) in [
            (Variant::Empty, ""),
            (Variant::Boolean(true), "True"),
            (Variant::Boolean(false), "False"),
            (Variant::Byte(7), "7"),
            (Variant::Integer(-3), "-3"),
            (Variant::Long(123456), "123456"),
            (Variant::LongLong(i64::MIN), "-9223372036854775808"),
            (Variant::Single(0.1), "0.1"),
            (Variant::Double(1.0 / 3.0), "0.333333333333333"),
            (Variant::Double(1e15), "1E+15"),
            (Variant::Currency(Currency::from_scaled(15_000)), "1.5"),
            (
                Variant::Decimal(Decimal::parse("1.50").unwrap().unwrap()),
                "1.50",
            ),
            (
                Variant::Date(Date::from_serial(43831.5).unwrap()),
                "1/1/2020 12:00:00 PM",
            ),
            (Variant::Error(VbaError::new(5)), "Error 5"),
            (v("abc"), "abc"),
        ] {
            assert_eq!(s(&input).as_deref(), Ok(want), "{input:?}");
        }
    }

    #[test]
    fn string_roundtrip_keeps_surrogates() {
        let odd = Variant::String(VbaString::from_units(vec![0xD800, 0x41]));
        assert_eq!(to_vba_string(&odd).unwrap().as_units(), &[0xD800, 0x41]);
    }

    #[test]
    fn convert_to_dispatch() {
        assert_eq!(
            convert_to(&Variant::Double(2.5), VarType::Integer),
            Ok(Variant::Integer(2))
        );
        assert_eq!(
            convert_to(&Variant::Long(3), VarType::Variant),
            Ok(Variant::Long(3))
        );
        assert_eq!(
            convert_to(&Variant::Null, VarType::Variant),
            Ok(Variant::Null)
        );
        assert_eq!(e(convert_to(&Variant::Null, VarType::Long)), 94);
        assert_eq!(e(convert_to(&Variant::Long(1), VarType::Object)), 13);
        assert_eq!(
            convert_to(
                &Variant::Object(crate::variant::ObjectRef::Nothing),
                VarType::Object
            ),
            Ok(Variant::Object(crate::variant::ObjectRef::Nothing))
        );
        assert_eq!(convert_to(&Variant::Long(65), VarType::String), Ok(v("65")));
    }
}
