//! `VBA.Strings` (MS-VBAL §6.1.2.11 and the Microsoft VBA language
//! reference). Typed functions come first, the registry adapters at the end.
//!
//! Provenance: argument ranges, result types and error numbers follow the
//! Microsoft language reference (**doc**); MS-VBAL only lists these functions
//! by signature (**spec**). Rules marked "unverified" in `docs/language/vba-stdlib.md`
//! have not been compared against a real host.

use crate::call::{Args, Ctx, compare_arg, text};
use crate::error::{StdError, StdResult};
use vba_value::collation::{Collation, EN_US};
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

fn prep(units: &[u16], mode: CompareMode, col: &dyn Collation) -> Vec<u16> {
    match mode {
        CompareMode::Binary => units.to_vec(),
        CompareMode::Text => col.fold(units),
    }
}

/// Needles up to this long are searched with the plain loop (at most this
/// many comparisons per position); longer ones with a linear-time search.
const NAIVE_NEEDLE: usize = 4;

/// The border table of `needle` (Knuth-Morris-Pratt): `t[i]` is the length of
/// the longest proper prefix of `needle[..=i]` that is also its suffix.
fn borders(needle: &[u16]) -> Vec<u32> {
    let mut t = vec![0u32; needle.len()];
    let mut k = 0usize;
    for i in 1..needle.len() {
        while k > 0 && needle[i] != needle[k] {
            k = t[k - 1] as usize;
        }
        if needle[i] == needle[k] {
            k += 1;
        }
        t[i] = k as u32;
    }
    t
}

/// The first occurrence of `needle` in `hay` at or after `from`. Linear in
/// `hay.len() + needle.len()` whatever the content, so the step budget can
/// charge for it up front.
fn find_from(hay: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    if needle.is_empty() {
        return Some(from);
    }
    if from > hay.len() || needle.len() > hay.len() - from {
        return None;
    }
    if needle.len() <= NAIVE_NEEDLE {
        return (from..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle);
    }
    let t = borders(needle);
    let mut k = 0usize;
    for (i, &c) in hay.iter().enumerate().skip(from) {
        while k > 0 && c != needle[k] {
            k = t[k - 1] as usize;
        }
        if c == needle[k] {
            k += 1;
        }
        if k == needle.len() {
            return Some(i + 1 - k);
        }
    }
    None
}

/// The last occurrence of `needle` that ends at or before `end`. Linear, like
/// [`find_from`].
fn rfind_within(hay: &[u16], needle: &[u16], end: usize) -> Option<usize> {
    if needle.len() > end {
        return None;
    }
    if needle.len() <= NAIVE_NEEDLE {
        return (0..=end - needle.len())
            .rev()
            .find(|&i| hay[i..i + needle.len()] == *needle);
    }
    let rev: Vec<u16> = needle.iter().rev().copied().collect();
    let t = borders(&rev);
    let mut k = 0usize;
    for j in (0..end).rev() {
        let c = hay[j];
        while k > 0 && c != rev[k] {
            k = t[k - 1] as usize;
        }
        if c == rev[k] {
            k += 1;
        }
        if k == rev.len() {
            return Some(j);
        }
    }
    None
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

/// `InStr(start, hay, needle, compare)` (1-based result, 0 = not found),
/// with the default en-US text collation.
pub fn instr(start: i32, hay: &VbaString, needle: &VbaString, mode: CompareMode) -> VbaResult<i32> {
    instr_with(start, hay, needle, mode, &EN_US)
}

/// [`instr`] with a caller-supplied text collation.
pub fn instr_with(
    start: i32,
    hay: &VbaString,
    needle: &VbaString,
    mode: CompareMode,
    col: &dyn Collation,
) -> VbaResult<i32> {
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
    let (h, n) = (prep(h, mode, col), prep(needle.as_units(), mode, col));
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
    instr_rev_with(s, find, start, mode, &EN_US)
}

/// [`instr_rev`] with a caller-supplied text collation.
pub fn instr_rev_with(
    s: &VbaString,
    find: &VbaString,
    start: i32,
    mode: CompareMode,
    col: &dyn Collation,
) -> VbaResult<i32> {
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
    let (h, n) = (
        prep(s.as_units(), mode, col),
        prep(find.as_units(), mode, col),
    );
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
    replace_with(expr, find, with, start, count, mode, &EN_US)
}

/// [`replace`] with a caller-supplied text collation.
pub fn replace_with(
    expr: &VbaString,
    find: &VbaString,
    with: &VbaString,
    start: i32,
    count: i32,
    mode: CompareMode,
    col: &dyn Collation,
) -> VbaResult<VbaString> {
    replace_bounded(expr, find, with, start, count, mode, col, usize::MAX)
}

/// [`replace_with`] that fails with error 7 as soon as the result would
/// exceed `max_units`, before the units are allocated.
#[allow(clippy::too_many_arguments)]
pub fn replace_bounded(
    expr: &VbaString,
    find: &VbaString,
    with: &VbaString,
    start: i32,
    count: i32,
    mode: CompareMode,
    col: &dyn Collation,
    max_units: usize,
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
    let (h, n) = (prep(tail, mode, col), prep(find.as_units(), mode, col));
    let mut out = Vec::with_capacity(tail.len());
    let (mut pos, mut done) = (0usize, 0i32);
    while count == -1 || done < count {
        match find_from(&h, &n, pos) {
            Some(i) => {
                let grown = out.len().saturating_add(i - pos).saturating_add(with.len());
                if grown > max_units {
                    return Err(VbaError::OUT_OF_MEMORY);
                }
                out.extend_from_slice(&tail[pos..i]);
                out.extend_from_slice(with.as_units());
                pos = i + n.len();
                done += 1;
            }
            None => break,
        }
    }
    if out.len().saturating_add(tail.len() - pos) > max_units {
        return Err(VbaError::OUT_OF_MEMORY);
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
    str_comp_with(a, b, mode, &EN_US)
}

/// [`str_comp`] with a caller-supplied text collation.
pub fn str_comp_with(a: &VbaString, b: &VbaString, mode: CompareMode, col: &dyn Collation) -> i16 {
    let ord = match mode {
        CompareMode::Binary => a.compare(b, mode),
        CompareMode::Text => a.compare_with(b, col),
    };
    match ord {
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
    split_with(expr, delim, limit, mode, &EN_US)
}

/// [`split`] with a caller-supplied text collation.
pub fn split_with(
    expr: &VbaString,
    delim: &VbaString,
    limit: i32,
    mode: CompareMode,
    col: &dyn Collation,
) -> VbaResult<Vec<VbaString>> {
    split_bounded(expr, delim, limit, mode, col, usize::MAX)
}

/// [`split_with`] that fails with error 7 when more than `max_parts` pieces
/// would be produced.
pub fn split_bounded(
    expr: &VbaString,
    delim: &VbaString,
    limit: i32,
    mode: CompareMode,
    col: &dyn Collation,
    max_parts: usize,
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
    let (h, d) = (
        prep(expr.as_units(), mode, col),
        prep(delim.as_units(), mode, col),
    );
    let e = expr.as_units();
    let mut out = Vec::new();
    let mut pos = 0;
    while limit == -1 || (out.len() as i32) < limit - 1 {
        match find_from(&h, &d, pos) {
            Some(i) => {
                if out.len() >= max_parts {
                    return Err(VbaError::OUT_OF_MEMORY);
                }
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
    join_bounded(source, delim, usize::MAX)
}

/// [`join`] that fails with error 7 when the result would exceed
/// `max_units`, before it is allocated.
pub fn join_bounded(source: &Variant, delim: &VbaString, max_units: usize) -> VbaResult<VbaString> {
    let items = string_items(source)?;
    let mut total = 0usize;
    for (i, s) in items.iter().enumerate() {
        if i > 0 {
            total = total.saturating_add(delim.len());
        }
        total = total.saturating_add(s.len());
        if total > max_units {
            return Err(VbaError::OUT_OF_MEMORY);
        }
    }
    let mut out = Vec::with_capacity(total);
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
    filter_with(source, pattern, include, mode, &EN_US)
}

/// [`filter`] with a caller-supplied text collation.
pub fn filter_with(
    source: &Variant,
    pattern: &VbaString,
    include: bool,
    mode: CompareMode,
    col: &dyn Collation,
) -> VbaResult<Vec<VbaString>> {
    let items = string_items(source)?;
    let p = prep(pattern.as_units(), mode, col);
    Ok(items
        .into_iter()
        .filter(|s| find_from(&prep(s.as_units(), mode, col), &p, 0).is_some() == include)
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

/// `StrConv` flag bits not covered by [`conv`].
mod flag {
    pub const WIDE: i32 = 4;
    pub const NARROW: i32 = 8;
    pub const KATAKANA: i32 = 16;
    pub const HIRAGANA: i32 = 32;
    pub const UNICODE: i32 = 64;
    pub const FROM_UNICODE: i32 = 128;
    pub const SIMPLIFIED: i32 = 256;
    pub const TRADITIONAL: i32 = 512;
    pub const LINGUISTIC: i32 = 1024;
}

/// `StrConv(s, conversion)` with the Windows-1252 code page for
/// `vbUnicode`/`vbFromUnicode`; see [`str_conv_cp`].
pub fn str_conv(s: &VbaString, conversion: i32) -> StdResult<VbaString> {
    str_conv_cp(s, conversion, 1252)
}

/// `StrConv(s, conversion)`.
///
/// Implemented: `vbUpperCase`, `vbLowerCase`, `vbProperCase`, `vbWide`,
/// `vbNarrow`, `vbKatakana`, `vbHiragana` (width and kana first, then case;
/// `vbWide`/`vbNarrow` and `vbKatakana`/`vbHiragana` are mutually exclusive,
/// else error 5), and `vbUnicode`/`vbFromUnicode` through `code_page`
/// (`encoding_rs` pages only, see [`crate::textconv::encoding_for`]; other
/// pages are [`StdError::Unsupported`]). `vbUnicode`/`vbFromUnicode` cannot be
/// combined with other flags (error 5, **unverified**).
///
/// `vbFromUnicode` yields the code page bytes packed two per `UTF-16` unit
/// (little endian), which is how a VBA string holds them. A result with an odd
/// number of bytes cannot be represented and is `Unsupported`.
/// `vbSimplifiedChinese` and `vbTraditionalChinese` are `Unsupported`;
/// `vbLinguisticCasing` is accepted with `vbUpperCase`/`vbLowerCase` only
/// (error 5 otherwise) and changes nothing for en-US; other bits are error 5.
pub fn str_conv_cp(s: &VbaString, conversion: i32, code_page: u32) -> StdResult<VbaString> {
    const KNOWN: i32 = 3
        | flag::WIDE
        | flag::NARROW
        | flag::KATAKANA
        | flag::HIRAGANA
        | flag::UNICODE
        | flag::FROM_UNICODE
        | flag::SIMPLIFIED
        | flag::TRADITIONAL
        | flag::LINGUISTIC;
    if conversion < 0 || conversion & !KNOWN != 0 {
        return Err(E5.into());
    }
    let has = |f: i32| conversion & f != 0;
    if (has(flag::WIDE) && has(flag::NARROW))
        || (has(flag::KATAKANA) && has(flag::HIRAGANA))
        || (has(flag::SIMPLIFIED) && has(flag::TRADITIONAL))
    {
        return Err(E5.into());
    }
    if has(flag::UNICODE) || has(flag::FROM_UNICODE) {
        if conversion & !(flag::UNICODE | flag::FROM_UNICODE) != 0
            || (has(flag::UNICODE) && has(flag::FROM_UNICODE))
        {
            return Err(E5.into());
        }
        let bytes: Vec<u8> = s.as_units().iter().flat_map(|u| u.to_le_bytes()).collect();
        if has(flag::FROM_UNICODE) {
            let enc = crate::textconv::encode(s, code_page)?;
            return from_bytes(&enc);
        }
        return crate::textconv::decode(&bytes, code_page);
    }
    if has(flag::SIMPLIFIED) || has(flag::TRADITIONAL) {
        return Err(StdError::Unsupported(
            "StrConv vbSimplifiedChinese/vbTraditionalChinese",
        ));
    }
    if has(flag::LINGUISTIC) && !matches!(conversion & 3, conv::UPPER | conv::LOWER) {
        // Valid with vbUpperCase and vbLowerCase only. For the en-US locale
        // the linguistic rules equal the plain conversion.
        return Err(E5.into());
    }
    let mut r = s.clone();
    if has(flag::WIDE) {
        r = crate::textconv::to_wide(&r);
    }
    if has(flag::NARROW) {
        r = crate::textconv::to_narrow(&r);
    }
    if has(flag::KATAKANA) {
        r = crate::textconv::to_katakana(&r);
    }
    if has(flag::HIRAGANA) {
        r = crate::textconv::to_hiragana(&r);
    }
    match conversion & 3 {
        0 => Ok(r),
        conv::UPPER => Ok(ucase(&r)),
        conv::LOWER => Ok(lcase(&r)),
        _ => {
            let mut at_start = true;
            let units = r
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
    }
}

/// `Asc` in an arbitrary supported code page: the first character encoded in
/// `code_page`; a double-byte character yields `lead * 256 + trail` as a
/// signed `Integer`. Unmappable characters are `Unsupported` (real VBA returns
/// an environment-dependent substitute).
pub fn asc_cp(s: &VbaString, code_page: u32) -> StdResult<i16> {
    if code_page == 1252 {
        return asc(s);
    }
    let first = char::decode_utf16(s.as_units().iter().copied())
        .next()
        .ok_or(E5)?
        .map_err(|_| StdError::Unsupported("Asc of an unpaired surrogate"))?;
    let enc = crate::textconv::encoding_for(code_page).ok_or(StdError::Unsupported(
        "code page not supported by encoding_rs",
    ))?;
    let mut buf = [0u8; 4];
    let (bytes, _, bad) = enc.encode(first.encode_utf8(&mut buf));
    if bad {
        return Err(StdError::Unsupported(
            "Asc of a character outside the code page",
        ));
    }
    match bytes.as_ref() {
        [b] => Ok(i16::from(*b)),
        [l, t] => Ok(((u16::from(*l) << 8) | u16::from(*t)) as i16),
        _ => Err(StdError::Unsupported("Asc of a multi-byte character")),
    }
}

/// `Chr` in a supported code page. 0..=255 is one byte; 256..=65535 is a
/// lead/trail byte pair and only valid for double-byte pages (932, 936, 949,
/// 950); anything undecodable is error 5.
pub fn chr_cp(n: i32, code_page: u32) -> StdResult<VbaString> {
    if code_page == 1252 {
        return Ok(chr(n)?);
    }
    let bytes: Vec<u8> = match n {
        0..=255 => vec![n as u8],
        256..=65535 if crate::textconv::is_double_byte(code_page) => {
            vec![(n >> 8) as u8, n as u8]
        }
        _ => return Err(E5.into()),
    };
    let enc = crate::textconv::encoding_for(code_page).ok_or(StdError::Unsupported(
        "code page not supported by encoding_rs",
    ))?;
    let (text, bad) = enc
        .decode_without_bom_handling_and_without_replacement(&bytes)
        .map_or_else(|| (std::borrow::Cow::Borrowed(""), true), |t| (t, false));
    if bad || text.is_empty() {
        return Err(E5.into());
    }
    Ok(VbaString::from(text.as_ref()))
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
    Ok(Variant::Long(instr_with(
        start,
        &a.string(i1)?,
        &a.string(i1 + 1)?,
        mode,
        ctx.host.collation(),
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
    Ok(Variant::Long(instr_rev_with(
        &a.string(0)?,
        &a.string(1)?,
        a.long_or(2, -1)?,
        mode,
        ctx.host.collation(),
    )?))
}
pub(crate) fn f_replace(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    // Replace returns a String, so Null arguments are error 94.
    let mode = if a.get(5).is_none() {
        ctx.compare
    } else {
        mode_arg(ctx, a, 5)?
    };
    Ok(text(replace_bounded(
        &a.string(0)?,
        &a.string(1)?,
        &a.string(2)?,
        a.long_or(3, 1)?,
        a.long_or(4, -1)?,
        mode,
        ctx.host.collation(),
        ctx.max_string_units,
    )?))
}
pub(crate) fn f_space(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 0);
    let n = a.long(0)?;
    if usize::try_from(n).is_ok_and(|n| n > ctx.max_string_units) {
        return Err(VbaError::OUT_OF_MEMORY.into());
    }
    Ok(text(space(n)?))
}
pub(crate) fn f_string(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) || a.is_null(1) {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    let n = a.long(0)?;
    if usize::try_from(n).is_ok_and(|n| n > ctx.max_string_units) {
        return Err(VbaError::OUT_OF_MEMORY.into());
    }
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
    Ok(Variant::Integer(str_comp_with(
        &a.string(0)?,
        &a.string(1)?,
        mode,
        ctx.host.collation(),
    )))
}
pub(crate) fn f_str_conv(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    null_guard!(a, 0);
    let page = match a.long_or(2, 0)? {
        0 => ctx.host.locale().ansi_code_page,
        lcid => crate::textconv::lcid_code_page(lcid).ok_or(StdError::Unsupported(
            "StrConv LocaleID without a known code page",
        ))?,
    };
    // A `Byte` array argument is read as the string holding those bytes.
    let input = match a.req(0)? {
        Variant::Array(arr) if arr.to_bytes().is_some() => {
            from_bytes(&arr.to_bytes().unwrap_or_default())?
        }
        _ => a.string(0)?,
    };
    Ok(text(str_conv_cp(&input, a.long(1)?, page)?))
}
pub(crate) fn f_asc(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    Ok(Variant::Integer(asc_cp(
        &a.string(0)?,
        ctx.host.locale().ansi_code_page,
    )?))
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
pub(crate) fn f_chr(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    if a.is_null(0) {
        return Err(VbaError::INVALID_USE_OF_NULL.into());
    }
    Ok(text(chr_cp(a.long(0)?, ctx.host.locale().ansi_code_page)?))
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
    let parts = split_bounded(
        &a.string(0)?,
        &delim,
        a.long_or(2, -1)?,
        mode,
        ctx.host.collation(),
        ctx.max_array_elements,
    )?;
    Ok(string_array(parts)?)
}
pub(crate) fn f_join(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let delim = match a.get(1) {
        Some(v) => convert::to_vba_string(v)?,
        None => VbaString::from(" "),
    };
    Ok(text(join_bounded(a.req(0)?, &delim, ctx.max_string_units)?))
}
pub(crate) fn f_filter(ctx: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    let mode = if a.get(3).is_none() {
        ctx.compare
    } else {
        mode_arg(ctx, a, 3)?
    };
    let r = filter_with(
        a.req(0)?,
        &a.string(1)?,
        a.bool_or(2, true)?,
        mode,
        ctx.host.collation(),
    )?;
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
mod search_tests {
    use super::*;

    fn u(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn linear_search_agrees_with_the_plain_loop() {
        let alphabet = [u("a"), u("b")];
        let texts: Vec<Vec<u16>> = (0..=9u32)
            .flat_map(|n| {
                (0..(1u32 << n)).map(move |bits| {
                    (0..n)
                        .map(|i| {
                            if bits >> i & 1 == 1 {
                                'a' as u16
                            } else {
                                'b' as u16
                            }
                        })
                        .collect::<Vec<u16>>()
                })
            })
            .collect();
        let _ = alphabet;
        for hay in texts.iter().filter(|t| t.len() <= 9) {
            for needle in texts.iter().filter(|n| (1..=7).contains(&n.len())) {
                for from in 0..=hay.len() {
                    let naive = if needle.len() > hay.len() - from {
                        None
                    } else {
                        (from..=hay.len() - needle.len())
                            .find(|&i| hay[i..i + needle.len()] == needle[..])
                    };
                    assert_eq!(find_from(hay, needle, from), naive, "{hay:?} {needle:?}");
                }
                let end = hay.len();
                let naive = if needle.len() > end {
                    None
                } else {
                    (0..=end - needle.len())
                        .rev()
                        .find(|&i| hay[i..i + needle.len()] == needle[..])
                };
                assert_eq!(rfind_within(hay, needle, end), naive, "{hay:?} {needle:?}");
            }
        }
    }

    #[test]
    fn a_long_needle_in_a_long_text_is_fast() {
        let hay = vec![0x61u16; 2_000_000];
        let mut needle = vec![0x61u16; 20_000];
        needle.push(0x62);
        let t = std::time::Instant::now();
        assert_eq!(find_from(&hay, &needle, 0), None);
        assert_eq!(rfind_within(&hay, &needle, hay.len()), None);
        assert!(t.elapsed().as_secs() < 2, "{:?}", t.elapsed());
    }
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
