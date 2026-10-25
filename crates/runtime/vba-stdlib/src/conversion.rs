//! `VBA.Conversion` (MS-VBAL §6.1.2.2 and the Microsoft language reference).
//! The `C*` functions delegate to `vba_value::convert`.

use crate::call::{Args, Ctx, text};
use crate::error::{StdError, StdResult};
use vba_value::{VarType, Variant, VbaError, VbaResult, VbaString, convert};

const E5: VbaError = VbaError::INVALID_PROCEDURE_CALL;

/// `Hex` (`radix = 16`) and `Oct` (`radix = 8`) of a number.
///
/// `Byte` is unsigned 8-bit, `Integer`/`Boolean` 16-bit two's complement,
/// `Long` 32-bit, `LongLong` 64-bit; every other numeric subtype (and numeric
/// strings) is first rounded to a `Long`, so values outside the `Long` range
/// are error 6. `Empty` is `"0"`.
pub fn radix_string(v: &Variant, radix: u32) -> VbaResult<String> {
    let bits: u64 = match v {
        Variant::Empty => 0,
        Variant::Byte(x) => u64::from(*x),
        Variant::Integer(x) => u64::from(*x as u16),
        Variant::Boolean(b) => u64::from(if *b { 0xFFFFu16 } else { 0 }),
        Variant::Long(x) => u64::from(*x as u32),
        Variant::LongLong(x) => *x as u64,
        Variant::Null => return Err(VbaError::INVALID_USE_OF_NULL),
        other => u64::from(convert::to_long(other)? as u32),
    };
    Ok(if radix == 16 {
        format!("{bits:X}")
    } else {
        format!("{bits:o}")
    })
}

/// `Str(number)`: the number as text with a leading space in place of the
/// sign for non-negative values. `.` is always the decimal separator.
/// `Date` arguments are unsupported (their `Str` form is not documented).
pub fn str_number(v: &Variant) -> StdResult<VbaString> {
    let n = match v {
        Variant::Null => return Err(VbaError::INVALID_USE_OF_NULL.into()),
        Variant::Date(_) => return Err(StdError::Unsupported("Str of a Date value")),
        Variant::Empty => Variant::Integer(0),
        Variant::Boolean(b) => Variant::Integer(if *b { -1 } else { 0 }),
        Variant::String(_) => Variant::Double(convert::to_double(v)?),
        Variant::Error(e) => return Err((*e).into()),
        other => other.clone(),
    };
    if !n.is_numeric_subtype() {
        return Err(VbaError::TYPE_MISMATCH.into());
    }
    let s = convert::to_vba_string(&n)?.to_string_lossy();
    Ok(VbaString::from(if s.starts_with('-') {
        s
    } else {
        format!(" {s}")
    }))
}

fn is_val_blank(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{3000}')
}

/// `Val(string)`: the numbers in a string, stopping at the first character
/// that cannot be part of a number. Blanks (space, tab, CR, LF) are skipped,
/// including between digits. `&H`/`&O` prefixes read hexadecimal/octal: 16-bit
/// results are sign-extended unless the literal ends with `&`. A result beyond
/// the `Double` range is error 6.
pub fn val(s: &VbaString) -> VbaResult<f64> {
    let t = s.to_string_lossy();
    let mut it = t.chars().filter(|c| !is_val_blank(*c)).peekable();
    // Leading blanks were removed together with inner blanks, which is what
    // Val does for digits; a sign must still come first.
    if it.peek() == Some(&'&') {
        let mut look = it.clone();
        look.next();
        let radix = match look.next() {
            Some('H' | 'h') => 16,
            Some('O' | 'o') => 8,
            _ => return Ok(0.0),
        };
        let mut acc: u64 = 0;
        let mut digits = 0;
        let mut long_suffix = false;
        for c in look {
            if let Some(d) = c.to_digit(radix) {
                acc = acc
                    .saturating_mul(u64::from(radix))
                    .saturating_add(u64::from(d));
                digits += 1;
            } else {
                long_suffix = c == '&';
                break;
            }
        }
        if digits == 0 {
            return Ok(0.0);
        }
        return if acc > 0xFFFF_FFFF {
            Err(VbaError::OVERFLOW)
        } else if acc <= 0xFFFF && !long_suffix {
            Ok(f64::from(acc as u16 as i16))
        } else {
            Ok(f64::from(acc as u32 as i32))
        };
    }
    let mut out = String::new();
    if let Some(&c) = it.peek().filter(|c| matches!(c, '+' | '-')) {
        out.push(c);
        it.next();
    }
    let mut mant_digits = 0;
    let mut seen_point = false;
    while let Some(&c) = it.peek() {
        if c.is_ascii_digit() {
            mant_digits += 1;
        } else if c == '.' && !seen_point {
            seen_point = true;
        } else {
            break;
        }
        out.push(c);
        it.next();
    }
    if mant_digits == 0 {
        return Ok(0.0);
    }
    if matches!(it.peek(), Some('e' | 'E' | 'd' | 'D')) {
        let mut look = it.clone();
        look.next();
        let mut exp = String::from("e");
        if let Some(&c) = look.peek().filter(|c| matches!(c, '+' | '-')) {
            exp.push(c);
            look.next();
        }
        let mut n = 0;
        while let Some(&c) = look.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            exp.push(c);
            n += 1;
            look.next();
        }
        if n > 0 {
            out.push_str(&exp);
        }
    }
    let v: f64 = out.parse().map_err(|_| E5)?;
    if v.is_infinite() {
        Err(VbaError::OVERFLOW)
    } else {
        Ok(v)
    }
}

// ---------------------------------------------------------------------------
// Registry adapters.

pub(crate) fn f_hex(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) && !a.dollar {
        return Ok(Variant::Null);
    }
    Ok(text(radix_string(a.req(0)?, 16)?.as_str()))
}
pub(crate) fn f_oct(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) && !a.dollar {
        return Ok(Variant::Null);
    }
    Ok(text(radix_string(a.req(0)?, 8)?.as_str()))
}
pub(crate) fn f_str(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(text(str_number(a.req(0)?)?))
}
pub(crate) fn f_val(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Double(val(&a.string(0)?)?))
}
pub(crate) fn f_cverr(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let n = a.long(0)?;
    if !(0..=65535).contains(&n) {
        return Err(E5.into());
    }
    Ok(Variant::Error(VbaError::new(n)))
}
pub(crate) fn f_error(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let n = a.long_or(0, 0)?;
    if !(0..=65535).contains(&n) {
        return Err(E5.into());
    }
    Ok(text(if n == 0 {
        ""
    } else {
        VbaError::new(n).description()
    }))
}

macro_rules! cfn {
    ($name:ident, $ty:expr) => {
        pub(crate) fn $name(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
            Ok(convert::convert_to(a.req(0)?, $ty)?)
        }
    };
}
cfn!(f_cbool, VarType::Boolean);
cfn!(f_cbyte, VarType::Byte);
cfn!(f_ccur, VarType::Currency);
cfn!(f_cdate, VarType::Date);
cfn!(f_cdbl, VarType::Double);
cfn!(f_cdec, VarType::Decimal);
cfn!(f_cint, VarType::Integer);
cfn!(f_clng, VarType::Long);
cfn!(f_clnglng, VarType::LongLong);
cfn!(f_csng, VarType::Single);
cfn!(f_cstr, VarType::String);
cfn!(f_cvar, VarType::Variant);

pub(crate) fn f_clngptr(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let v = convert::to_longlong(a.req(0)?)?;
    Ok(Variant::from_long_ptr(v, ctx.pointer_width)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> f64 {
        val(&VbaString::from(s)).unwrap()
    }

    #[test]
    fn val_rules() {
        assert_eq!(v("2457"), 2457.0);
        assert_eq!(v(" 2 45 7"), 2457.0);
        assert_eq!(v("24 and 57"), 24.0);
        assert_eq!(v("1615 198th Street N.E."), 1615198.0);
        assert_eq!(v("&HFFFF"), -1.0);
        assert_eq!(v("&H8000"), -32768.0);
        assert_eq!(v("&HFFFF&"), 65535.0);
        assert_eq!(v("&HFFFFFFFF"), -1.0);
        assert_eq!(v("&O17"), 15.0);
        assert_eq!(v("-.5e1x"), -5.0);
        assert_eq!(v("1e"), 1.0);
        assert_eq!(v("1,000"), 1.0);
        assert_eq!(v("abc"), 0.0);
        assert_eq!(v(""), 0.0);
        assert_eq!(v("3d2"), 300.0);
        assert_eq!(v("$5"), 0.0);
    }
}
