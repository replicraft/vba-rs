//! Number and date text conversion used by `CStr`, `&` and by implicit
//! String-to-number coercion (MS-VBAL §5.6.9.x, §6.1.2.3).
//!
//! The behavior reproduced is that of the invariant/en-US locale. Other
//! locales (decimal comma, other date orders) are not supported.
//!
//! Provenance: the grammar of numeric strings (leading/trailing blanks,
//! `&H`/`&O` radix prefixes, exponent letters `E` and `D`, thousands commas,
//! a leading `$`) and the 15/7 significant digit output of doubles/singles are
//! documented VBA behavior rather than MS-VBAL text.

use crate::date::Date;
use crate::decimal::Decimal;
use crate::error::{VbaError, VbaResult};
use crate::string::VbaString;

/// Formats a double as `CStr` does: up to 15 significant digits, shortest
/// form, scientific (`1E+15`, `1.5E-05`) when the decimal exponent is below -4
/// or at least 15.
pub fn format_double(v: f64) -> String {
    format_float(v, 15)
}

/// Formats a single as `CStr` does (up to 7 significant digits).
pub fn format_single(v: f32) -> String {
    format_float(f64::from(v), 7)
}

fn format_float(v: f64, precision: usize) -> String {
    if v.is_nan() {
        return "1.#QNAN".to_string();
    }
    if v.is_infinite() {
        return if v < 0.0 { "-1.#INF" } else { "1.#INF" }.to_string();
    }
    if v == 0.0 {
        return "0".to_string();
    }
    let text = format!("{:.*e}", precision - 1, v.abs());
    let (mant, exp) = text.split_once('e').unwrap_or((&text, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let all: String = mant.chars().filter(char::is_ascii_digit).collect();
    let digits = all.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let sign = if v < 0.0 { "-" } else { "" };
    if exp < -4 || exp >= precision as i32 {
        let frac = &digits[1..];
        let mantissa = if frac.is_empty() {
            digits[..1].to_string()
        } else {
            format!("{}.{}", &digits[..1], frac)
        };
        let esign = if exp < 0 { '-' } else { '+' };
        format!("{sign}{mantissa}E{esign}{:02}", exp.abs())
    } else if exp >= 0 {
        let int_len = exp as usize + 1;
        if digits.len() <= int_len {
            format!("{sign}{digits}{}", "0".repeat(int_len - digits.len()))
        } else {
            format!("{sign}{}.{}", &digits[..int_len], &digits[int_len..])
        }
    } else {
        format!("{sign}0.{}{digits}", "0".repeat((-exp - 1) as usize))
    }
}

/// Result of parsing a numeric string.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Numeric {
    /// An integral value (including `&H`/`&O` literals) that fits in 64 bits.
    Integer(i64),
    /// A real number. `decimal` holds the exact decimal value when it is in
    /// the `Decimal` range.
    Real {
        /// The value as a double.
        value: f64,
        /// The exact value when representable as a `Decimal`.
        decimal: Option<Decimal>,
    },
}

impl Numeric {
    /// The value as a double.
    pub fn as_f64(&self) -> f64 {
        match *self {
            Numeric::Integer(i) => i as f64,
            Numeric::Real { value, .. } => value,
        }
    }
}

fn is_blank(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{a0}' | '\u{3000}')
}

/// Parses a numeric string as implicit coercion and `CDbl`/`CLng`/... do.
/// Non-numeric text (including the empty string) is error 13; values beyond
/// the double range are error 6.
pub fn parse_numeric(s: &VbaString) -> VbaResult<Numeric> {
    let text = s.to_string_lossy();
    let t = text.trim_matches(is_blank);
    if t.is_empty() {
        return Err(VbaError::TYPE_MISMATCH);
    }
    let b = t.as_bytes();
    if b.len() >= 2 && b[0] == b'&' && matches!(b[1], b'h' | b'H' | b'o' | b'O') {
        return parse_radix(&t[2..], matches!(b[1], b'h' | b'H'));
    }
    parse_decimal_text(t)
}

fn parse_radix(rest: &str, hex: bool) -> VbaResult<Numeric> {
    let (digits, long_suffix) = match rest.strip_suffix('&') {
        Some(d) => (d, true),
        None => (rest, false),
    };
    if digits.is_empty() {
        return Err(VbaError::TYPE_MISMATCH);
    }
    let radix = if hex { 16 } else { 8 };
    let mut v: u64 = 0;
    for c in digits.chars() {
        let d = c.to_digit(radix).ok_or(VbaError::TYPE_MISMATCH)?;
        v = v
            .checked_mul(u64::from(radix))
            .and_then(|x| x.checked_add(u64::from(d)))
            .ok_or(VbaError::OVERFLOW)?;
    }
    let value = if !long_suffix && v <= 0xFFFF {
        i64::from(v as u16 as i16)
    } else if v <= 0xFFFF_FFFF {
        i64::from(v as u32 as i32)
    } else {
        v as i64
    };
    Ok(Numeric::Integer(value))
}

fn parse_decimal_text(t: &str) -> VbaResult<Numeric> {
    let b = t.as_bytes();
    let mut i = 0;
    let mut neg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        neg = b[i] == b'-';
        i += 1;
    }
    if i < b.len() && b[i] == b'$' {
        i += 1;
    }
    let mut clean = String::new();
    if neg {
        clean.push('-');
    }
    let mut digits = 0;
    let mut is_real = false;
    while i < b.len() && (b[i].is_ascii_digit() || (b[i] == b',' && digits > 0)) {
        if b[i] != b',' {
            clean.push(b[i] as char);
            digits += 1;
        }
        i += 1;
    }
    if i < b.len() && b[i] == b'.' {
        is_real = true;
        clean.push('.');
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            clean.push(b[i] as char);
            digits += 1;
            i += 1;
        }
    }
    if digits == 0 {
        return Err(VbaError::TYPE_MISMATCH);
    }
    if i < b.len() && matches!(b[i], b'e' | b'E' | b'd' | b'D') {
        is_real = true;
        clean.push('e');
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            clean.push(b[i] as char);
            i += 1;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            clean.push(b[i] as char);
            i += 1;
        }
        if start == i {
            return Err(VbaError::TYPE_MISMATCH);
        }
    }
    if i != b.len() {
        return Err(VbaError::TYPE_MISMATCH);
    }
    if !is_real && let Ok(v) = clean.parse::<i64>() {
        return Ok(Numeric::Integer(v));
    }
    let value: f64 = clean.parse().map_err(|_| VbaError::TYPE_MISMATCH)?;
    if !value.is_finite() {
        return Err(VbaError::OVERFLOW);
    }
    let decimal = Decimal::parse(&clean).and_then(Result::ok);
    Ok(Numeric::Real { value, decimal })
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

fn month_from_name(tok: &str) -> Option<u32> {
    let l = tok.to_ascii_lowercase();
    if l.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| m.starts_with(&l))
        .map(|p| p as u32 + 1)
}

fn expand_year(y: u32, digits: usize) -> i32 {
    if digits <= 2 {
        // Windows default two-digit year window: 00-29 -> 2000s, 30-99 -> 1900s.
        if y <= 29 {
            2000 + y as i32
        } else {
            1900 + y as i32
        }
    } else {
        y as i32
    }
}

fn parse_time_token(tok: &str) -> Option<i64> {
    let parts: Vec<&str> = tok.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    let mut vals = [0i64; 3];
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() || !p.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        vals[i] = p.parse().ok()?;
    }
    if vals[1] > 59 || vals[2] > 59 {
        return None;
    }
    Some(vals[0] * 3600 + vals[1] * 60 + vals[2])
}

/// Applies an AM/PM designator to a seconds-of-day value (hour must be 1..=12).
fn apply_meridiem(secs: i64, pm: bool) -> Option<i64> {
    let h = secs / 3600;
    if !(1..=12).contains(&h) {
        return None;
    }
    let h24 = match (h, pm) {
        (12, false) => 0,
        (12, true) => 12,
        (h, false) => h,
        (h, true) => h + 12,
    };
    Some(secs % 3600 + h24 * 3600)
}

fn parse_number_u32(s: &str) -> Option<u32> {
    if s.is_empty() || s.len() > 9 || !s.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Parses a date/time string as `CDate` does for the en-US locale.
///
/// Supported: numeric strings (serial numbers); `m/d/y`, `m-d-y`, `y-m-d`,
/// `y/m/d`; month-name forms (`Jan 5, 2020`, `5 January 2020`, `5-Jan-2020`);
/// times `h:mm[:ss]` and `h AM/PM` with optional AM/PM. Dates without a year
/// are not supported (they depend on the current date). Failure is error 13.
pub fn parse_date(s: &VbaString) -> VbaResult<Date> {
    let text = s.to_string_lossy();
    let t = text.trim_matches(is_blank);
    if t.is_empty() {
        return Err(VbaError::TYPE_MISMATCH);
    }
    if let Ok(n) = parse_numeric(s) {
        return Date::from_serial(n.as_f64());
    }
    let mut time: Option<i64> = None;
    let mut meridiem: Option<bool> = None;
    let mut date_tokens: Vec<String> = Vec::new();
    for raw in t
        .split(|c: char| is_blank(c) || c == ',')
        .filter(|x| !x.is_empty())
    {
        let upper = raw.to_ascii_uppercase();
        if upper == "AM" || upper == "PM" {
            if meridiem.is_some() {
                return Err(VbaError::TYPE_MISMATCH);
            }
            meridiem = Some(upper == "PM");
            // "6 PM": a preceding lone number is the hour.
            if time.is_none()
                && let Some(h) = date_tokens.last().and_then(|l| parse_number_u32(l))
            {
                date_tokens.pop();
                time = Some(i64::from(h) * 3600);
            }
            continue;
        }
        let (core, attached) = if let Some(c) = upper.strip_suffix("AM") {
            (c, Some(false))
        } else if let Some(c) = upper.strip_suffix("PM") {
            (c, Some(true))
        } else {
            (upper.as_str(), None)
        };
        if core.contains(':') {
            if time.is_some() {
                return Err(VbaError::TYPE_MISMATCH);
            }
            time = Some(parse_time_token(core).ok_or(VbaError::TYPE_MISMATCH)?);
            meridiem = attached.or(meridiem);
        } else if let (Some(pm), Some(h)) = (attached, parse_number_u32(core)) {
            if time.is_some() {
                return Err(VbaError::TYPE_MISMATCH);
            }
            time = Some(i64::from(h) * 3600);
            meridiem = Some(pm);
        } else {
            date_tokens.push(raw.to_string());
        }
    }
    if let Some(pm) = meridiem {
        let secs = time.ok_or(VbaError::TYPE_MISMATCH)?;
        time = Some(apply_meridiem(secs, pm).ok_or(VbaError::TYPE_MISMATCH)?);
    }
    if time.is_some_and(|s| s >= 86_400) {
        return Err(VbaError::TYPE_MISMATCH);
    }
    let secs = time.unwrap_or(0);
    if date_tokens.is_empty() {
        if time.is_none() {
            return Err(VbaError::TYPE_MISMATCH);
        }
        return Date::from_serial(secs as f64 / 86_400.0);
    }
    let (y, m, d) = parse_date_tokens(&date_tokens).ok_or(VbaError::TYPE_MISMATCH)?;
    let base = Date::from_ymd_hms(y, m, d, 0, 0, 0).map_err(|_| VbaError::TYPE_MISMATCH)?;
    let serial = base.serial();
    let frac = secs as f64 / 86_400.0;
    Date::from_serial(if serial < 0.0 {
        serial - frac
    } else {
        serial + frac
    })
}

/// Interprets the non-time tokens of a date string as (year, month, day).
fn parse_date_tokens(tokens: &[String]) -> Option<(i32, u32, u32)> {
    // Normalize to three parts: split a single separated token, or take the
    // three words as they are.
    let parts: Vec<&str> = match tokens {
        [one] => {
            let sep = one.find(['/', '-', '.'])?;
            let sepc = one.as_bytes()[sep] as char;
            one.split(sepc).collect()
        }
        [a, b, c] => vec![a.as_str(), b.as_str(), c.as_str()],
        _ => return None,
    };
    if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    let named: Vec<Option<u32>> = parts.iter().map(|p| month_from_name(p)).collect();
    if let Some(mpos) = named.iter().position(Option::is_some) {
        let m = named[mpos]?;
        let others: Vec<(u32, usize)> = (0..3)
            .filter(|&i| i != mpos)
            .map(|i| parse_number_u32(parts[i]).map(|n| (n, parts[i].len())))
            .collect::<Option<_>>()?;
        let (a, al) = others[0];
        let (b, bl) = others[1];
        // A 3+ digit first number is the year (y-Mon-d); otherwise d/y order
        // as written (Mon d y, d Mon y).
        return if mpos == 2 && al >= 3 || mpos == 1 && al >= 3 {
            Some((expand_year(a, al), m, b))
        } else {
            Some((expand_year(b, bl), m, a))
        };
    }
    let n: Vec<(u32, usize)> = (0..3)
        .map(|i| parse_number_u32(parts[i]).map(|v| (v, parts[i].len())))
        .collect::<Option<_>>()?;
    if n[0].1 >= 3 {
        Some((n[0].0 as i32, n[1].0, n[2].0))
    } else {
        Some((expand_year(n[2].0, n[2].1), n[0].0, n[1].0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_formatting() {
        for (v, s) in [
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (-1.5, "-1.5"),
            (0.1 + 0.2, "0.3"),
            (1.0 / 3.0, "0.333333333333333"),
            (100.0, "100"),
            (123456789012345.0, "123456789012345"),
            (1e14, "100000000000000"),
            (1e15, "1E+15"),
            (1.5e20, "1.5E+20"),
            (123456789012345678.0, "1.23456789012346E+17"),
            (0.0001, "0.0001"),
            (0.00001, "1E-05"),
            (1.5e-5, "1.5E-05"),
            (-2.5e-10, "-2.5E-10"),
            (1e100, "1E+100"),
            (1e-100, "1E-100"),
            (0.000123, "0.000123"),
            (std::f64::consts::PI, "3.14159265358979"),
            (f64::MAX, "1.79769313486232E+308"),
        ] {
            assert_eq!(format_double(v), s, "{v:?}");
        }
    }

    #[test]
    fn single_formatting() {
        for (v, s) in [
            (0.1f32, "0.1"),
            (1.0e7f32, "1E+07"),
            (1234567.0f32, "1234567"),
            (12345678.0f32, "1.234568E+07"),
            (std::f32::consts::PI, "3.141593"),
            (1.0e-5f32, "1E-05"),
            (-0.5f32, "-0.5"),
        ] {
            assert_eq!(format_single(v), s, "{v:?}");
        }
    }

    fn num(s: &str) -> VbaResult<Numeric> {
        parse_numeric(&VbaString::from(s))
    }

    #[test]
    fn numeric_parsing() {
        assert_eq!(num("42"), Ok(Numeric::Integer(42)));
        assert_eq!(num("  -42  "), Ok(Numeric::Integer(-42)));
        assert_eq!(num("+7"), Ok(Numeric::Integer(7)));
        assert_eq!(num("1,234"), Ok(Numeric::Integer(1234)));
        assert_eq!(num("$5"), Ok(Numeric::Integer(5)));
        assert_eq!(num("&HFF"), Ok(Numeric::Integer(255)));
        assert_eq!(num("&HFFFF"), Ok(Numeric::Integer(-1)));
        assert_eq!(num("&H8000"), Ok(Numeric::Integer(-32768)));
        assert_eq!(num("&H10000"), Ok(Numeric::Integer(65536)));
        assert_eq!(num("&HFFFF&"), Ok(Numeric::Integer(65535)));
        assert_eq!(num("&HFFFFFFFF"), Ok(Numeric::Integer(-1)));
        assert_eq!(num("&o17"), Ok(Numeric::Integer(15)));
        assert_eq!(num("&O177777"), Ok(Numeric::Integer(-1)));
        assert_eq!(num("9223372036854775807"), Ok(Numeric::Integer(i64::MAX)));
        let Ok(Numeric::Real { value, decimal }) = num("1.5") else {
            panic!()
        };
        assert_eq!(value, 1.5);
        assert_eq!(decimal.unwrap().to_string(), "1.5");
        assert_eq!(num(".5").unwrap().as_f64(), 0.5);
        assert_eq!(num("5.").unwrap().as_f64(), 5.0);
        assert_eq!(num("1e3").unwrap().as_f64(), 1000.0);
        assert_eq!(num("1.5D2").unwrap().as_f64(), 150.0);
        assert_eq!(num("-1E-2").unwrap().as_f64(), -0.01);
        assert!(matches!(
            num("99999999999999999999"),
            Ok(Numeric::Real { .. })
        ));
    }

    #[test]
    fn numeric_parsing_errors() {
        for s in [
            "", "  ", "abc", "1x", "--1", "1e", "e5", "&H", "&HG", "1 2", ".", "$", ",",
        ] {
            assert_eq!(num(s), Err(VbaError::TYPE_MISMATCH), "{s:?}");
        }
        assert_eq!(num("1e999"), Err(VbaError::OVERFLOW));
        assert_eq!(num("&HFFFFFFFFFFFFFFFFF"), Err(VbaError::OVERFLOW));
    }

    fn date(s: &str) -> VbaResult<f64> {
        parse_date(&VbaString::from(s)).map(Date::serial)
    }

    #[test]
    fn date_parsing() {
        assert_eq!(date("1/1/2020"), Ok(43831.0));
        assert_eq!(date("2020-01-01"), Ok(43831.0));
        assert_eq!(date("2020/1/1"), Ok(43831.0));
        assert_eq!(date("1/1/20"), Ok(43831.0));
        assert_eq!(date("1/1/99"), date("1/1/1999"));
        assert_eq!(date("Jan 1, 2020"), Ok(43831.0));
        assert_eq!(date("January 1 2020"), Ok(43831.0));
        assert_eq!(date("1 Jan 2020"), Ok(43831.0));
        assert_eq!(date("1-Jan-2020"), Ok(43831.0));
        assert_eq!(date("1/1/2020 12:00"), Ok(43831.5));
        assert_eq!(date("1/1/2020 12:00:00 PM"), Ok(43831.5));
        assert_eq!(date("1/1/2020 12:00 AM"), Ok(43831.0));
        assert_eq!(date("1/1/2020 6:30 PM"), Ok(43831.0 + 18.5 / 24.0));
        assert_eq!(date("6:00"), Ok(0.25));
        assert_eq!(date("6:00 PM"), Ok(0.75));
        assert_eq!(date("6:00PM"), Ok(0.75));
        assert_eq!(date("6 PM"), Ok(0.75));
        assert_eq!(date("43831.5"), Ok(43831.5));
        assert_eq!(date("12/30/1899"), Ok(0.0));
        assert_eq!(date("12/29/1899 6:00"), Ok(-1.25));
    }

    #[test]
    fn date_parsing_errors() {
        for s in [
            "",
            "abc",
            "2/30/2020",
            "13/1/2020",
            "1/1/2020 25:00",
            "1/1/2020 13:00 PM",
            "1:2:3:4",
            "Jan 2020 x",
        ] {
            assert_eq!(date(s), Err(VbaError::TYPE_MISMATCH), "{s:?}");
        }
        assert_eq!(date("99999999"), Err(VbaError::OVERFLOW));
    }
}
