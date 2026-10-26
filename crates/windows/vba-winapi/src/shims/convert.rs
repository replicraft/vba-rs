//! kernel32 MultiByteToWideChar and WideCharToMultiByte.
//!
//! Docs: learn.microsoft.com/windows/win32/api/stringapiset/nf-stringapiset-multibytetowidechar
//! and ...-widechartomultibyte.
//!
//! Supported code pages: 20127 (US-ASCII), 28591 (ISO-8859-1), 1252, 65001
//! (UTF-8), plus `CP_ACP` (0) and `CP_THREAD_ACP` (3) resolving to the context's
//! ANSI code page. Everything else, flags whose effect is not reproduced and
//! best-fit substitutions fail visibly with `Unsupported`.

use super::{Args, Shim, invalid_parameter};
use crate::{CallContext, WinApiError};
use vba_ffi::{CodecError, Codepage, NativeReturn, Unmappable};
use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_NO_UNICODE_TRANSLATION};
use windows_sys::Win32::Globalization::{
    CP_ACP, CP_THREAD_ACP, MB_COMPOSITE, MB_ERR_INVALID_CHARS, MB_PRECOMPOSED, MB_USEGLYPHCHARS,
    WC_COMPOSITECHECK, WC_DEFAULTCHAR, WC_DISCARDNS, WC_ERR_INVALID_CHARS, WC_NO_BEST_FIT_CHARS,
    WC_SEPCHARS,
};

pub(super) const SHIMS: [Shim; 2] = [
    Shim {
        dll: "kernel32",
        symbol: "MultiByteToWideChar",
        arity: 6,
        run: mb_to_wc,
    },
    Shim {
        dll: "kernel32",
        symbol: "WideCharToMultiByte",
        arity: 8,
        run: wc_to_mb,
    },
];

const WC_UNIMPLEMENTED: u32 = WC_DISCARDNS | WC_SEPCHARS | WC_DEFAULTCHAR | WC_COMPOSITECHECK;

fn unsupported(symbol: &str, reason: impl Into<String>) -> WinApiError {
    WinApiError::Unsupported {
        symbol: format!("kernel32!{symbol}"),
        reason: reason.into(),
    }
}

fn code_page(ctx: &CallContext, symbol: &str, id: u32) -> Result<Codepage, WinApiError> {
    let id = if id == CP_ACP || id == CP_THREAD_ACP {
        ctx.ansi_codepage.id()
    } else {
        id
    };
    Codepage::from_id(id)
        .ok_or_else(|| unsupported(symbol, format!("code page {id} is not reproduced")))
}

fn fail(ctx: &mut CallContext, code: u32) -> NativeReturn {
    ctx.set_last_error(code);
    NativeReturn::Int(0)
}

fn mb_to_wc(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let cp = code_page(ctx, "MultiByteToWideChar", a.u32(0)?)?;
    let flags = a.u32(1)?;
    let (src, cb) = (a.ptr(2)?, a.i32(3)?);
    let (dst, cch) = (a.ptr(4)?, a.i32(5)?);
    let known = MB_PRECOMPOSED | MB_COMPOSITE | MB_USEGLYPHCHARS | MB_ERR_INVALID_CHARS;
    let flags_ok = if cp == Codepage::Utf8 {
        flags & !MB_ERR_INVALID_CHARS == 0
    } else {
        flags & !known == 0
    };
    if !flags_ok || src == 0 || cb == 0 || cb < -1 || cch < 0 || (cch > 0 && dst == 0) {
        return Ok(invalid_parameter(ctx));
    }
    if flags & (MB_COMPOSITE | MB_USEGLYPHCHARS) != 0 {
        return Err(unsupported(
            "MultiByteToWideChar",
            "MB_COMPOSITE and MB_USEGLYPHCHARS are not reproduced",
        ));
    }
    let mut bytes = if cb == -1 {
        ctx.memory.read_cstr(src)?
    } else {
        ctx.memory.read(src, cb as usize)?.to_vec()
    };
    if cb == -1 {
        bytes.push(0);
    }
    let strict = flags & MB_ERR_INVALID_CHARS != 0;
    let units = match cp.decode(&bytes, strict) {
        Ok(u) => u,
        Err(CodecError::InvalidSequence { .. }) if strict => {
            return Ok(fail(ctx, ERROR_NO_UNICODE_TRANSLATION));
        }
        Err(_) => {
            return Err(unsupported(
                "MultiByteToWideChar",
                "bytes above 0x7F in US-ASCII: Windows' substitution is not reproduced (use MB_ERR_INVALID_CHARS)",
            ));
        }
    };
    if cch == 0 {
        return Ok(NativeReturn::Int(units.len() as i64));
    }
    if (cch as usize) < units.len() {
        return Ok(fail(ctx, ERROR_INSUFFICIENT_BUFFER));
    }
    let out: Vec<u8> = units.iter().flat_map(|u| u.to_le_bytes()).collect();
    ctx.memory.write(dst, &out)?;
    Ok(NativeReturn::Int(units.len() as i64))
}

fn wc_to_mb(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let cp = code_page(ctx, "WideCharToMultiByte", a.u32(0)?)?;
    let flags = a.u32(1)?;
    let (src, cch) = (a.ptr(2)?, a.i32(3)?);
    let (dst, cb) = (a.ptr(4)?, a.i32(5)?);
    let (def, used) = (a.ptr(6)?, a.ptr(7)?);
    let utf8 = cp == Codepage::Utf8;
    let allowed = if utf8 {
        WC_ERR_INVALID_CHARS
    } else {
        WC_NO_BEST_FIT_CHARS | WC_UNIMPLEMENTED
    };
    if flags & !allowed != 0
        || src == 0
        || cch == 0
        || cch < -1
        || cb < 0
        || (cb > 0 && dst == 0)
        || (utf8 && (def != 0 || used != 0))
    {
        return Ok(invalid_parameter(ctx));
    }
    if flags & WC_UNIMPLEMENTED != 0 {
        return Err(unsupported(
            "WideCharToMultiByte",
            "WC_COMPOSITECHECK, WC_DISCARDNS, WC_SEPCHARS and WC_DEFAULTCHAR are not reproduced",
        ));
    }
    let mut units = if cch == -1 {
        ctx.memory.read_wstr(src)?
    } else {
        let raw = ctx.memory.read(src, cch as usize * 2)?;
        raw.as_chunks::<2>()
            .0
            .iter()
            .map(|p| u16::from_le_bytes(*p))
            .collect()
    };
    if cch == -1 {
        units.push(0);
    }
    let default_char = if def != 0 {
        ctx.memory.read(def, 1)?[0]
    } else {
        b'?'
    };
    let strict = flags & WC_ERR_INVALID_CHARS != 0;
    let encoded = match cp.encode(&units, Unmappable::Reject, strict) {
        Ok(e) => e,
        Err(_) if utf8 => return Ok(fail(ctx, ERROR_NO_UNICODE_TRANSLATION)),
        Err(_) if flags & WC_NO_BEST_FIT_CHARS != 0 => cp
            .encode(&units, Unmappable::Replace(default_char), false)
            .map_err(|e| unsupported("WideCharToMultiByte", e.to_string()))?,
        Err(_) => {
            return Err(unsupported(
                "WideCharToMultiByte",
                "best-fit character substitution is not reproduced (pass WC_NO_BEST_FIT_CHARS)",
            ));
        }
    };
    if used != 0 {
        ctx.memory
            .write_u32(used, u32::from(encoded.used_default))?;
    }
    if cb == 0 {
        return Ok(NativeReturn::Int(encoded.bytes.len() as i64));
    }
    if (cb as usize) < encoded.bytes.len() {
        return Ok(fail(ctx, ERROR_INSUFFICIENT_BUFFER));
    }
    ctx.memory.write(dst, &encoded.bytes)?;
    Ok(NativeReturn::Int(encoded.bytes.len() as i64))
}
