//! `VBA.Strings` (MS-VBAL §6.1.2.11 and the Microsoft VBA language
//! reference). Typed functions come first, the registry adapters at the end.
//!
//! Provenance: argument ranges, result types and error numbers follow the
//! Microsoft language reference (**doc**); MS-VBAL only lists these functions
//! by signature (**spec**). Rules marked "unverified" in `docs/vba-stdlib.md`
//! have not been compared against a real host.

use crate::call::{Args, Ctx, compare_arg, text};
use crate::error::{StdError, StdResult};
use vba_value::{
    Bounds, CompareMode, VarType, Variant, VbaArray, VbaError, VbaResult, VbaString, convert,
};

const E5: VbaError = VbaError::INVALID_PROCEDURE_CALL;

/// Windows-1252 mapping of bytes 0x80..=0x9F.
const CP1252_HIGH: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
];

/// Simple per-code-unit lower-casing (single BMP result or unchanged).
pub(crate) fn fold(u: u16) -> u16 {
    map_unit(u, |c| {
        let mut it = c.to_lowercase();
        match (it.next(), it.next()) {
            (Some(l), None) => Some(l),
            _ => None,
        }
    })
}

/// Simple per-code-unit upper-casing (single BMP result or unchanged).
pub(crate) fn upper(u: u16) -> u16 {
    map_unit(u, |c| {
        let mut it = c.to_uppercase();
        match (it.next(), it.next()) {
            (Some(l), None) => Some(l),
            _ => None,
        }
    })
}

fn map_unit(u: u16, f: impl Fn(char) -> Option<char>) -> u16 {
    char::from_u32(u32::from(u))
        .and_then(f)
        .and_then(|c| u16::try_from(u32::from(c)).ok())
        .unwrap_or(u)
}

fn prep(units: &[u16], mode: CompareMode) -> Vec<u16> {
    match mode {
        CompareMode::Binary => units.to_vec(),
        CompareMode::Text => units.iter().map(|&u| fold(u)).collect(),
    }
}

fn find_from(hay: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    if needle.is_empty() {
        return Some(from);
    }
    if from > hay.len() || needle.len() > hay.len() - from {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
}

fn rfind_within(hay: &[u16], needle: &[u16], end: usize) -> Option<usize> {
    if needle.len() > end {
        return None;
    }
    (0..=end - needle.len())
        .rev()
        .find(|&i| hay[i..i + needle.len()] == *needle)
}

fn vs(units: &[u16]) -> VbaString {
    VbaString::from_slice(units)
}

fn to_bytes(s: &VbaString) -> Vec<u8> {
    s.as_units().iter().flat_map(|u| u.to_le_bytes()).collect()
}

fn from_bytes(b: &[u8]) -> StdResult<VbaString> {
    if !b.len().is_multiple_of(2) {
        return Err(StdError::Unsupported(
            "a string with an odd number of bytes cannot be represented as UTF-16",
        ));
    }
    Ok(VbaString::from_units(
        b.chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect(),
    ))
}

/// `Left(s, n)`; negative `n` is error 5.
pub fn left(s: &VbaString, n: i32) -> VbaResult<VbaString> {
    let n = usize::try_from(n).map_err(|_| E5)?;
    Ok(vs(&s.as_units()[..n.min(s.len())]))
}

/// `Right(s, n)`; negative `n` is error 5.
pub fn right(s: &VbaString, n: i32) -> VbaResult<VbaString> {
    let n = usize::try_from(n).map_err(|_| E5)?;
    Ok(vs(&s.as_units()[s.len() - n.min(s.len())..]))
}

/// `Mid(s, start, [length])`; `start < 1` or `length < 0` is error 5.
pub fn mid(s: &VbaString, start: i32, length: Option<i32>) -> VbaResult<VbaString> {
    if start < 1 {
        return Err(E5);
    }
    let len = match length {
        Some(l) => usize::try_from(l).map_err(|_| E5)?,
        None => usize::MAX,
    };
    let from = (start as usize - 1).min(s.len());
    let to = from.saturating_add(len).min(s.len());
    Ok(vs(&s.as_units()[from..to]))
}

/// `LeftB`: counts bytes. A result with an odd byte count is unsupported.
pub fn left_b(s: &VbaString, n: i32) -> StdResult<VbaString> {
    let n = usize::try_from(n).map_err(|_| E5)?;
    let b = to_bytes(s);
    from_bytes(&b[..n.min(b.len())])
}

/// `RightB`: counts bytes. A result with an odd byte count is unsupported.
pub fn right_b(s: &VbaString, n: i32) -> StdResult<VbaString> {
    let n = usize::try_from(n).map_err(|_| E5)?;
    let b = to_bytes(s);
    from_bytes(&b[b.len() - n.min(b.len())..])
}

/// `MidB`: counts bytes. A result with an odd byte count is unsupported.
pub fn mid_b(s: &VbaString, start: i32, length: Option<i32>) -> StdResult<VbaString> {
    if start < 1 {
        return Err(E5.into());
    }
    let len = match length {
        Some(l) => usize::try_from(l).map_err(|_| E5)?,
        None => usize::MAX,
    };
    let b = to_bytes(s);
    let from = (start as usize - 1).min(b.len());
    let to = from.saturating_add(len).min(b.len());
    from_bytes(&b[from..to])
}

/// `InStr(start, hay, needle, compare)` (1-based result, 0 = not found).
pub fn instr(start: i32, hay: &VbaString, needle: &VbaString, mode: CompareMode) -> VbaResult<i32> {
    if start < 1 {
        return Err(E5);
    }
    let h = hay.as_units();
    if h.is_empty() || start as usize > h.len() {
        return Ok(0);
    }
    if needle.is_empty() {
        return Ok(start);
    }
    let (h, n) = (prep(h, mode), prep(needle.as_units(), mode));
    Ok(find_from(&h, &n, start as usize - 1).map_or(0, |i| i as i32 + 1))
}

/// `InStrB`: byte positions, binary comparison only (text comparison on bytes
/// is meaningless and unsupported).
pub fn instr_b(
    start: i32,
    hay: &VbaString,
    needle: &VbaString,
    mode: CompareMode,
) -> StdResult<i32> {
    if start < 1 {
        return Err(E5.into());
    }
    if mode == CompareMode::Text {
        return Err(StdError::Unsupported("InStrB with text comparison"));
    }
    let (h, n) = (to_bytes(hay), to_bytes(needle));
    if h.is_empty() || start as usize > h.len() {
        return Ok(0);
    }
    if n.is_empty() {
        return Ok(start);
    }
    let from = start as usize - 1;
    if n.len() > h.len() - from {
        return Ok(0);
    }
    Ok((from..=h.len() - n.len())
        .find(|&i| h[i..i + n.len()] == n[..])
        .map_or(0, |i| i as i32 + 1))
}

/// `InStrRev(s, find, start, compare)`; `start = -1` means the end of `s`.
pub fn instr_rev(s: &VbaString, find: &VbaString, start: i32, mode: CompareMode) -> VbaResult<i32> {
    if start == 0 || start < -1 {
        return Err(E5);
    }
    let len = s.len();
    let start = if start == -1 { len } else { start as usize };
    if len == 0 || start > len {
        return Ok(0);
    }
    if find.is_empty() {
        return Ok(start as i32);
    }
    let (h, n) = (prep(s.as_units(), mode), prep(find.as_units(), mode));
    Ok(rfind_within(&h, &n, start).map_or(0, |i| i as i32 + 1))
}

/// `Replace(expression, find, replace, start, count, compare)`.
pub fn replace(
    expr: &VbaString,
    find: &VbaString,
    with: &VbaString,
    start: i32,
    count: i32,
    mode: CompareMode,
) -> VbaResult<VbaString> {
    if start < 1 || count < -1 {
        return Err(E5);
    }
    let e = expr.as_units();
    if start as usize > e.len() {
        return Ok(VbaString::new());
    }
    let tail = &e[start as usize - 1..];
    if find.is_empty() || count == 0 {
        return Ok(vs(tail));
    }
    let (h, n) = (prep(tail, mode), prep(find.as_units(), mode));
    let mut out = Vec::with_capacity(tail.len());
    let (mut pos, mut done) = (0usize, 0i32);
    while count == -1 || done < count {
        match find_from(&h, &n, pos) {
            Some(i) => {
                out.extend_from_slice(&tail[pos..i]);
                out.extend_from_slice(with.as_units());
                pos = i + n.len();
                done += 1;
            }
            None => break,
        }
    }
    out.extend_from_slice(&tail[pos..]);
    Ok(VbaString::from_units(out))
}

/// `Space(n)`; negative is error 5, absurd sizes error 7.
pub fn space(n: i32) -> VbaResult<VbaString> {
    let n = usize::try_from(n).map_err(|_| E5)?;
    if n > 1 << 30 {
        return Err(VbaError::OUT_OF_MEMORY);
    }
    Ok(VbaString::from_units(vec![0x20; n]))
}

/// `String(n, character)` with a numeric character code (taken Mod 256 when
/// above 255, as documented).
pub fn string_of_code(n: i32, code: i32) -> VbaResult<VbaString> {
    if code < 0 {
        return Err(E5);
    }
    let ch = chr(code % 256)?;
    string_of_unit(n, ch.as_units()[0])
}

/// `String(n, character)` repeating one UTF-16 code unit.
pub fn string_of_unit(n: i32, unit: u16) -> VbaResult<VbaString> {
    let n = usize::try_from(n).map_err(|_| E5)?;
    if n > 1 << 30 {
        return Err(VbaError::OUT_OF_MEMORY);
    }
    Ok(VbaString::from_units(vec![unit; n]))
}

/// `StrReverse`: reverses UTF-16 code units (surrogate pairs are reversed as
/// two units, as the Windows implementation does; unverified).
pub fn str_reverse(s: &VbaString) -> VbaString {
    let mut u = s.as_units().to_vec();
    u.reverse();
    VbaString::from_units(u)
}

/// `LCase`.
pub fn lcase(s: &VbaString) -> VbaString {
    VbaString::from_units(s.as_units().iter().map(|&u| fold(u)).collect())
}

/// `UCase`.
pub fn ucase(s: &VbaString) -> VbaString {
    VbaString::from_units(s.as_units().iter().map(|&u| upper(u)).collect())
}

/// `LTrim`: removes leading spaces (U+0020 only).
pub fn ltrim(s: &VbaString) -> VbaString {
    let u = s.as_units();
    let n = u.iter().take_while(|&&c| c == 0x20).count();
    vs(&u[n..])
}

/// `RTrim`: removes trailing spaces (U+0020 only).
pub fn rtrim(s: &VbaString) -> VbaString {
    let u = s.as_units();
    let n = u.iter().rev().take_while(|&&c| c == 0x20).count();
    vs(&u[..u.len() - n])
}

/// `Trim`.
pub fn trim(s: &VbaString) -> VbaString {
    ltrim(&rtrim(s))
}

/// `StrComp`: -1, 0 or 1.
pub fn str_comp(a: &VbaString, b: &VbaString, mode: CompareMode) -> i16 {
    match a.compare(b, mode) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// `Asc` (Windows-1252). Characters outside that code page are unsupported
/// (real VBA returns a best-fit or `?` code that depends on the system).
pub fn asc(s: &VbaString) -> StdResult<i16> {
    let u = *s.as_units().first().ok_or(E5)?;
    match u {
        0..=0x7F | 0xA0..=0xFF => Ok(u as i16),
        _ => CP1252_HIGH
            .iter()
            .position(|&c| c == u)
            .map(|i| 0x80 + i as i16)
            .ok_or(StdError::Unsupported(
                "Asc of a character outside Windows-1252",
            )),
    }
}

/// `AscW`: the first UTF-16 code unit as a (possibly negative) `Integer`.
pub fn asc_w(s: &VbaString) -> VbaResult<i16> {
    s.as_units().first().map(|&u| u as i16).ok_or(E5)
}

/// `AscB`: the first byte of the UTF-16LE representation.
pub fn asc_b(s: &VbaString) -> VbaResult<u8> {
    s.as_units().first().map(|&u| u.to_le_bytes()[0]).ok_or(E5)
}

/// `Chr(n)` for 0..=255 (Windows-1252); other values are error 5.
pub fn chr(n: i32) -> VbaResult<VbaString> {
    if !(0..=255).contains(&n) {
        return Err(E5);
    }
    let u = if (0x80..=0x9F).contains(&n) {
        CP1252_HIGH[n as usize - 0x80]
    } else {
        n as u16
    };
    Ok(VbaString::from_units(vec![u]))
}

/// `ChrW(n)` for -32768..=65535.
pub fn chr_w(n: i32) -> VbaResult<VbaString> {
    match n {
        -32768..=-1 => Ok(VbaString::from_units(vec![(n + 65536) as u16])),
        0..=65535 => Ok(VbaString::from_units(vec![n as u16])),
        _ => Err(E5),
    }
}

/// Builds a one-dimensional zero-based `String` array; an empty list becomes
/// an allocated array without elements (`UBound` is -1), as `Split("")` does.
pub fn string_array(items: Vec<VbaString>) -> VbaResult<Variant> {
    if items.is_empty() {
        return Ok(Variant::Array(Box::new(VbaArray::new_empty(
            VarType::String,
            0,
        ))));
    }
    let mut a = VbaArray::new_dynamic(VarType::String);
    a.redim(&[Bounds::new(0, items.len() as i32 - 1)], false)?;
    for (i, s) in items.into_iter().enumerate() {
        a.set(&[i as i32], Variant::String(s))?;
    }
    Ok(Variant::Array(Box::new(a)))
}

/// `Split(expression, delimiter, limit, compare)`.
pub fn split(
    expr: &VbaString,
    delim: &VbaString,
    limit: i32,
    mode: CompareMode,
) -> VbaResult<Vec<VbaString>> {
    if limit < -1 {
        return Err(E5);
    }
    if expr.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    if delim.is_empty() || limit == 1 {
        return Ok(vec![expr.clone()]);
    }
    let (h, d) = (prep(expr.as_units(), mode), prep(delim.as_units(), mode));
    let e = expr.as_units();
    let mut out = Vec::new();
    let mut pos = 0;
    while limit == -1 || (out.len() as i32) < limit - 1 {
        match find_from(&h, &d, pos) {
            Some(i) => {
                out.push(vs(&e[pos..i]));
                pos = i + d.len();
            }
            None => break,
        }
    }
    out.push(vs(&e[pos..]));
    Ok(out)
}

fn string_items(v: &Variant) -> VbaResult<Vec<VbaString>> {
    let Variant::Array(a) = v else {
        return Err(E5);
    };
    if a.dimensions() != 1 {
        return Err(E5);
    }
    a.iter().map(convert::to_vba_string).collect()
}

/// `Join(sourceArray, delimiter)`; a non-array or multi-dimensional array is
/// error 5, a `Null` element error 94.
pub fn join(source: &Variant, delim: &VbaString) -> VbaResult<VbaString> {
    let items = string_items(source)?;
    let mut out = Vec::new();
    for (i, s) in items.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(delim.as_units());
        }
        out.extend_from_slice(s.as_units());
    }
    Ok(VbaString::from_units(out))
}

/// `Filter(source, match, include, compare)`.
pub fn filter(
    source: &Variant,
    pattern: &VbaString,
    include: bool,
    mode: CompareMode,
) -> VbaResult<Vec<VbaString>> {
    let items = string_items(source)?;
    let p = prep(pattern.as_units(), mode);
    Ok(items
        .into_iter()
        .filter(|s| find_from(&prep(s.as_units(), mode), &p, 0).is_some() == include)
        .collect())
}

/// `StrConv` conversion flags.
pub mod conv {
    /// `vbUpperCase`.
    pub const UPPER: i32 = 1;
    /// `vbLowerCase`.
    pub const LOWER: i32 = 2;
    /// `vbProperCase`.
    pub const PROPER: i32 = 3;
}

/// `StrConv(s, conversion)`. Upper, lower and proper case are implemented;
/// the wide/narrow/katakana/hiragana/Unicode/FromUnicode modes depend on
/// Windows code pages and are unsupported.
pub fn str_conv(s: &VbaString, conversion: i32) -> StdResult<VbaString> {
    match conversion {
        0 => Ok(s.clone()),
        conv::UPPER => Ok(ucase(s)),
        conv::LOWER => Ok(lcase(s)),
        conv::PROPER => {
            let mut at_start = true;
            let units = s
                .as_units()
                .iter()
                .map(|&u| {
                    let c = char::from_u32(u32::from(u));
                    let word = c.is_some_and(|c| c.is_alphanumeric() || c == '\'');
                    let r = if at_start { upper(u) } else { fold(u) };
                    at_start = !word;
                    r
                })
                .collect();
            Ok(VbaString::from_units(units))
        }
        4 | 8 | 16 | 32 | 64 | 128 | 1024 | 2048 => Err(StdError::Unsupported(
            "StrConv modes that depend on Windows code pages or LCIDs",
        )),
        c if c > 0 && c & !(3 | 4 | 8 | 16 | 32 | 64 | 128 | 1024 | 2048) == 0 => Err(
            StdError::Unsupported("StrConv combinations that depend on Windows code pages"),
        ),
        _ => Err(E5.into()),
    }
}

// ---------------------------------------------------------------------------
// Registry adapters.

/// Reads string argument `i`; `Null` yields `Ok(None)` (the caller returns
/// `Null`) unless the `$` form was used, which makes it error 94.
fn nullable(a: &Args<'_>, i: usize) -> StdResult<Option<VbaString>> {
    match a.req(i)? {
        Variant::Null if !a.dollar => Ok(None),
        v => Ok(Some(convert::to_vba_string(v)?)),
    }
}

macro_rules! null_guard {
    ($a:expr, $($i:expr),+) => {
        for i in [$($i),+] {
            if $a.is_null(i) {
                if $a.dollar { return Err(VbaError::INVALID_USE_OF_NULL.into()); }
                return Ok(Variant::Null);
            }
        }
    };
}

fn mode_arg(ctx: &Ctx<'_>, a: &Args<'_>, i: usize) -> StdResult<CompareMode> {
    let v = match a.get(i) {
        Some(v) => Some(convert::to_long(v)?),
        None => None,
    };
    Ok(compare_arg(v, ctx.compare)?)
}

pub(crate) fn f_left(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    Ok(text(left(&s, a.long(1)?)?))
}
pub(crate) fn f_right(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    Ok(text(right(&s, a.long(1)?)?))
}
pub(crate) fn f_mid(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    let len = a.get(2).map(convert::to_long).transpose()?;
    Ok(text(mid(&s, a.long(1)?, len)?))
}
pub(crate) fn f_left_b(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    Ok(text(left_b(&s, a.long(1)?)?))
}
pub(crate) fn f_right_b(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    Ok(text(right_b(&s, a.long(1)?)?))
}
pub(crate) fn f_mid_b(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    let len = a.get(2).map(convert::to_long).transpose()?;
    Ok(text(mid_b(&s, a.long(1)?, len)?))
}

/// `InStr` overloads: `(s1, s2)`, `(start, s1, s2)`, `(start, s1, s2, cmp)`;
/// in the four-slot form (as a compiler binds it) `start` and `cmp` may be
/// omitted.
fn instr_args<'a>(a: &Args<'a>) -> StdResult<(i32, usize, Option<usize>)> {
    match a.len() {
        2 => Ok((1, 0, None)),
        3 => Ok((a.long_or(0, 1)?, 1, None)),
        4 => Ok((a.long_or(0, 1)?, 1, a.get(3).map(|_| 3))),
        _ => Err(crate::error::WRONG_ARGUMENT_COUNT.into()),
    }
}

pub(crate) fn f_instr(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let (start, i1, cmp) = instr_args(a)?;
    null_guard!(a, i1, i1 + 1);
    let mode = match cmp {
        Some(i) => mode_arg(ctx, a, i)?,
        None => ctx.compare,
    };
    Ok(Variant::Long(instr(
        start,
        &a.string(i1)?,
        &a.string(i1 + 1)?,
        mode,
    )?))
}
pub(crate) fn f_instr_b(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let (start, i1, cmp) = instr_args(a)?;
    null_guard!(a, i1, i1 + 1);
    let mode = match cmp {
        Some(i) => mode_arg(ctx, a, i)?,
        None => ctx.compare,
    };
    Ok(Variant::Long(instr_b(
        start,
        &a.string(i1)?,
        &a.string(i1 + 1)?,
        mode,
    )?))
}
pub(crate) fn f_instr_rev(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 0, 1);
    let mode = mode_arg(ctx, a, 3)?;
    let mode = if a.get(3).is_none() {
        CompareMode::Binary
    } else {
        mode
    };
    Ok(Variant::Long(instr_rev(
        &a.string(0)?,
        &a.string(1)?,
        a.long_or(2, -1)?,
        mode,
    )?))
}
pub(crate) fn f_replace(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    // Replace returns a String, so Null arguments are error 94.
    let mode = if a.get(5).is_none() {
        ctx.compare
    } else {
        mode_arg(ctx, a, 5)?
    };
    Ok(text(replace(
        &a.string(0)?,
        &a.string(1)?,
        &a.string(2)?,
        a.long_or(3, 1)?,
        a.long_or(4, -1)?,
        mode,
    )?))
}
pub(crate) fn f_space(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 0);
    Ok(text(space(a.long(0)?)?))
}
pub(crate) fn f_string(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) || a.is_null(1) {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    let n = a.long(0)?;
    match a.req(1)? {
        Variant::String(s) => {
            let u = *s.as_units().first().ok_or(E5)?;
            Ok(text(string_of_unit(n, u)?))
        }
        v => Ok(text(string_of_code(n, convert::to_long(v)?)?)),
    }
}
pub(crate) fn f_str_reverse(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(text(str_reverse(&a.string(0)?)))
}
pub(crate) fn f_lcase(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    Ok(text(lcase(&s)))
}
pub(crate) fn f_ucase(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    Ok(text(ucase(&s)))
}
pub(crate) fn f_ltrim(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    Ok(text(ltrim(&s)))
}
pub(crate) fn f_rtrim(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    Ok(text(rtrim(&s)))
}
pub(crate) fn f_trim(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let Some(s) = nullable(a, 0)? else {
        return Ok(Variant::Null);
    };
    Ok(text(trim(&s)))
}
pub(crate) fn f_str_comp(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let mode = mode_arg(ctx, a, 2)?;
    null_guard!(a, 0, 1);
    Ok(Variant::Integer(str_comp(
        &a.string(0)?,
        &a.string(1)?,
        mode,
    )))
}
pub(crate) fn f_str_conv(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 0);
    Ok(text(str_conv(&a.string(0)?, a.long(1)?)?))
}
pub(crate) fn f_asc(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    Ok(Variant::Integer(asc(&a.string(0)?)?))
}
pub(crate) fn f_asc_w(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    Ok(Variant::Integer(asc_w(&a.string(0)?)?))
}
pub(crate) fn f_asc_b(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    Ok(Variant::Integer(i16::from(asc_b(&a.string(0)?)?)))
}
pub(crate) fn f_chr(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    Ok(text(chr(a.long(0)?)?))
}
pub(crate) fn f_chr_w(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    Ok(text(chr_w(a.long(0)?)?))
}
pub(crate) fn f_chr_b(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let n = a.long(0)?;
    if !(0..=255).contains(&n) {
        return Err(E5.into());
    }
    Err(StdError::Unsupported(
        "ChrB yields a one-byte string, which UTF-16 strings cannot represent",
    ))
}
pub(crate) fn f_split(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let delim = match a.get(1) {
        Some(v) => convert::to_vba_string(v)?,
        None => VbaString::from(" "),
    };
    let mode = if a.get(3).is_none() {
        ctx.compare
    } else {
        mode_arg(ctx, a, 3)?
    };
    let parts = split(&a.string(0)?, &delim, a.long_or(2, -1)?, mode)?;
    Ok(string_array(parts)?)
}
pub(crate) fn f_join(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let delim = match a.get(1) {
        Some(v) => convert::to_vba_string(v)?,
        None => VbaString::from(" "),
    };
    Ok(text(join(a.req(0)?, &delim)?))
}
pub(crate) fn f_filter(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let mode = if a.get(3).is_none() {
        ctx.compare
    } else {
        mode_arg(ctx, a, 3)?
    };
    let r = filter(a.req(0)?, &a.string(1)?, a.bool_or(2, true)?, mode)?;
    Ok(string_array(r)?)
}
pub(crate) fn f_len(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Ok(Variant::Null);
    }
    Ok(Variant::Long(a.string(0)?.len() as i32))
}
pub(crate) fn f_len_b(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let n = match a.req(0)? {
        Variant::Null => return Ok(Variant::Null),
        Variant::String(s) => s.len_bytes(),
        Variant::Empty => 0,
        Variant::Boolean(_) | Variant::Integer(_) => 2,
        Variant::Byte(_) => 1,
        Variant::Long(_) | Variant::Single(_) => 4,
        Variant::LongLong(_) | Variant::Double(_) | Variant::Currency(_) | Variant::Date(_) => 8,
        Variant::Decimal(_) => 16,
        _ => return Err(VbaError::TYPE_MISMATCH.into()),
    };
    Ok(Variant::Long(n as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> VbaString {
        VbaString::from(x)
    }

    #[test]
    fn instr_basics() {
        let b = CompareMode::Binary;
        assert_eq!(instr(1, &s("abcabc"), &s("c"), b), Ok(3));
        assert_eq!(instr(4, &s("abcabc"), &s("c"), b), Ok(6));
        assert_eq!(instr(1, &s("abc"), &s("B"), b), Ok(0));
        assert_eq!(instr(1, &s("abc"), &s("B"), CompareMode::Text), Ok(2));
        assert_eq!(instr(2, &s("abc"), &s(""), b), Ok(2));
        assert_eq!(instr(1, &s(""), &s(""), b), Ok(0));
        assert_eq!(instr(0, &s("a"), &s("a"), b), Err(E5));
    }

    #[test]
    fn replace_rules() {
        let b = CompareMode::Binary;
        let r = |e, f, w, st, c| {
            replace(&s(e), &s(f), &s(w), st, c, b)
                .unwrap()
                .to_string_lossy()
        };
        assert_eq!(r("banana", "an", "AN", 1, -1), "bANANa");
        assert_eq!(r("banana", "an", "AN", 1, 1), "bANana");
        assert_eq!(r("banana", "an", "AN", 3, -1), "nANa");
        assert_eq!(r("banana", "", "x", 1, -1), "banana");
        assert_eq!(r("banana", "a", "x", 7, -1), "");
    }
}
