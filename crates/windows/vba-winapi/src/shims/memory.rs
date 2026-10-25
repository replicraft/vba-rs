//! kernel32 RtlMoveMemory, RtlFillMemory, RtlZeroMemory.
//!
//! `CopyMemory`, `FillMemory` and `ZeroMemory` are SDK macros, not exports;
//! VBA code reaches these functions through `Alias "RtlMoveMemory"` etc.
//! Docs: learn.microsoft.com/windows/win32/devnotes/rtlmovememory,
//! .../rtlfillmemory, .../rtlzeromemory.
//!
//! Real Windows corrupts memory or crashes on a bad range; here every range is
//! validated against the emulated address space first and nothing is written
//! if any part is invalid (`WinApiError::Memory`).

use super::{Args, Shim};
use crate::{CallContext, WinApiError};
use vba_ffi::NativeReturn;

pub(super) const SHIMS: [Shim; 3] = [
    Shim {
        dll: "kernel32",
        symbol: "RtlMoveMemory",
        arity: 3,
        run: move_memory,
    },
    Shim {
        dll: "kernel32",
        symbol: "RtlFillMemory",
        arity: 3,
        run: fill_memory,
    },
    Shim {
        dll: "kernel32",
        symbol: "RtlZeroMemory",
        arity: 2,
        run: zero_memory,
    },
];

/// `memmove` semantics: overlapping ranges are handled.
fn move_memory(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let (dst, src, len) = (a.ptr(0)?, a.ptr(1)?, a.len(2)?);
    if len > 0 {
        ctx.memory.copy(dst, src, len)?;
    }
    Ok(NativeReturn::Void)
}

/// Arguments are `(Destination, Length, Fill)`.
fn fill_memory(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let (dst, len) = (a.ptr(0)?, a.len(1)?);
    let fill = u8::try_from(a.int(2)?).map_err(|_| a.bad(2, "fill must be a Byte"))?;
    if len > 0 {
        ctx.memory.fill(dst, len, fill)?;
    }
    Ok(NativeReturn::Void)
}

fn zero_memory(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let (dst, len) = (a.ptr(0)?, a.len(1)?);
    if len > 0 {
        ctx.memory.fill(dst, len, 0)?;
    }
    Ok(NativeReturn::Void)
}
