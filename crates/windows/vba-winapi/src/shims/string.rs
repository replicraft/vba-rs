//! String helpers: lstrlen, lstrcpy, lstrcat, lstrcmp, lstrcmpi (kernel32) and
//! CharUpper, CharLower (user32), each in ANSI and Unicode form.
//!
//! Docs: learn.microsoft.com/windows/win32/api/winbase/nf-winbase-lstrlena,
//! ...-lstrcpya, ...-lstrcata, ...-lstrcmpa, ...-lstrcmpia and
//! .../winuser/nf-winuser-charuppera, ...-charlowera (and the W forms).

use super::{Args, Shim};
use crate::{CallContext, WinApiError};
use std::cmp::Ordering;
use vba_ffi::{Codepage, NativeReturn, Unmappable};

macro_rules! shim {
    ($dll:literal, $sym:literal, $n:literal, $f:expr) => {
        Shim {
            dll: $dll,
            symbol: $sym,
            arity: $n,
            run: $f,
        }
    };
}

pub(super) const SHIMS: [Shim; 14] = [
    shim!("kernel32", "lstrlenA", 1, |c, a| lstrlen(c, a, false)),
    shim!("kernel32", "lstrlenW", 1, |c, a| lstrlen(c, a, true)),
    shim!("kernel32", "lstrcpyA", 2, |c, a| lstrcpy(c, a, false)),
    shim!("kernel32", "lstrcpyW", 2, |c, a| lstrcpy(c, a, true)),
    shim!("kernel32", "lstrcatA", 2, |c, a| lstrcat(c, a, false)),
    shim!("kernel32", "lstrcatW", 2, |c, a| lstrcat(c, a, true)),
    shim!("kernel32", "lstrcmpA", 2, |c, a| lstrcmp(
        c, a, false, false
    )),
    shim!("kernel32", "lstrcmpW", 2, |c, a| lstrcmp(c, a, true, false)),
    shim!("kernel32", "lstrcmpiA", 2, |c, a| lstrcmp(
        c, a, false, true
    )),
    shim!("kernel32", "lstrcmpiW", 2, |c, a| lstrcmp(c, a, true, true)),
    shim!("user32", "CharUpperA", 1, |c, a| char_case(
        c, a, false, true
    )),
    shim!("user32", "CharUpperW", 1, |c, a| char_case(
        c, a, true, true
    )),
    shim!("user32", "CharLowerA", 1, |c, a| char_case(
        c, a, false, false
    )),
    shim!("user32", "CharLowerW", 1, |c, a| char_case(
        c, a, true, false
    )),
];

/// Raw bytes of the NUL-terminated string at `p` (without the terminator).
fn read_raw(ctx: &CallContext, p: u64, wide: bool) -> Result<Vec<u8>, WinApiError> {
    Ok(if wide {
        ctx.memory
            .read_wstr(p)?
            .iter()
            .flat_map(|u| u.to_le_bytes())
            .collect()
    } else {
        ctx.memory.read_cstr(p)?
    })
}

fn write_terminated(
    ctx: &mut CallContext,
    at: u64,
    raw: &[u8],
    wide: bool,
) -> Result<(), WinApiError> {
    let mut v = raw.to_vec();
    v.extend_from_slice(if wide { &[0, 0] } else { &[0] });
    ctx.memory.write(at, &v)?;
    Ok(())
}

/// Returns the length in characters; a NULL pointer gives 0 (documented).
fn lstrlen(ctx: &mut CallContext, a: &Args, wide: bool) -> Result<NativeReturn, WinApiError> {
    let p = a.ptr(0)?;
    if p == 0 {
        return Ok(NativeReturn::Int(0));
    }
    let n = read_raw(ctx, p, wide)?.len() / if wide { 2 } else { 1 };
    Ok(NativeReturn::Int(n as i64))
}

/// Copies including the terminator; returns the destination pointer, or NULL
/// when either pointer is NULL (documented failure value). An undersized
/// destination is an emulated memory fault instead of a buffer overrun.
fn lstrcpy(ctx: &mut CallContext, a: &Args, wide: bool) -> Result<NativeReturn, WinApiError> {
    let (d, s) = (a.ptr(0)?, a.ptr(1)?);
    if d == 0 || s == 0 {
        return Ok(NativeReturn::Int(0));
    }
    let raw = read_raw(ctx, s, wide)?;
    write_terminated(ctx, d, &raw, wide)?;
    Ok(NativeReturn::Int(d as i64))
}

fn lstrcat(ctx: &mut CallContext, a: &Args, wide: bool) -> Result<NativeReturn, WinApiError> {
    let (d, s) = (a.ptr(0)?, a.ptr(1)?);
    if d == 0 || s == 0 {
        return Ok(NativeReturn::Int(0));
    }
    let tail = read_raw(ctx, s, wide)?;
    let end = d + read_raw(ctx, d, wide)?.len() as u64;
    write_terminated(ctx, end, &tail, wide)?;
    Ok(NativeReturn::Int(d as i64))
}

/// Culture-sensitive comparison cannot be reproduced in general. Implemented
/// exactly for strings of ASCII letters and digits under en-US/invariant word
/// sort: digits sort before letters, letters compare ignoring case first, and
/// (for `lstrcmp` only) a lowercase letter sorts before its uppercase twin at
/// the first case difference. Anything else is `Unsupported`.
fn lstrcmp(
    ctx: &mut CallContext,
    a: &Args,
    wide: bool,
    ignore_case: bool,
) -> Result<NativeReturn, WinApiError> {
    let (x, y) = (a.ptr(0)?, a.ptr(1)?);
    let to_chars = |raw: Vec<u8>| -> Vec<u16> {
        if wide {
            raw.as_chunks::<2>()
                .0
                .iter()
                .map(|p| u16::from_le_bytes(*p))
                .collect()
        } else {
            raw.iter().map(|b| u16::from(*b)).collect()
        }
    };
    let (x, y) = (
        to_chars(read_raw(ctx, x, wide)?),
        to_chars(read_raw(ctx, y, wide)?),
    );
    let simple = |s: &[u16]| {
        s.iter()
            .all(|u| u8::try_from(*u).is_ok_and(|b| b.is_ascii_alphanumeric()))
    };
    if !simple(&x) || !simple(&y) {
        return Err(WinApiError::Unsupported {
            symbol: if wide { "lstrcmpW" } else { "lstrcmpA" }.to_string()
                + if ignore_case { " (i)" } else { "" },
            reason: "locale-sensitive collation is only reproduced for ASCII letters and digits"
                .into(),
        });
    }
    let fold = |s: &[u16]| {
        s.iter()
            .map(|u| (*u as u8).to_ascii_lowercase())
            .collect::<Vec<_>>()
    };
    let mut ord = fold(&x).cmp(&fold(&y));
    if ord == Ordering::Equal && !ignore_case {
        // Same letters; lowercase before uppercase at the first difference.
        ord = x
            .iter()
            .zip(&y)
            .find(|(p, q)| p != q)
            .map(|(p, _)| {
                if (*p as u8).is_ascii_lowercase() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            })
            .unwrap_or(Ordering::Equal);
    }
    Ok(NativeReturn::Int(ord as i64))
}

fn map_char(c: char, upper: bool) -> Option<char> {
    let mut it: Box<dyn Iterator<Item = char>> = if upper {
        Box::new(c.to_uppercase())
    } else {
        Box::new(c.to_lowercase())
    };
    match (it.next(), it.next()) {
        (Some(m), None) => Some(m),
        _ => None,
    }
}

/// Maps one ANSI byte; characters whose mapping is not representable in the
/// code page (or is not a single character) are left unchanged.
fn map_byte(cp: Codepage, b: u8, upper: bool) -> u8 {
    let Ok(units) = cp.decode(&[b], false) else {
        return b;
    };
    let Some(Ok(c)) = char::decode_utf16(units).next() else {
        return b;
    };
    let Some(m) = map_char(c, upper) else {
        return b;
    };
    let mut buf = [0u16; 2];
    match cp.encode(m.encode_utf16(&mut buf), Unmappable::Reject, true) {
        Ok(e) if e.bytes.len() == 1 => e.bytes[0],
        _ => b,
    }
}

fn map_unit(u: u16, upper: bool) -> u16 {
    char::from_u32(u32::from(u))
        .and_then(|c| map_char(c, upper))
        .and_then(|m| u16::try_from(u32::from(m)).ok())
        .unwrap_or(u)
}

/// A pointer whose high word is zero is a single character (documented
/// `CharUpper` convention) and the converted character is returned; otherwise
/// the string is converted in place and the pointer is returned. Case mapping
/// uses Unicode simple mappings of the Rust standard library; Windows' own
/// tables may differ for rare characters (see docs/winapi.md).
fn char_case(
    ctx: &mut CallContext,
    a: &Args,
    wide: bool,
    upper: bool,
) -> Result<NativeReturn, WinApiError> {
    let p = a.ptr(0)?;
    let cp = ctx.ansi_codepage;
    if !wide && cp == Codepage::Utf8 {
        return Err(WinApiError::Unsupported {
            symbol: "user32!CharUpperA/CharLowerA".into(),
            reason: "UTF-8 is not a valid system ANSI code page for single-byte case mapping"
                .into(),
        });
    }
    if p <= 0xFFFF {
        let v = if wide {
            u64::from(map_unit(p as u16, upper))
        } else {
            u64::from(map_byte(cp, p as u8, upper))
        };
        return Ok(NativeReturn::Int(v as i64));
    }
    if wide {
        let units = ctx.memory.read_wstr(p)?;
        let bytes: Vec<u8> = units
            .iter()
            .flat_map(|u| map_unit(*u, upper).to_le_bytes())
            .collect();
        ctx.memory.write(p, &bytes)?;
    } else {
        let bytes: Vec<u8> = ctx
            .memory
            .read_cstr(p)?
            .iter()
            .map(|b| map_byte(cp, *b, upper))
            .collect();
        ctx.memory.write(p, &bytes)?;
    }
    Ok(NativeReturn::Int(p as i64))
}
