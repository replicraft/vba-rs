//! `Format`, `FormatNumber`, `FormatCurrency`, `FormatPercent` and
//! `FormatDateTime` (Microsoft VBA language reference: Format function,
//! named and user-defined numeric, date/time and string formats).
//!
//! Numbers are formatted from their 15-significant-digit decimal value with
//! ties rounded away from zero. Locale data comes from the host
//! ([`Locale`]); the default is en-US. The Microsoft Learn Format reference
//! (2024 revision) defines no fraction formats, no `*` fill, no `?`
//! placeholder and no `[color]`/`[condition]` sections for VBA's `Format`; none
//! is interpreted. For leniency `*x`, `_x`, `?` and `[...]` are skipped or
//! approximated (undocumented, unverified).

use crate::datetime::{rel_weekday, week_of_year};
use crate::digits::{Dec, Rounding};
use crate::host::Locale;
use vba_value::{Date, Variant, VbaError, VbaResult, VbaString, convert, text};

const E5: VbaError = VbaError::INVALID_PROCEDURE_CALL;

/// Settings for `w`/`ww` and the locale.
#[derive(Clone, Copy)]
pub struct FormatEnv<'a> {
    /// Locale data.
    pub locale: &'a Locale,
    /// Resolved first day of the week, 1 (Sunday) to 7.
    pub first_day_of_week: i64,
    /// Resolved first week of the year rule, 1 to 3.
    pub first_week_of_year: i64,
}

impl<'a> FormatEnv<'a> {
    /// Environment with the locale defaults.
    pub fn new(locale: &'a Locale) -> Self {
        FormatEnv {
            locale,
            first_day_of_week: i64::from(locale.first_day_of_week),
            first_week_of_year: i64::from(locale.first_week_of_year),
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point

/// `Format(expression, format)`. `Null` yields `Null`.
pub fn format(value: &Variant, fmt: Option<&VbaString>, env: &FormatEnv<'_>) -> VbaResult<Variant> {
    // "Format truncates format to 257 characters."
    let fmt_text: String = fmt
        .map(VbaString::to_string_lossy)
        .unwrap_or_default()
        .chars()
        .take(MAX_FORMAT_CHARS)
        .collect();
    if matches!(value, Variant::Null) {
        return Ok(match null_section(&fmt_text) {
            Some(t) => Variant::String(VbaString::from(t)),
            None => Variant::Null,
        });
    }
    if fmt_text.is_empty() {
        return Ok(Variant::String(convert::to_vba_string(value)?));
    }
    let out = format_text(value, &fmt_text, env)?;
    Ok(Variant::String(VbaString::from(out)))
}

/// Maximum length of a format expression (documented truncation).
const MAX_FORMAT_CHARS: usize = 257;

/// The documented section for `Null`: the fourth section of a four-section
/// numeric format, or the second section of a two-section string format.
/// Only the literal text of that section is produced (placeholders have no
/// digits to show). `None` means the result is `Null`.
fn null_section(fmt: &str) -> Option<String> {
    let chars: Vec<char> = fmt.chars().collect();
    let secs = split_sections(&chars);
    let stringy = fmt
        .chars()
        .any(|c| matches!(c, '@' | '&' | '<' | '>' | '!'));
    let sec = match secs.len() {
        4 => &secs[3],
        2 if stringy => &secs[1],
        _ => return None,
    };
    let mut out = String::new();
    let mut i = 0;
    while i < sec.len() {
        match sec[i] {
            '"' => {
                let end = sec[i + 1..]
                    .iter()
                    .position(|&x| x == '"')
                    .map_or(sec.len(), |p| i + 1 + p);
                out.extend(&sec[i + 1..end]);
                i = end;
            }
            '\\' => {
                if let Some(&n) = sec.get(i + 1) {
                    out.push(n);
                }
                i += 1;
            }
            '0' | '#' | '.' | ',' | '%' | '@' | '&' | '<' | '>' | '!' => {}
            c => out.push(c),
        }
        i += 1;
    }
    Some(out)
}

enum Kind {
    Number(f64),
    Date(Date),
    Text(VbaString),
}

fn classify(value: &Variant, fmt: &str) -> VbaResult<Kind> {
    Ok(match value {
        Variant::Date(d) => Kind::Date(*d),
        Variant::String(s) => {
            let stringy = fmt
                .chars()
                .any(|c| matches!(c, '@' | '&' | '<' | '>' | '!'));
            if stringy {
                Kind::Text(s.clone())
            } else if let Ok(n) = text::parse_numeric(s) {
                Kind::Number(n.as_f64())
            } else if is_date_format(fmt) {
                match text::parse_date(s) {
                    Ok(d) => Kind::Date(d),
                    Err(_) => Kind::Text(s.clone()),
                }
            } else {
                Kind::Text(s.clone())
            }
        }
        Variant::Empty => Kind::Text(VbaString::new()),
        Variant::Object(_) | Variant::UserDefined(_) | Variant::Array(_) => {
            return Err(VbaError::TYPE_MISMATCH);
        }
        other => Kind::Number(convert::to_double(other)?),
    })
}

fn format_text(value: &Variant, fmt: &str, env: &FormatEnv<'_>) -> VbaResult<String> {
    let loc = env.locale;
    let lower = fmt.trim().to_ascii_lowercase();
    let kind = classify(value, fmt)?;
    // Named formats.
    match lower.as_str() {
        "general number" => {
            return Ok(match kind {
                Kind::Number(_) | Kind::Date(_) => {
                    let v = match value {
                        Variant::Date(d) => Variant::Double(d.serial()),
                        v => v.clone(),
                    };
                    convert::to_vba_string(&v)?.to_string_lossy()
                }
                Kind::Text(s) => s.to_string_lossy(),
            });
        }
        "currency" | "fixed" | "standard" | "percent" | "scientific" | "yes/no" | "true/false"
        | "on/off" => {
            let x = match kind {
                Kind::Number(x) => x,
                Kind::Date(d) => d.serial(),
                Kind::Text(s) => return Ok(s.to_string_lossy()),
            };
            return Ok(match lower.as_str() {
                "currency" => number_with(
                    x,
                    &currency_pattern(
                        loc,
                        loc.currency_digits.into(),
                        true,
                        loc.currency_negative_parens,
                        true,
                    ),
                    env,
                ),
                "fixed" => number_with(x, "0.00", env),
                "standard" => number_with(x, "#,##0.00", env),
                "percent" => number_with(x, "0.00%", env),
                "scientific" => number_with(x, "0.00E+00", env),
                "yes/no" => (if x != 0.0 { "Yes" } else { "No" }).into(),
                "true/false" => (if x != 0.0 { "True" } else { "False" }).into(),
                _ => (if x != 0.0 { "On" } else { "Off" }).into(),
            });
        }
        "general date" | "long date" | "medium date" | "short date" | "long time"
        | "medium time" | "short time" => {
            let d = match kind {
                Kind::Date(d) => d,
                Kind::Number(x) => Date::from_serial(x)?,
                Kind::Text(s) => text::parse_date(&s)?,
            };
            let pat = match lower.as_str() {
                "general date" => return general_date(d, env),
                "long date" => &loc.long_date,
                "medium date" => &loc.medium_date,
                "short date" => &loc.short_date,
                "long time" => &loc.long_time,
                "medium time" => &loc.medium_time,
                _ => &loc.short_time,
            };
            return format_date(d, pat, env);
        }
        _ => {}
    }
    match kind {
        Kind::Text(s) => Ok(format_string(&s, fmt)),
        Kind::Date(d) => {
            if is_date_format(fmt) {
                format_date(d, fmt, env)
            } else {
                Ok(number_with(d.serial(), fmt, env))
            }
        }
        Kind::Number(x) => {
            if is_date_format(fmt) {
                format_date(Date::from_serial(x)?, fmt, env)
            } else {
                Ok(number_with(x, fmt, env))
            }
        }
    }
}

fn general_date(d: Date, env: &FormatEnv<'_>) -> VbaResult<String> {
    let loc = env.locale;
    let s = d.serial();
    let has_date = s.trunc() != 0.0;
    let has_time = s.fract() != 0.0;
    match (has_date, has_time) {
        (_, false) => format_date(d, &loc.short_date, env),
        (false, true) => format_date(d, &loc.long_time, env),
        (true, true) => format_date(d, &format!("{} {}", loc.short_date, loc.long_time), env),
    }
}

// ---------------------------------------------------------------------------
// Named-format helpers

fn currency_pattern(loc: &Locale, digits: u32, leading: bool, parens: bool, group: bool) -> String {
    let int = match (group, leading) {
        (true, true) => "#,##0",
        (true, false) => "#,###",
        (false, true) => "0",
        (false, false) => "#",
    };
    let frac = if digits > 0 {
        format!(".{}", "0".repeat(digits as usize))
    } else {
        String::new()
    };
    let sym = format!("\"{}\"", loc.currency_symbol);
    let body = if loc.currency_prefix {
        format!("{sym}{int}{frac}")
    } else {
        format!("{int}{frac}{sym}")
    };
    if parens {
        format!("{body};({body})")
    } else {
        body
    }
}

/// Tri-state option of the `FormatXxx` functions: -1 true, 0 false, -2 use
/// the locale default.
fn tri(v: i32, default: bool) -> VbaResult<bool> {
    match v {
        -2 => Ok(default),
        0 => Ok(false),
        -1 => Ok(true),
        _ => Err(E5),
    }
}

fn digits_arg(v: i32, default: u32) -> VbaResult<u32> {
    match v {
        -1 => Ok(default),
        0..=99 => Ok(v as u32),
        _ => Err(E5),
    }
}

/// `FormatNumber`.
pub fn format_number(
    x: f64,
    digits: i32,
    lead: i32,
    parens: i32,
    group: i32,
    env: &FormatEnv<'_>,
) -> VbaResult<String> {
    let d = digits_arg(digits, 2)?;
    let pat = number_pattern(
        "",
        d,
        tri(lead, true)?,
        tri(parens, false)?,
        tri(group, true)?,
        false,
    );
    Ok(number_with(x, &pat, env))
}

/// `FormatCurrency`.
pub fn format_currency(
    x: f64,
    digits: i32,
    lead: i32,
    parens: i32,
    group: i32,
    env: &FormatEnv<'_>,
) -> VbaResult<String> {
    let loc = env.locale;
    let d = digits_arg(digits, u32::from(loc.currency_digits))?;
    let pat = currency_pattern(
        loc,
        d,
        tri(lead, true)?,
        tri(parens, loc.currency_negative_parens)?,
        tri(group, true)?,
    );
    Ok(number_with(x, &pat, env))
}

/// `FormatPercent`: multiplies by 100 and appends `%`.
pub fn format_percent(
    x: f64,
    digits: i32,
    lead: i32,
    parens: i32,
    group: i32,
    env: &FormatEnv<'_>,
) -> VbaResult<String> {
    let d = digits_arg(digits, 2)?;
    let pat = number_pattern(
        "",
        d,
        tri(lead, true)?,
        tri(parens, false)?,
        tri(group, true)?,
        true,
    );
    Ok(number_with(x, &pat, env))
}

fn number_pattern(
    prefix: &str,
    digits: u32,
    lead: bool,
    parens: bool,
    group: bool,
    percent: bool,
) -> String {
    let int = match (group, lead) {
        (true, true) => "#,##0",
        (true, false) => "#,###",
        (false, true) => "0",
        (false, false) => "#",
    };
    let frac = if digits > 0 {
        format!(".{}", "0".repeat(digits as usize))
    } else {
        String::new()
    };
    let body = format!("{prefix}{int}{frac}{}", if percent { "%" } else { "" });
    if parens {
        format!("{body};({body})")
    } else {
        body
    }
}

/// `FormatDateTime(date, namedFormat)`: 0 general, 1 long date, 2 short date,
/// 3 long time, 4 short time (24-hour `hh:mm`).
pub fn format_date_time(d: Date, named: i32, env: &FormatEnv<'_>) -> VbaResult<String> {
    let loc = env.locale;
    match named {
        0 => general_date(d, env),
        1 => format_date(d, &loc.long_date, env),
        2 => format_date(d, &loc.short_date, env),
        3 => format_date(d, &loc.long_time, env),
        4 => format_date(d, &loc.short_time, env),
        _ => Err(E5),
    }
}

// ---------------------------------------------------------------------------
// Detection

fn is_date_format(fmt: &str) -> bool {
    let c: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '"' => {
                i += 1;
                while i < c.len() && c[i] != '"' {
                    i += 1;
                }
            }
            '\\' | '*' | '_' => i += 1,
            '[' => {
                while i < c.len() && c[i] != ']' {
                    i += 1;
                }
            }
            ch if "dymhnswqtc".contains(ch.to_ascii_lowercase()) => return true,
            _ => {}
        }
        i += 1;
    }
    false
}

// ---------------------------------------------------------------------------
// Numbers

fn split_sections(c: &[char]) -> Vec<Vec<char>> {
    let mut out = vec![Vec::new()];
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            ';' => out.push(Vec::new()),
            '"' => {
                let end = c[i + 1..]
                    .iter()
                    .position(|&x| x == '"')
                    .map_or(c.len(), |p| i + 1 + p + 1);
                out.last_mut()
                    .expect("section")
                    .extend_from_slice(&c[i..end.min(c.len())]);
                i = end;
                continue;
            }
            '\\' if i + 1 < c.len() => {
                out.last_mut()
                    .expect("section")
                    .extend_from_slice(&c[i..i + 2]);
                i += 2;
                continue;
            }
            ch => out.last_mut().expect("section").push(ch),
        }
        i += 1;
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
enum NTok {
    Lit(String),
    Digit(char),
    Point,
    Comma,
    Percent,
    Exp { upper: bool, plus: bool },
}

fn tokenize_number(c: &[char]) -> Vec<NTok> {
    let mut out = Vec::new();
    let (mut i, mut seen_point) = (0, false);
    while i < c.len() {
        let ch = c[i];
        match ch {
            '"' => {
                let end = c[i + 1..]
                    .iter()
                    .position(|&x| x == '"')
                    .map_or(c.len(), |p| i + 1 + p);
                out.push(NTok::Lit(c[i + 1..end].iter().collect()));
                i = end;
            }
            '\\' => {
                if let Some(&n) = c.get(i + 1) {
                    out.push(NTok::Lit(n.to_string()));
                }
                i += 1;
            }
            '0' | '#' | '?' => out.push(NTok::Digit(ch)),
            '.' if !seen_point => {
                seen_point = true;
                out.push(NTok::Point);
            }
            ',' => out.push(NTok::Comma),
            '%' => out.push(NTok::Percent),
            'E' | 'e' if matches!(c.get(i + 1), Some('+' | '-')) => {
                out.push(NTok::Exp {
                    upper: ch == 'E',
                    plus: c[i + 1] == '+',
                });
                i += 1;
            }
            '*' => i += 1,
            '_' => {
                out.push(NTok::Lit(" ".into()));
                i += 1;
            }
            '[' => {
                while i < c.len() && c[i] != ']' {
                    i += 1;
                }
            }
            other => out.push(NTok::Lit(other.to_string())),
        }
        i += 1;
    }
    out
}

/// Formats `x` with a user-defined numeric format (up to three sections).
fn number_with(x: f64, fmt: &str, env: &FormatEnv<'_>) -> String {
    let chars: Vec<char> = fmt.chars().collect();
    let secs = split_sections(&chars);
    // An empty section between semicolons uses the positive format; for a
    // negative value the minus sign is then added as with one section.
    let (sec, auto_minus) = if x > 0.0 || (x == 0.0 && secs.len() < 3) {
        (&secs[0], false)
    } else if x < 0.0 {
        if secs.len() >= 2 && !secs[1].is_empty() {
            (&secs[1], false)
        } else {
            (&secs[0], true)
        }
    } else if secs[2].is_empty() {
        (&secs[0], false)
    } else {
        (&secs[2], false)
    };
    if sec.is_empty() {
        return String::new();
    }
    let toks = tokenize_number(sec);
    let body = render_number(&toks, Dec::from_f64(x), env.locale);
    if auto_minus { format!("-{body}") } else { body }
}

fn render_number(toks: &[NTok], mut dec: Dec, loc: &Locale) -> String {
    let point_idx = toks.iter().position(|t| *t == NTok::Point);
    let exp_idx = toks.iter().position(|t| matches!(t, NTok::Exp { .. }));
    let int_end = point_idx.or(exp_idx).unwrap_or(toks.len());
    let dec_end = exp_idx.unwrap_or(toks.len());
    let int_slots: Vec<usize> = (0..int_end)
        .filter(|&i| matches!(toks[i], NTok::Digit(_)))
        .collect();
    let dec_slots: Vec<usize> = point_idx
        .map(|p| {
            (p + 1..dec_end)
                .filter(|&i| matches!(toks[i], NTok::Digit(_)))
                .collect()
        })
        .unwrap_or_default();
    let exp_width = exp_idx
        .map(|e| {
            toks[e + 1..]
                .iter()
                .filter(|t| matches!(t, NTok::Digit(_)))
                .count()
        })
        .unwrap_or(0);
    // Commas: thousands separators between integer digits, scaling commas
    // right after the last integer digit.
    let (mut thousands, mut scale) = (false, 0);
    let last_digit = (0..dec_end)
        .rev()
        .find(|&i| matches!(toks[i], NTok::Digit(_)));
    for (i, t) in toks.iter().enumerate().take(dec_end) {
        if *t == NTok::Comma {
            if last_digit.is_some_and(|l| i > l) {
                scale += 1;
            } else if let (Some(&f), Some(&l)) = (int_slots.first(), int_slots.last())
                && i > f
                && i < l
            {
                thousands = true;
            }
        }
    }
    let pct = toks.iter().filter(|t| **t == NTok::Percent).count() as i32;
    if !dec.is_zero() {
        dec.point += 2 * pct - 3 * scale;
    }
    let n = int_slots.len();
    let mut exp_val = 0i32;
    let places = dec_slots.len() as i32;
    if exp_idx.is_some() {
        let k = n.max(1) as i32;
        if !dec.is_zero() {
            exp_val = dec.exp10() - (k - 1);
            dec.point = k;
        }
        dec = dec.round(places, Rounding::HalfUp);
        if !dec.is_zero() && dec.point > k {
            dec.point = k;
            exp_val += 1;
        }
    } else {
        dec = dec.round(places, Rounding::HalfUp);
    }
    let m = dec.point.max(0);
    let digit_char = |pos: i32| char::from(b'0' + dec.digit_at(pos));
    let mut out = String::new();
    let (mut int_seen, mut dec_seen) = (0usize, 0usize);
    // Fractional bookkeeping.
    let last_required = dec_slots
        .iter()
        .rposition(|&i| toks[i] == NTok::Digit('0'))
        .map_or(-1, |p| p as i32);
    let last_nonzero = (0..places)
        .rev()
        .find(|&i| dec.digit_at(-(i + 1)) != 0)
        .unwrap_or(-1);
    let limit = last_required.max(last_nonzero);
    let emit_extra = |out: &mut String| {
        for rank in (n as i32..m).rev() {
            out.push(digit_char(rank));
            if thousands && rank % 3 == 0 && rank > 0 {
                out.push(loc.thousands);
            }
        }
    };
    let mut extra_done = false;
    for (i, t) in toks.iter().enumerate() {
        if exp_idx.is_some_and(|e| i > e) && matches!(t, NTok::Digit(_)) {
            continue;
        }
        match t {
            NTok::Lit(s) => out.push_str(s),
            NTok::Percent => out.push('%'),
            NTok::Comma => {}
            NTok::Point => {
                if n == 0 && !extra_done {
                    emit_extra(&mut out);
                    extra_done = true;
                }
                out.push(loc.decimal);
            }
            NTok::Digit(kind) => {
                if i < int_end {
                    if int_seen == 0 && !extra_done {
                        emit_extra(&mut out);
                        extra_done = true;
                    }
                    let rank = (n - 1 - int_seen) as i32;
                    int_seen += 1;
                    if rank < m {
                        out.push(digit_char(rank));
                        if thousands && rank % 3 == 0 && rank > 0 {
                            out.push(loc.thousands);
                        }
                    } else if *kind == '0' {
                        out.push('0');
                        if thousands && rank % 3 == 0 && rank > 0 {
                            out.push(loc.thousands);
                        }
                    } else if *kind == '?' {
                        out.push(' ');
                    }
                } else {
                    let idx = dec_seen as i32;
                    dec_seen += 1;
                    if idx <= limit {
                        out.push(digit_char(-(idx + 1)));
                    } else if *kind == '?' {
                        out.push(' ');
                    }
                }
            }
            NTok::Exp { upper, plus } => {
                out.push(if *upper { 'E' } else { 'e' });
                if exp_val < 0 {
                    out.push('-');
                } else if *plus {
                    out.push('+');
                }
                out.push_str(&format!("{:0>w$}", exp_val.abs(), w = exp_width.max(1)));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Strings

fn format_string(s: &VbaString, fmt: &str) -> String {
    let chars: Vec<char> = fmt.chars().collect();
    let secs = split_sections(&chars);
    let sec = if s.is_empty() && secs.len() > 1 {
        &secs[1]
    } else {
        &secs[0]
    };
    let val: Vec<char> = s.to_string_lossy().chars().collect();
    #[derive(PartialEq)]
    enum T {
        Lit(String),
        At,
        Amp,
    }
    let (mut toks, mut ltr, mut case) = (Vec::new(), false, None::<bool>);
    let mut i = 0;
    while i < sec.len() {
        match sec[i] {
            '@' => toks.push(T::At),
            '&' => toks.push(T::Amp),
            '<' => case = Some(false),
            '>' => case = Some(true),
            '!' => ltr = true,
            '"' => {
                let end = sec[i + 1..]
                    .iter()
                    .position(|&x| x == '"')
                    .map_or(sec.len(), |p| i + 1 + p);
                toks.push(T::Lit(sec[i + 1..end].iter().collect()));
                i = end;
            }
            '\\' => {
                if let Some(&n) = sec.get(i + 1) {
                    toks.push(T::Lit(n.to_string()));
                }
                i += 1;
            }
            '*' => i += 1,
            c => toks.push(T::Lit(c.to_string())),
        }
        i += 1;
    }
    let conv = |c: char, out: &mut String| match case {
        Some(true) => out.extend(c.to_uppercase()),
        Some(false) => out.extend(c.to_lowercase()),
        None => out.push(c),
    };
    let n = toks.iter().filter(|t| matches!(t, T::At | T::Amp)).count();
    let mut out = String::new();
    if n == 0 {
        for t in &toks {
            if let T::Lit(l) = t {
                out.push_str(l);
            }
        }
        // No placeholders: the whole value is the result (case flags apply).
        let mut v = String::new();
        for &c in &val {
            conv(c, &mut v);
        }
        return if out.is_empty() {
            v
        } else {
            format!("{v}{out}")
        };
    }
    let (l, nn) = (val.len() as i64, n as i64);
    let mut ph = 0i64;
    for t in &toks {
        match t {
            T::Lit(s) => out.push_str(s),
            T::At | T::Amp => {
                let pad = if *t == T::At { Some(' ') } else { None };
                if ltr {
                    match val.get(ph as usize) {
                        Some(&c) => conv(c, &mut out),
                        None => out.extend(pad),
                    }
                    if ph == nn - 1 {
                        for &c in val.iter().skip(n) {
                            conv(c, &mut out);
                        }
                    }
                } else {
                    let off = l - nn;
                    let ci = ph + off;
                    if ci < 0 {
                        out.extend(pad);
                    } else if ph == 0 {
                        for &c in &val[..=ci as usize] {
                            conv(c, &mut out);
                        }
                    } else {
                        conv(val[ci as usize], &mut out);
                    }
                }
                ph += 1;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Dates

#[derive(Debug, Clone, PartialEq)]
enum DTok {
    Lit(String),
    Day(usize),
    Week(usize),
    Month(usize),
    Quarter,
    Year(usize),
    Hour(usize),
    Minute(usize),
    Second(usize),
    LongTime,
    General,
    Ampm(AmpmStyle),
    DateSep,
    TimeSep,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AmpmStyle {
    Upper,
    Lower,
    LetterUpper,
    LetterLower,
    Designator,
}

fn tokenize_date(c: &[char]) -> Vec<DTok> {
    let mut out = Vec::new();
    let mut i = 0;
    let lower = |k: usize| c.get(k).map(|x| x.to_ascii_lowercase());
    while i < c.len() {
        let ch = c[i];
        let lc = ch.to_ascii_lowercase();
        let run = |ch: char| {
            c[i..]
                .iter()
                .take_while(|x| x.to_ascii_lowercase() == ch)
                .count()
        };
        match lc {
            '"' => {
                let end = c[i + 1..]
                    .iter()
                    .position(|&x| x == '"')
                    .map_or(c.len(), |p| i + 1 + p);
                out.push(DTok::Lit(c[i + 1..end].iter().collect()));
                i = end + 1;
            }
            '\\' => {
                if let Some(&n) = c.get(i + 1) {
                    out.push(DTok::Lit(n.to_string()));
                }
                i += 2;
            }
            'd' => {
                let r = run('d');
                out.push(DTok::Day(r.min(6)));
                i += r;
            }
            'w' => {
                let r = run('w');
                out.push(DTok::Week(r.min(2)));
                i += r;
            }
            'm' => {
                let r = run('m');
                out.push(DTok::Month(r.min(4)));
                i += r;
            }
            'q' => {
                out.push(DTok::Quarter);
                i += 1;
            }
            'y' => {
                let r = run('y');
                out.push(DTok::Year(if r == 1 {
                    1
                } else if r == 2 {
                    2
                } else {
                    4
                }));
                i += r;
            }
            'h' => {
                let r = run('h');
                out.push(DTok::Hour(r.min(2)));
                i += r;
            }
            'n' => {
                let r = run('n');
                out.push(DTok::Minute(r.min(2)));
                i += r;
            }
            's' => {
                let r = run('s');
                out.push(DTok::Second(r.min(2)));
                i += r;
            }
            't' if run('t') >= 5 => {
                out.push(DTok::LongTime);
                i += run('t');
            }
            'c' => {
                out.push(DTok::General);
                i += 1;
            }
            'a' if lower(i + 1) == Some('m')
                && lower(i + 2) == Some('/')
                && lower(i + 3) == Some('p')
                && lower(i + 4) == Some('m') =>
            {
                out.push(DTok::Ampm(if ch == 'A' {
                    AmpmStyle::Upper
                } else {
                    AmpmStyle::Lower
                }));
                i += 5;
            }
            'a' if lower(i + 1) == Some('m')
                && lower(i + 2) == Some('p')
                && lower(i + 3) == Some('m') =>
            {
                out.push(DTok::Ampm(AmpmStyle::Designator));
                i += 4;
            }
            'a' if lower(i + 1) == Some('/') && lower(i + 2) == Some('p') => {
                out.push(DTok::Ampm(if ch == 'A' {
                    AmpmStyle::LetterUpper
                } else {
                    AmpmStyle::LetterLower
                }));
                i += 3;
            }
            '/' => {
                out.push(DTok::DateSep);
                i += 1;
            }
            ':' => {
                out.push(DTok::TimeSep);
                i += 1;
            }
            '*' => i += 2,
            '_' => {
                out.push(DTok::Lit(" ".into()));
                i += 2;
            }
            '[' => {
                while i < c.len() && c[i] != ']' {
                    i += 1;
                }
                i += 1;
            }
            _ => {
                out.push(DTok::Lit(ch.to_string()));
                i += 1;
            }
        }
    }
    out
}

/// Formats a date with a user-defined date/time pattern.
pub fn format_date(d: Date, pattern: &str, env: &FormatEnv<'_>) -> VbaResult<String> {
    let chars: Vec<char> = pattern.chars().collect();
    let toks = tokenize_date(&chars);
    let loc = env.locale;
    let p = d.parts();
    let has_ampm = toks.iter().any(|t| matches!(t, DTok::Ampm(_)));
    let is_field = |t: &DTok| !matches!(t, DTok::Lit(_) | DTok::DateSep | DTok::TimeSep);
    let mut out = String::new();
    for (i, t) in toks.iter().enumerate() {
        match t {
            DTok::Lit(s) => out.push_str(s),
            DTok::DateSep => out.push(loc.date_separator),
            DTok::TimeSep => out.push(loc.time_separator),
            DTok::Day(n) => {
                let wd = (d.weekday() - 1) as usize;
                match n {
                    1 => out.push_str(&p.day.to_string()),
                    2 => out.push_str(&format!("{:02}", p.day)),
                    3 => out.push_str(&loc.day_abbreviations[wd]),
                    4 => out.push_str(&loc.day_names[wd]),
                    5 => out.push_str(&format_date(d, &loc.short_date, env)?),
                    _ => out.push_str(&format_date(d, &loc.long_date, env)?),
                }
            }
            DTok::Week(n) => {
                if *n == 1 {
                    out.push_str(&rel_weekday(d, env.first_day_of_week).to_string());
                } else {
                    out.push_str(
                        &week_of_year(d, env.first_day_of_week, env.first_week_of_year)?
                            .to_string(),
                    );
                }
            }
            DTok::Month(n) => {
                let prev_h = toks[..i]
                    .iter()
                    .rev()
                    .find(|t| is_field(t))
                    .is_some_and(|t| matches!(t, DTok::Hour(_)));
                let next_s = toks[i + 1..]
                    .iter()
                    .find(|t| is_field(t))
                    .is_some_and(|t| matches!(t, DTok::Second(_)));
                if (prev_h || next_s) && *n <= 2 {
                    out.push_str(&if *n == 1 {
                        p.minute.to_string()
                    } else {
                        format!("{:02}", p.minute)
                    });
                } else {
                    let mi = (p.month - 1) as usize;
                    match n {
                        1 => out.push_str(&p.month.to_string()),
                        2 => out.push_str(&format!("{:02}", p.month)),
                        3 => out.push_str(&loc.month_abbreviations[mi]),
                        _ => out.push_str(&loc.month_names[mi]),
                    }
                }
            }
            DTok::Quarter => out.push_str(&((p.month - 1) / 3 + 1).to_string()),
            DTok::Year(n) => match n {
                1 => out.push_str(&crate::datetime::day_of_year(d).to_string()),
                2 => out.push_str(&format!("{:02}", p.year % 100)),
                _ => out.push_str(&format!("{:04}", p.year)),
            },
            DTok::Hour(n) => {
                let h = if has_ampm {
                    if p.hour.is_multiple_of(12) {
                        12
                    } else {
                        p.hour % 12
                    }
                } else {
                    p.hour
                };
                out.push_str(&if *n == 1 {
                    h.to_string()
                } else {
                    format!("{h:02}")
                });
            }
            DTok::Minute(n) => out.push_str(&if *n == 1 {
                p.minute.to_string()
            } else {
                format!("{:02}", p.minute)
            }),
            DTok::Second(n) => out.push_str(&if *n == 1 {
                p.second.to_string()
            } else {
                format!("{:02}", p.second)
            }),
            DTok::LongTime => out.push_str(&format_date(d, &loc.long_time, env)?),
            DTok::General => out.push_str(&general_date(d, env)?),
            DTok::Ampm(style) => {
                let s = if p.hour < 12 { &loc.am } else { &loc.pm };
                match style {
                    AmpmStyle::Upper => out.push_str(&s.to_uppercase()),
                    AmpmStyle::Lower => out.push_str(&s.to_lowercase()),
                    AmpmStyle::Designator => out.push_str(s),
                    AmpmStyle::LetterUpper => {
                        out.extend(s.chars().next().map(|c| c.to_ascii_uppercase()))
                    }
                    AmpmStyle::LetterLower => {
                        out.extend(s.chars().next().map(|c| c.to_ascii_lowercase()))
                    }
                }
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Registry adapters.

use crate::call::{Args, Ctx, text as text_variant};
use crate::error::StdResult;

fn env_for<'a>(
    ctx: &'a Ctx<'_>,
    a: &Args<'_>,
    fdow_i: usize,
    fwoy_i: usize,
) -> StdResult<FormatEnv<'a>> {
    let loc = ctx.host.locale();
    let mut env = FormatEnv::new(loc);
    let fdow = a.long_or(fdow_i, 1)?;
    let fwoy = a.long_or(fwoy_i, 1)?;
    match fdow {
        0 => {}
        1..=7 => env.first_day_of_week = i64::from(fdow),
        _ => return Err(E5.into()),
    }
    match fwoy {
        0 => {}
        1..=3 => env.first_week_of_year = i64::from(fwoy),
        _ => return Err(E5.into()),
    }
    Ok(env)
}

pub(crate) fn f_format(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let v = a.req(0)?;
    if v.is_null() && a.dollar {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    let fmt = match a.get(1) {
        Some(f) => Some(convert::to_vba_string(f)?),
        None => None,
    };
    let env = env_for(ctx, a, 2, 3)?;
    Ok(format(v, fmt.as_ref(), &env)?)
}

macro_rules! format_fn {
    ($name:ident, $imp:ident) => {
        pub(crate) fn $name(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
            let env = FormatEnv::new(ctx.host.locale());
            let x = convert::to_double(a.req(0)?)?;
            let s = $imp(
                x,
                a.long_or(1, -1)?,
                a.long_or(2, -2)?,
                a.long_or(3, -2)?,
                a.long_or(4, -2)?,
                &env,
            )?;
            Ok(text_variant(s.as_str()))
        }
    };
}
format_fn!(f_format_number, format_number);
format_fn!(f_format_currency, format_currency);
format_fn!(f_format_percent, format_percent);

pub(crate) fn f_format_date_time(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let env = FormatEnv::new(ctx.host.locale());
    let d = convert::to_date(a.req(0)?)?;
    Ok(text_variant(
        format_date_time(d, a.long_or(1, 0)?, &env)?.as_str(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: Variant, fmt: &str) -> String {
        let loc = Locale::en_us();
        let env = FormatEnv::new(&loc);
        match format(&v, Some(&VbaString::from(fmt)), &env).unwrap() {
            Variant::String(s) => s.to_string_lossy(),
            other => panic!("{other:?}"),
        }
    }
    fn n(x: f64, fmt: &str) -> String {
        f(Variant::Double(x), fmt)
    }

    #[test]
    fn documented_numeric_examples() {
        // Examples from the Format function reference (user-defined formats).
        assert_eq!(n(5459.4, "##,##0.00"), "5,459.40");
        assert_eq!(n(334.9, "###0.00"), "334.90");
        assert_eq!(n(5.0, "0.00%"), "500.00%");
        assert_eq!(n(5.0, "0.00E+00"), "5.00E+00");
        assert_eq!(n(1234.5, "0.0E-00"), "1.2E03");
        assert_eq!(n(0.000123, "0.00E+00"), "1.23E-04");
        assert_eq!(n(5.0, "0.##"), "5.");
        assert_eq!(n(0.5, "#.00"), ".50");
        assert_eq!(n(0.0, "#"), "");
        assert_eq!(n(-5.0, "0.00"), "-5.00");
        assert_eq!(n(-5.0, "0.00;(0.00)"), "(5.00)");
        assert_eq!(n(0.0, "0.00;(0.00);\"zero\""), "zero");
        assert_eq!(n(1234567.0, "#,##0"), "1,234,567");
        assert_eq!(n(1234567.0, "#,##0,"), "1,235");
        assert_eq!(n(1234567.0, "0.0,,"), "1.2");
        assert_eq!(n(0.125, "0.00"), "0.13");
        assert_eq!(n(12.0, "00000"), "00012");
        assert_eq!(n(123456.0, "(###) ###-####"), "() 12-3456");
        assert_eq!(n(12.3, ".00"), "12.30");
        assert_eq!(f(Variant::from_text("123"), "00000"), "00123");
    }

    #[test]
    fn named_numeric() {
        assert_eq!(n(1234.5, "Currency"), "$1,234.50");
        assert_eq!(n(-1234.5, "Currency"), "($1,234.50)");
        assert_eq!(n(1234.5, "Fixed"), "1234.50");
        assert_eq!(n(1234.5, "Standard"), "1,234.50");
        assert_eq!(n(0.5, "Percent"), "50.00%");
        assert_eq!(n(1234.5, "Scientific"), "1.23E+03");
        assert_eq!(n(1.0, "Yes/No"), "Yes");
        assert_eq!(n(0.0, "True/False"), "False");
        assert_eq!(n(2.0, "On/Off"), "On");
        assert_eq!(n(1234.5, "General Number"), "1234.5");
    }

    #[test]
    fn strings() {
        // Documented: Format("HELLO", "<") is "hello", Format("This is it", ">") is upper.
        assert_eq!(f(Variant::from_text("HELLO"), "<"), "hello");
        assert_eq!(f(Variant::from_text("This is it"), ">"), "THIS IS IT");
        assert_eq!(f(Variant::from_text("abc"), "@@@@@"), "  abc");
        assert_eq!(f(Variant::from_text("abc"), "&&&&&"), "abc");
        assert_eq!(f(Variant::from_text("abc"), "!@@@@@"), "abc  ");
        assert_eq!(f(Variant::from_text("abcdef"), "@@@"), "abcdef");
        assert_eq!(
            f(Variant::from_text("1234567890"), "(@@@) @@@-@@@@"),
            "(123) 456-7890"
        );
    }

    #[test]
    fn dates() {
        let d = Date::from_ymd_hms(2024, 3, 5, 14, 7, 9).unwrap();
        let df = |p: &str| f(Variant::Date(d), p);
        assert_eq!(df("yyyy-mm-dd hh:nn:ss"), "2024-03-05 14:07:09");
        assert_eq!(df("m/d/yy"), "3/5/24");
        assert_eq!(df("dddd, mmmm d, yyyy"), "Tuesday, March 5, 2024");
        assert_eq!(df("ddd mmm"), "Tue Mar");
        assert_eq!(df("h:mm AM/PM"), "2:07 PM");
        assert_eq!(df("h:mm am/pm"), "2:07 pm");
        assert_eq!(df("h:mm A/P"), "2:07 P");
        assert_eq!(df("hh:mm:ss"), "14:07:09");
        assert_eq!(df("m"), "3");
        assert_eq!(df("h:m"), "14:7");
        assert_eq!(df("q"), "1");
        assert_eq!(df("y"), "65");
        assert_eq!(df("ww"), "10");
        assert_eq!(df("w"), "3");
        assert_eq!(df("General Date"), "3/5/2024 2:07:09 PM");
        assert_eq!(df("Long Date"), "Tuesday, March 5, 2024");
        assert_eq!(df("Medium Date"), "05-Mar-24");
        assert_eq!(df("Short Date"), "3/5/2024");
        assert_eq!(df("Long Time"), "2:07:09 PM");
        assert_eq!(df("Medium Time"), "02:07 PM");
        assert_eq!(df("Short Time"), "14:07");
        assert_eq!(df("\"Q\"q"), "Q1");
        assert_eq!(df("ddddd"), "3/5/2024");
        assert_eq!(f(Variant::Double(36526.0), "yyyy"), "2000");
        assert_eq!(df("0"), "45357");
    }

    // ---- Microsoft Learn "Format function (VBA)" examples, en-US assumed.

    fn doc_time() -> Variant {
        Variant::Date(Date::from_ymd_hms(1899, 12, 30, 17, 4, 23).unwrap())
    }
    fn doc_date() -> Variant {
        Variant::Date(Date::from_ymd_hms(1993, 1, 27, 0, 0, 0).unwrap())
    }

    #[test]
    fn learn_page_examples() {
        assert_eq!(f(doc_time(), "h:m:s"), "17:4:23");
        assert_eq!(f(doc_time(), "hh:mm:ss am/pm"), "05:04:23 pm");
        assert_eq!(f(doc_time(), "hh:mm:ss AM/PM"), "05:04:23 PM");
        assert_eq!(f(doc_date(), "dddd, mmm d yyyy"), "Wednesday, Jan 27 1993");
        let loc = Locale::en_us();
        let env = FormatEnv::new(&loc);
        // No format: like Str but without the leading space.
        assert_eq!(
            format(&Variant::Long(23), None, &env).unwrap(),
            Variant::from("23")
        );
        assert_eq!(n(5459.4, "##,##0.00"), "5,459.40");
        assert_eq!(n(334.9, "###0.00"), "334.90");
        assert_eq!(n(5.0, "0.00%"), "500.00%");
        assert_eq!(f(Variant::from_text("HELLO"), "<"), "hello");
        assert_eq!(f(Variant::from_text("This is it"), ">"), "THIS IS IT");
    }

    #[test]
    fn learn_page_sections() {
        // "$#,##0;($#,##0)"
        assert_eq!(n(1234.0, "$#,##0;($#,##0)"), "$1,234");
        assert_eq!(n(-1234.0, "$#,##0;($#,##0)"), "($1,234)");
        assert_eq!(n(0.0, "$#,##0;($#,##0)"), "$0");
        // "$#,##0;;\Z\e\r\o": the missing section uses the positive format.
        let z = "$#,##0;;\\Z\\e\\r\\o";
        assert_eq!(n(1234.0, z), "$1,234");
        assert_eq!(n(0.0, z), "Zero");
        // Negative: positive format; the minus sign is added as with one
        // section (unverified against Office).
        assert_eq!(n(-1234.0, z), "-$1,234");
        // Three sections: positive; negative; zero.
        assert_eq!(n(0.0, "0;-0;\"nil\""), "nil");
        // Four sections: the fourth is for Null.
        let loc = Locale::en_us();
        let env = FormatEnv::new(&loc);
        let four = VbaString::from("0;-0;0;\"none\"");
        assert_eq!(
            format(&Variant::Null, Some(&four), &env).unwrap(),
            Variant::from("none")
        );
        let one = VbaString::from("0");
        assert_eq!(
            format(&Variant::Null, Some(&one), &env).unwrap(),
            Variant::Null
        );
        // Strings: second section for Null and "".
        let two = VbaString::from("@@;\"empty\"");
        assert_eq!(
            format(&Variant::Null, Some(&two), &env).unwrap(),
            Variant::from("empty")
        );
        assert_eq!(f(Variant::from_text(""), "@@;\"empty\""), "empty");
        assert_eq!(f(Variant::from_text("x"), "@@;\"empty\""), " x");
    }

    #[test]
    fn learn_page_thousands_scaling() {
        assert_eq!(n(100_000_000.0, "##0,,"), "100");
        assert_eq!(n(100_000.0, "##0,,"), "0");
        // Two adjacent separators elsewhere only select grouping.
        assert_eq!(n(1234567.0, "#,,##0"), "1,234,567");
    }

    #[test]
    fn learn_page_literals_and_escapes() {
        assert_eq!(n(5.0, "$0 (+)-"), "$5 (+)-");
        assert_eq!(n(5.0, "0\\%"), "5%");
        assert_eq!(n(5.0, "0\"%\""), "5%");
        assert_eq!(n(5.0, "0\\\\"), "5\\");
        assert_eq!(n(5.0, "\\a0"), "a5");
        // Format is truncated to 257 characters.
        let long = format!("0{}", "x".repeat(300));
        assert_eq!(n(1.0, &long), format!("1{}", "x".repeat(256)));
    }

    #[test]
    fn learn_page_date_tokens() {
        let d = Date::from_ymd_hms(1993, 1, 27, 17, 4, 23).unwrap();
        let df = |p: &str| f(Variant::Date(d), p);
        assert_eq!(df("d dd"), "27 27");
        assert_eq!(df("ddd dddd"), "Wed Wednesday");
        assert_eq!(df("w"), "4");
        assert_eq!(df("m mm mmm mmmm"), "1 01 Jan January");
        assert_eq!(df("q"), "1");
        assert_eq!(df("y"), "27");
        assert_eq!(df("yy yyyy"), "93 1993");
        assert_eq!(df("h hh n nn s ss"), "17 17 4 04 23 23");
        assert_eq!(df("h:mm AMPM"), "5:04 PM");
        assert_eq!(df("h A/P"), "5 P");
        assert_eq!(df("h a/p"), "5 p");
        assert_eq!(df("ddddd"), "1/27/1993");
        assert_eq!(df("dddddd"), "Wednesday, January 27, 1993");
        assert_eq!(df("ttttt"), "5:04:23 PM");
        assert_eq!(df("c"), "1/27/1993 5:04:23 PM");
        // c shows only the date, or only the time, when the other is absent.
        assert_eq!(f(doc_date(), "c"), "1/27/1993");
        assert_eq!(f(doc_time(), "c"), "5:04:23 PM");
        assert_eq!(f(doc_date(), "General Date"), "1/27/1993");
        assert_eq!(f(doc_time(), "General Date"), "5:04:23 PM");
        // `m` directly after `h` is minutes.
        assert_eq!(df("hh:m"), "17:4");
        assert_eq!(df("hh:mm"), "17:04");
    }

    #[test]
    fn undocumented_tokens_are_tolerated_not_interpreted() {
        // `*` fill, `_` spacing, `?` and `[color]` do not appear in the VBA
        // Format reference; they are skipped/approximated (unverified).
        assert_eq!(n(5.0, "[Red]0"), "5");
    }
}
