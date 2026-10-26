//! Callbacks from shims into VBA procedures (`AddressOf`).
//!
//! A shim that takes a procedure pointer never calls native code: it receives
//! the opaque token VBA code got from `AddressOf` (see
//! `vba_ffi::CallbackTable`) and asks a [`CallbackHost`], supplied by the
//! interpreter for the duration of the call, to run the procedure. The host
//! validates the token and checks the procedure against the [`CallbackSig`]
//! the shim expects (arity, `ByVal` parameter types, result) before running
//! anything, and runs it under the session's step, depth and stack budgets.
//!
//! The emulated window list and timer table that the enumeration and timer
//! shims work on live in the [`CallContext`](crate::CallContext), so they are
//! empty unless the embedding application fills them.

use crate::CallContext;

/// The class of one callback parameter slot. The host maps a slot to the VBA
/// type the target platform gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CbSlot {
    /// A 32-bit integer (`DWORD`, `UINT`, `int`, `BOOL`): VBA `Long`.
    Int32,
    /// A pointer-sized integer (`HWND`, `LPARAM`, `UINT_PTR`, `void*`): VBA
    /// `LongPtr`, which is `Long` on a 32-bit target and `LongLong` on a
    /// 64-bit one.
    Ptr,
}

/// What a callback returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CbRet {
    /// Nothing is used (`VOID`); a `Sub` or a `Function` are both accepted.
    Void,
    /// A 32-bit integer: the procedure must be a `Function ... As Long`.
    Int32,
}

/// The signature a shim requires of the procedure behind a token. Every
/// parameter must be `ByVal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallbackSig {
    /// The Win32 type name, for diagnostics (`WNDENUMPROC`).
    pub name: &'static str,
    /// Parameters, in order.
    pub params: &'static [CbSlot],
    /// Result.
    pub ret: CbRet,
}

/// `BOOL CALLBACK EnumWindowsProc(HWND hwnd, LPARAM lParam)`.
pub const WNDENUMPROC: CallbackSig = CallbackSig {
    name: "WNDENUMPROC",
    params: &[CbSlot::Ptr, CbSlot::Ptr],
    ret: CbRet::Int32,
};

/// `VOID CALLBACK TimerProc(HWND hwnd, UINT uMsg, UINT_PTR idEvent, DWORD dwTime)`.
pub const TIMERPROC: CallbackSig = CallbackSig {
    name: "TIMERPROC",
    params: &[CbSlot::Ptr, CbSlot::Int32, CbSlot::Ptr, CbSlot::Int32],
    ret: CbRet::Void,
};

/// `int __cdecl compare(const void *a, const void *b)`.
pub const QSORT_COMPARE: CallbackSig = CallbackSig {
    name: "qsort comparison function",
    params: &[CbSlot::Ptr, CbSlot::Ptr],
    ret: CbRet::Int32,
};

/// Why a callback did not produce a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackError {
    /// The token was refused or the procedure does not fit the signature.
    /// Nothing ran.
    Rejected(String),
    /// The procedure ran and ended abnormally (a run-time error, a resource
    /// limit, cancellation, `End`, `Stop`). The host keeps the cause and
    /// raises it itself once the shim has returned; the shim must stop and
    /// return without touching anything else.
    Aborted,
}

/// The interpreter side of a callback.
pub trait CallbackHost {
    /// Checks `token` against `sig` without running anything.
    fn validate(&mut self, token: i64, sig: &CallbackSig) -> Result<(), CallbackError>;

    /// Runs the procedure behind `token` with `args` (one per slot of `sig`)
    /// and returns its result (0 for [`CbRet::Void`]). The context is the
    /// one the calling shim holds: nested `Declare` calls made by the
    /// procedure see the same emulated memory, handles and last error.
    fn invoke(
        &mut self,
        ctx: &mut CallContext,
        token: i64,
        sig: &CallbackSig,
        args: &[i64],
    ) -> Result<i64, CallbackError>;
}

/// An entry of the emulated window list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// The `HWND` value VBA code sees: an opaque non-zero number chosen by
    /// the embedding application, not a host window.
    pub hwnd: u64,
    /// Parent `HWND`; 0 for a top-level window.
    pub parent: u64,
    /// The creating thread id.
    pub thread: u32,
}

/// The windows the program can enumerate. Empty by default.
#[derive(Debug, Clone, Default)]
pub struct WindowList {
    windows: Vec<Window>,
}

impl WindowList {
    /// Adds a window; `hwnd` must be non-zero and unused.
    pub fn add(&mut self, window: Window) -> bool {
        if window.hwnd == 0 || self.windows.iter().any(|w| w.hwnd == window.hwnd) {
            return false;
        }
        self.windows.push(window);
        true
    }

    /// All windows in z-order (insertion order).
    pub fn all(&self) -> &[Window] {
        &self.windows
    }

    /// Top-level windows.
    pub fn top_level(&self) -> Vec<u64> {
        self.windows
            .iter()
            .filter(|w| w.parent == 0)
            .map(|w| w.hwnd)
            .collect()
    }

    /// Every descendant of `parent`, depth first, in insertion order within
    /// a level (the order `EnumChildWindows` documents).
    pub fn descendants(&self, parent: u64) -> Vec<u64> {
        let mut out = Vec::new();
        let mut seen = vec![parent];
        self.collect(parent, &mut out, &mut seen);
        out
    }

    fn collect(&self, parent: u64, out: &mut Vec<u64>, seen: &mut Vec<u64>) {
        for w in self.windows.iter().filter(|w| w.parent == parent) {
            if seen.contains(&w.hwnd) {
                continue;
            }
            seen.push(w.hwnd);
            out.push(w.hwnd);
            self.collect(w.hwnd, out, seen);
        }
    }

    /// Whether `hwnd` is in the list.
    pub fn contains(&self, hwnd: u64) -> bool {
        self.windows.iter().any(|w| w.hwnd == hwnd)
    }
}

/// Most timers one session may hold (`SetTimer` fails with
/// `ERROR_NO_SYSTEM_RESOURCES` beyond it).
pub const MAX_TIMERS: usize = 256;

/// A timer set with `SetTimer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timer {
    /// Owning window, or 0.
    pub hwnd: u64,
    /// Timer id.
    pub id: u64,
    /// Interval in milliseconds.
    pub interval_ms: u32,
    /// Clock reading at which it is next due.
    pub due_ms: u64,
    /// The callback token.
    pub token: i64,
}

/// The timers of a session.
#[derive(Debug, Clone, Default)]
pub struct TimerTable {
    timers: Vec<Timer>,
    next_id: u64,
}

impl TimerTable {
    /// Number of live timers.
    pub fn len(&self) -> usize {
        self.timers.len()
    }

    /// Whether no timer is set.
    pub fn is_empty(&self) -> bool {
        self.timers.is_empty()
    }

    /// The live timers.
    pub fn timers(&self) -> &[Timer] {
        &self.timers
    }

    /// Sets (or resets) the timer `(hwnd, id)`. With `hwnd == 0` the system
    /// picks a new id, as `SetTimer` does. `None` when the table is full.
    pub fn set(
        &mut self,
        hwnd: u64,
        id: u64,
        interval_ms: u32,
        now_ms: u64,
        token: i64,
    ) -> Option<u64> {
        let id = if hwnd == 0 {
            self.next_id += 1;
            self.next_id
        } else {
            id
        };
        let t = Timer {
            hwnd,
            id,
            interval_ms,
            due_ms: now_ms + u64::from(interval_ms),
            token,
        };
        if let Some(slot) = self
            .timers
            .iter_mut()
            .find(|x| x.hwnd == hwnd && x.id == id)
        {
            *slot = t;
        } else if self.timers.len() >= MAX_TIMERS {
            return None;
        } else {
            self.timers.push(t);
        }
        Some(id)
    }

    /// Removes the timer `(hwnd, id)`; whether it existed.
    pub fn kill(&mut self, hwnd: u64, id: u64) -> bool {
        let before = self.timers.len();
        self.timers.retain(|t| !(t.hwnd == hwnd && t.id == id));
        self.timers.len() != before
    }

    /// Forgets every timer.
    pub fn clear(&mut self) {
        self.timers.clear();
    }

    /// The timers due at `now_ms`, earliest first, rescheduled one interval
    /// after `now_ms` (a message pump delivers one `WM_TIMER` however late
    /// it runs).
    pub(crate) fn take_due(&mut self, now_ms: u64) -> Vec<Timer> {
        let mut due: Vec<Timer> = self
            .timers
            .iter()
            .filter(|t| t.due_ms <= now_ms)
            .copied()
            .collect();
        due.sort_by_key(|t| t.due_ms);
        for t in &mut self.timers {
            if t.due_ms <= now_ms {
                t.due_ms = now_ms + u64::from(t.interval_ms);
            }
        }
        due
    }
}

/// Delivers the timers that are due (the `WM_TIMER` a message pump would
/// dispatch) by calling their `TIMERPROC`s through `host`. Returns how many
/// ran. A timer killed by an earlier callback of the same pump is skipped.
pub fn dispatch_timers(
    ctx: &mut CallContext,
    host: &mut dyn CallbackHost,
) -> Result<usize, CallbackError> {
    let now = ctx.env.tick_ms();
    let due = ctx.timers.take_due(now);
    let mut ran = 0;
    for t in due {
        if !ctx
            .timers
            .timers()
            .iter()
            .any(|x| x.hwnd == t.hwnd && x.id == t.id)
        {
            continue;
        }
        host.invoke(
            ctx,
            t.token,
            &TIMERPROC,
            &[t.hwnd as i64, 0x113, t.id as i64, now as u32 as i32 as i64],
        )?;
        ran += 1;
    }
    Ok(ran)
}
