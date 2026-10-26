//! Per-process state a Win32 call sees: injected clock, identity, reported
//! OS version, emulated memory, handles and last-error. Nothing here reads the
//! real machine.

use crate::WinApiError;
use crate::callback::{TimerTable, WindowList};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use vba_ffi::{Codepage, HandleTable, LastErrorStore, LogicalThread, Memory};
use vba_profile::Target;

/// Time and sleeping, injectable so shims are deterministic under test.
pub trait HostEnv {
    /// Milliseconds since an arbitrary epoch (`GetTickCount64`).
    fn tick_ms(&self) -> u64;
    /// High-resolution counter (`QueryPerformanceCounter`).
    fn perf_counter(&self) -> u64;
    /// Counter ticks per second (`QueryPerformanceFrequency`), never zero.
    fn perf_frequency(&self) -> u64;
    /// Blocks for `ms` milliseconds.
    fn sleep_ms(&self, ms: u32);
}

/// Real monotonic clock; `sleep_ms` really sleeps.
#[derive(Debug)]
pub struct SystemClock {
    start: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        SystemClock {
            start: Instant::now(),
        }
    }
}

impl HostEnv for SystemClock {
    fn tick_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
    fn perf_counter(&self) -> u64 {
        self.start.elapsed().as_nanos() as u64
    }
    fn perf_frequency(&self) -> u64 {
        1_000_000_000
    }
    fn sleep_ms(&self, ms: u32) {
        std::thread::sleep(Duration::from_millis(u64::from(ms)));
    }
}

#[derive(Debug, Default)]
struct ManualState {
    ms: u64,
    slept: Vec<u32>,
    /// The `slept` list is charged to the run's memory meter.
    charge: vba_core::Charge,
}

/// A deterministic clock: time moves only when advanced or slept on.
#[derive(Debug, Clone, Default)]
pub struct ManualClock {
    state: Rc<RefCell<ManualState>>,
}

impl ManualClock {
    /// Counter ticks per millisecond (so frequency is 10 MHz).
    pub const TICKS_PER_MS: u64 = 10_000;

    /// Creates a clock at time `ms`.
    pub fn at(ms: u64) -> Self {
        let c = Self::default();
        c.state.borrow_mut().ms = ms;
        c
    }
    /// Advances time.
    pub fn advance(&self, ms: u64) {
        self.state.borrow_mut().ms += ms;
    }
    /// Durations passed to `Sleep`, in order.
    pub fn slept(&self) -> Vec<u32> {
        self.state.borrow().slept.clone()
    }
}

impl HostEnv for ManualClock {
    fn tick_ms(&self) -> u64 {
        self.state.borrow().ms
    }
    fn perf_counter(&self) -> u64 {
        self.state.borrow().ms * Self::TICKS_PER_MS
    }
    fn perf_frequency(&self) -> u64 {
        Self::TICKS_PER_MS * 1000
    }
    fn sleep_ms(&self, ms: u32) {
        let mut s = self.state.borrow_mut();
        s.slept.push(ms);
        let bytes = s.slept.capacity() as u64 * 4;
        s.charge.set(bytes);
        s.ms += u64::from(ms);
    }
}

/// Host-injected user and machine names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    user_name: String,
    computer_name: String,
}

impl Identity {
    /// Creates an identity. `UNLEN` is 256 and `MAX_COMPUTERNAME_LENGTH` 15;
    /// both names must be non-empty.
    pub fn new(user_name: &str, computer_name: &str) -> Result<Self, WinApiError> {
        if user_name.is_empty() || user_name.encode_utf16().count() > 256 {
            return Err(WinApiError::InvalidIdentity(
                "user name must be 1..=256 characters",
            ));
        }
        if computer_name.is_empty() || computer_name.encode_utf16().count() > 15 {
            return Err(WinApiError::InvalidIdentity(
                "computer name must be 1..=15 characters",
            ));
        }
        Ok(Identity {
            user_name: user_name.into(),
            computer_name: computer_name.into(),
        })
    }
    /// The user name.
    pub fn user_name(&self) -> &str {
        &self.user_name
    }
    /// The computer name.
    pub fn computer_name(&self) -> &str {
        &self.computer_name
    }
}

/// The Windows version reported to VBA code (never the host's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsVersion {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Build number.
    pub build: u32,
    /// Platform id (2 = `VER_PLATFORM_WIN32_NT`).
    pub platform_id: u32,
    /// Service pack text (`szCSDVersion`).
    pub csd: String,
    /// Service pack major.
    pub service_pack_major: u16,
    /// Service pack minor.
    pub service_pack_minor: u16,
    /// Suite mask.
    pub suite_mask: u16,
    /// Product type (1 = workstation).
    pub product_type: u8,
}

impl WindowsVersion {
    /// Windows 10 (10.0.19045), the default reported version. The vba-profile
    /// target carries the OS family and bitness but no OS version, so the
    /// version is a host-overridable constant, not derived from the real OS.
    pub fn windows10() -> Self {
        WindowsVersion {
            major: 10,
            minor: 0,
            build: 19045,
            platform_id: 2,
            csd: String::new(),
            service_pack_major: 0,
            service_pack_minor: 0,
            suite_mask: 0,
            product_type: 1,
        }
    }
}

/// Everything a shim may touch.
pub struct CallContext {
    /// The VBA-visible target (decides pointer width and whether Win32 exists).
    pub target: Target,
    /// Clock and sleep.
    pub env: Box<dyn HostEnv>,
    /// Injected identity.
    pub identity: Identity,
    /// Reported Windows version.
    pub version: WindowsVersion,
    /// The system ANSI code page (`CP_ACP`).
    pub ansi_codepage: Codepage,
    /// Emulated address space.
    pub memory: Memory,
    /// Handle table.
    pub handles: HandleTable,
    /// Emulated `GetLastError` slots.
    pub last_errors: LastErrorStore,
    /// The logical thread making the call.
    pub thread: LogicalThread,
    /// The windows `EnumWindows` and `EnumChildWindows` see (empty unless the
    /// embedding application adds some).
    pub windows: WindowList,
    /// Timers set with `SetTimer`.
    pub timers: TimerTable,
}

impl CallContext {
    /// Creates a context for `target` with Windows-1252 as ANSI code page and
    /// the default reported version.
    pub fn new(target: Target, env: Box<dyn HostEnv>, identity: Identity) -> Self {
        CallContext {
            target,
            env,
            identity,
            version: WindowsVersion::windows10(),
            ansi_codepage: Codepage::Windows1252,
            memory: Memory::new(target.bitness),
            handles: HandleTable::new(),
            last_errors: LastErrorStore::new(),
            thread: LogicalThread::default(),
            windows: WindowList::default(),
            timers: TimerTable::default(),
        }
    }

    /// `SetLastError` for the current logical thread.
    pub fn set_last_error(&mut self, code: u32) {
        self.last_errors.set(self.thread, code);
    }

    /// `GetLastError` for the current logical thread.
    pub fn last_error(&self) -> u32 {
        self.last_errors.get(self.thread)
    }
}
