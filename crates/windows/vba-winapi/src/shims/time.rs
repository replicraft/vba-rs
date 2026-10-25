//! kernel32 timing: GetTickCount, GetTickCount64, QueryPerformanceCounter,
//! QueryPerformanceFrequency, Sleep.
//!
//! Docs: learn.microsoft.com/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount,
//! ...-gettickcount64, .../profileapi/nf-profileapi-queryperformancecounter,
//! ...-queryperformancefrequency, .../synchapi/nf-synchapi-sleep.

use super::{Args, Shim};
use crate::{CallContext, WinApiError};
use vba_ffi::NativeReturn;

pub(super) const SHIMS: [Shim; 5] = [
    Shim {
        dll: "kernel32",
        symbol: "GetTickCount",
        arity: 0,
        run: get_tick_count,
    },
    Shim {
        dll: "kernel32",
        symbol: "GetTickCount64",
        arity: 0,
        run: get_tick_count64,
    },
    Shim {
        dll: "kernel32",
        symbol: "QueryPerformanceCounter",
        arity: 1,
        run: qpc,
    },
    Shim {
        dll: "kernel32",
        symbol: "QueryPerformanceFrequency",
        arity: 1,
        run: qpf,
    },
    Shim {
        dll: "kernel32",
        symbol: "Sleep",
        arity: 1,
        run: sleep,
    },
];

/// Returns the low 32 bits (a `DWORD`), so it wraps every 49.7 days.
fn get_tick_count(ctx: &mut CallContext, _: &Args) -> Result<NativeReturn, WinApiError> {
    Ok(NativeReturn::Int(i64::from(ctx.env.tick_ms() as u32)))
}

fn get_tick_count64(ctx: &mut CallContext, _: &Args) -> Result<NativeReturn, WinApiError> {
    Ok(NativeReturn::Int(ctx.env.tick_ms() as i64))
}

/// Writes a `LARGE_INTEGER`; always succeeds (returns TRUE = 1) for a valid
/// pointer. A bad pointer is an emulated memory fault, not a Win32 error.
fn qpc(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let v = ctx.env.perf_counter() as i64;
    ctx.memory.write(a.ptr(0)?, &v.to_le_bytes())?;
    Ok(NativeReturn::Int(1))
}

fn qpf(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let v = ctx.env.perf_frequency() as i64;
    ctx.memory.write(a.ptr(0)?, &v.to_le_bytes())?;
    Ok(NativeReturn::Int(1))
}

/// `Sleep(INFINITE)` would never return; it is refused rather than hanging.
fn sleep(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let ms = a.u32(0)?;
    if ms == u32::MAX {
        return Err(WinApiError::Unsupported {
            symbol: "kernel32!Sleep".into(),
            reason: "INFINITE would block forever".into(),
        });
    }
    ctx.env.sleep_ms(ms);
    Ok(NativeReturn::Void)
}
