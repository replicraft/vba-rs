//! Functions that take a procedure pointer: user32 `EnumWindows`,
//! `EnumChildWindows`, `SetTimer`, `KillTimer`, and msvcrt `qsort`.
//!
//! Docs: learn.microsoft.com/windows/win32/api/winuser/nf-winuser-enumwindows,
//! ...-enumchildwindows, ...-settimer, ...-killtimer, and
//! learn.microsoft.com/cpp/c-runtime-library/reference/qsort.
//!
//! The procedure pointer is the token `AddressOf` produced; the shim hands it
//! to the callback host (the interpreter), which runs the VBA procedure after
//! checking it against the callback signature. Enumeration works on the
//! context's window list (empty unless the embedding application fills it);
//! timers are kept in the context and delivered by
//! [`dispatch_timers`](crate::dispatch_timers), the stand-in for a message
//! pump.

use super::{Args, Shim};
use crate::callback::{QSORT_COMPARE, TIMERPROC, WNDENUMPROC};
use crate::{CallContext, WinApiError};
use vba_ffi::NativeReturn;
use windows_sys::Win32::Foundation::{
    ERROR_INVALID_PARAMETER, ERROR_INVALID_WINDOW_HANDLE, ERROR_NO_SYSTEM_RESOURCES,
};

pub(super) const SHIMS: [Shim; 5] = [
    Shim {
        dll: "user32",
        symbol: "EnumWindows",
        arity: 2,
        run: enum_windows,
    },
    Shim {
        dll: "user32",
        symbol: "EnumChildWindows",
        arity: 3,
        run: enum_child_windows,
    },
    Shim {
        dll: "user32",
        symbol: "SetTimer",
        arity: 4,
        run: set_timer,
    },
    Shim {
        dll: "user32",
        symbol: "KillTimer",
        arity: 2,
        run: kill_timer,
    },
    Shim {
        dll: "msvcrt",
        symbol: "qsort",
        arity: 4,
        run: qsort,
    },
];

/// Calls the enumeration procedure for each window; stops when it returns 0.
fn enumerate(
    ctx: &mut CallContext,
    a: &Args,
    list: Vec<u64>,
    token: i64,
    lparam: i64,
) -> Result<bool, WinApiError> {
    for hwnd in list {
        let r = a.callback(ctx, token, &WNDENUMPROC, &[hwnd as i64, lparam])?;
        if r as i32 == 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Returns TRUE when every window was visited, FALSE when the procedure
/// stopped the enumeration (documented for `EnumWindows`).
fn enum_windows(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let token = a.int(0)?;
    a.check_callback(0, token, &WNDENUMPROC)?;
    let list = ctx.windows.top_level();
    let done = enumerate(ctx, a, list, token, a.int(1)?)?;
    Ok(NativeReturn::Int(i64::from(done)))
}

/// A null parent is `EnumWindows`; an unknown parent is error 1400. The
/// documented return value is "not used"; this returns FALSE when the
/// procedure stopped the enumeration, as Windows does.
fn enum_child_windows(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let parent = a.ptr(0)?;
    let token = a.int(1)?;
    a.check_callback(1, token, &WNDENUMPROC)?;
    let list = if parent == 0 {
        ctx.windows.top_level()
    } else if ctx.windows.contains(parent) {
        ctx.windows.descendants(parent)
    } else {
        ctx.set_last_error(ERROR_INVALID_WINDOW_HANDLE);
        return Ok(NativeReturn::Int(0));
    };
    let done = enumerate(ctx, a, list, token, a.int(2)?)?;
    Ok(NativeReturn::Int(i64::from(done)))
}

/// `uElapse` is clamped to `USER_TIMER_MINIMUM` (10) and `USER_TIMER_MAXIMUM`
/// (0x7FFFFFFF). Without a procedure the timer would post `WM_TIMER` to the
/// window, and there is no message queue to receive it: `Unsupported`.
fn set_timer(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let hwnd = a.ptr(0)?;
    let id = a.ptr(1)?;
    let elapse = a.u32(2)?.clamp(10, 0x7FFF_FFFF);
    let token = a.int(3)?;
    if token == 0 {
        return Err(WinApiError::Unsupported {
            symbol: "user32!SetTimer".into(),
            reason: "a timer without a TIMERPROC posts WM_TIMER to a window, and there is no message queue".into(),
        });
    }
    a.check_callback(3, token, &TIMERPROC)?;
    if hwnd != 0 && !ctx.windows.contains(hwnd) {
        ctx.set_last_error(ERROR_INVALID_WINDOW_HANDLE);
        return Ok(NativeReturn::Int(0));
    }
    let now = ctx.env.tick_ms();
    match ctx.timers.set(hwnd, id, elapse, now, token) {
        Some(id) => Ok(NativeReturn::Int(id as i64)),
        None => {
            ctx.set_last_error(ERROR_NO_SYSTEM_RESOURCES);
            Ok(NativeReturn::Int(0))
        }
    }
}

/// An unknown timer is FALSE with error 87 (the documentation does not name
/// the code; the choice is recorded in the catalog).
fn kill_timer(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let (hwnd, id) = (a.ptr(0)?, a.ptr(1)?);
    if ctx.timers.kill(hwnd, id) {
        Ok(NativeReturn::Int(1))
    } else {
        ctx.set_last_error(ERROR_INVALID_PARAMETER);
        Ok(NativeReturn::Int(0))
    }
}

struct Sorter<'a, 'b, 'c> {
    ctx: &'a mut CallContext,
    a: &'a Args<'b, 'c>,
    base: u64,
    size: usize,
    token: i64,
}

impl Sorter<'_, '_, '_> {
    fn addr(&self, i: usize) -> u64 {
        self.base + (i * self.size) as u64
    }

    /// Calls the comparison procedure on elements `i` and `j` in place.
    fn cmp(&mut self, i: usize, j: usize) -> Result<i32, WinApiError> {
        let (x, y) = (self.addr(i) as i64, self.addr(j) as i64);
        let r = self
            .a
            .callback(self.ctx, self.token, &QSORT_COMPARE, &[x, y])?;
        Ok((r as i32).signum())
    }

    fn swap(&mut self, i: usize, j: usize) -> Result<(), WinApiError> {
        if i == j {
            return Ok(());
        }
        let (pi, pj) = (self.addr(i), self.addr(j));
        let bi = self.ctx.memory.read(pi, self.size)?.to_vec();
        let bj = self.ctx.memory.read(pj, self.size)?.to_vec();
        self.ctx.memory.write(pi, &bj)?;
        self.ctx.memory.write(pj, &bi)?;
        Ok(())
    }

    fn sift_down(&mut self, mut root: usize, end: usize) -> Result<(), WinApiError> {
        loop {
            let mut child = 2 * root + 1;
            if child >= end {
                return Ok(());
            }
            if child + 1 < end && self.cmp(child, child + 1)? < 0 {
                child += 1;
            }
            if self.cmp(root, child)? < 0 {
                self.swap(root, child)?;
                root = child;
            } else {
                return Ok(());
            }
        }
    }
}

/// Sorts `num` elements of `size` bytes at `base` in emulated memory with an
/// in-place heap sort: O(n log n) comparisons, no scratch memory, the
/// procedure always sees pointers to real elements. Like C `qsort` it is not
/// stable. The whole range is validated first, so a bad range sorts nothing.
fn qsort(ctx: &mut CallContext, a: &Args) -> Result<NativeReturn, WinApiError> {
    let base = a.ptr(0)?;
    let num = a.len(1)?;
    let size = a.len(2)?;
    let token = a.int(3)?;
    a.check_callback(3, token, &QSORT_COMPARE)?;
    if num < 2 || size == 0 {
        return Ok(NativeReturn::Void);
    }
    let total = num
        .checked_mul(size)
        .ok_or_else(|| a.bad(1, "num * size overflows"))?;
    ctx.memory.read(base, total)?;
    let mut s = Sorter {
        ctx,
        a,
        base,
        size,
        token,
    };
    for start in (0..num / 2).rev() {
        s.sift_down(start, num)?;
    }
    for end in (1..num).rev() {
        s.swap(0, end)?;
        s.sift_down(0, end)?;
    }
    Ok(NativeReturn::Void)
}
