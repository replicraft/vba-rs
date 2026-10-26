//! Resource limits as data, and a meter that enforces them.

use std::cell::Cell;
use vba_automation::ErrInfo;
use vba_core::{MemoryMeter, MeterScope};

/// Default maximum call depth.
pub const DEFAULT_MAX_CALL_DEPTH: u32 = 1000;

/// Step budget of [`ResourceLimits::untrusted`]: about half a minute of
/// ordinary statements.
pub const UNTRUSTED_MAX_STEPS: u64 = 100_000_000;

/// Memory budget of [`ResourceLimits::untrusted`], in bytes.
pub const UNTRUSTED_MAX_MEMORY_BYTES: u64 = 256 * 1024 * 1024;

/// Limits on one program run. `None` means unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceLimits {
    /// Fuel: maximum interpreter steps.
    pub max_steps: Option<u64>,
    /// Maximum bytes of VBA-visible memory.
    pub max_memory_bytes: Option<u64>,
    /// Maximum procedure call depth.
    pub max_call_depth: u32,
}

impl ResourceLimits {
    /// No step or memory limit; only the call depth is bounded. For code the
    /// embedder trusts, and for tests.
    pub const fn unlimited() -> Self {
        ResourceLimits {
            max_steps: None,
            max_memory_bytes: None,
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
        }
    }

    /// Finite limits for running code that is not trusted:
    /// [`UNTRUSTED_MAX_STEPS`] steps, [`UNTRUSTED_MAX_MEMORY_BYTES`] bytes and
    /// [`DEFAULT_MAX_CALL_DEPTH`] nested calls. Generous for real programs,
    /// finite for hostile ones. The facade's `Engine::new()` and `vbars run`
    /// start from these.
    pub const fn untrusted() -> Self {
        ResourceLimits {
            max_steps: Some(UNTRUSTED_MAX_STEPS),
            max_memory_bytes: Some(UNTRUSTED_MAX_MEMORY_BYTES),
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
        }
    }
}

/// The default is [`ResourceLimits::unlimited`]: the lower-level crates leave
/// the choice to the embedder. Use [`ResourceLimits::untrusted`] for code
/// that is not trusted.
impl Default for ResourceLimits {
    fn default() -> Self {
        ResourceLimits {
            max_steps: None,
            max_memory_bytes: None,
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
        }
    }
}

/// Which limit was hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitExceeded {
    /// The step budget is spent.
    Steps,
    /// The memory budget is spent.
    Memory,
    /// Calls nested too deeply.
    CallDepth,
}

impl LimitExceeded {
    /// The VBA error: call depth is 28 "Out of stack space", memory is 7 "Out
    /// of memory", and an exhausted step budget is 18 "User interrupt
    /// occurred" (the nearest documented equivalent of a forced stop).
    pub fn into_err_info(self) -> ErrInfo {
        let n = match self {
            LimitExceeded::Steps => 18,
            LimitExceeded::Memory => 7,
            LimitExceeded::CallDepth => 28,
        };
        ErrInfo::standard(n).with_source("vba-host: resource limit")
    }
}

impl From<LimitExceeded> for ErrInfo {
    fn from(l: LimitExceeded) -> Self {
        l.into_err_info()
    }
}

/// Counters enforcing a [`ResourceLimits`].
#[derive(Debug)]
pub struct Budget {
    limits: ResourceLimits,
    steps: Cell<u64>,
    memory: MemoryMeter,
    depth: Cell<u32>,
    /// Allocation volume already paid for in steps.
    volume_paid: Cell<u64>,
    /// Scanning work not yet worth a whole step.
    work_pending: Cell<u64>,
}

/// Bytes built per step: data a program builds costs CPU as well as memory,
/// so every this many bytes of tracked allocation (strings, arrays, objects,
/// file buffers) are charged as one step.
pub const ALLOC_BYTES_PER_STEP: u64 = 1024;

/// Units of scanning work per step: a built-in that examines text without
/// building it (`Like`, a search) charges one unit per UTF-16 unit it looks
/// at.
pub const WORK_UNITS_PER_STEP: u64 = 64;

impl Budget {
    /// A fresh meter.
    pub fn new(limits: ResourceLimits) -> Self {
        Budget {
            limits,
            steps: Cell::new(0),
            memory: MemoryMeter::new(limits.max_memory_bytes),
            depth: Cell::new(0),
            volume_paid: Cell::new(0),
            work_pending: Cell::new(0),
        }
    }

    /// The limits.
    pub fn limits(&self) -> &ResourceLimits {
        &self.limits
    }

    /// Steps charged so far.
    pub fn steps_used(&self) -> u64 {
        self.steps.get()
    }

    /// Current call depth.
    pub fn depth(&self) -> u32 {
        self.depth.get()
    }

    /// Bytes currently charged: strings, arrays, objects, files and every
    /// other owner that registers with the meter.
    pub fn memory_used(&self) -> u64 {
        self.memory.used()
    }

    /// The shared meter behind `max_memory_bytes`. Owners of heap-like data
    /// outside the interpreter (an in-memory file system, a dictionary, an
    /// output buffer) charge their bytes to it.
    pub fn meter(&self) -> &MemoryMeter {
        &self.memory
    }

    /// Makes this budget's meter the current one on this thread, so that
    /// values created meanwhile are charged to it. The interpreter does this
    /// for the duration of every entry point.
    pub fn enter(&self) -> MeterScope {
        self.memory.enter()
    }

    /// Charges `n` steps.
    pub fn charge_steps(&self, n: u64) -> Result<(), LimitExceeded> {
        let built = self.memory.allocated_total();
        let unpaid = built - self.volume_paid.get();
        let mut n = n;
        if unpaid >= ALLOC_BYTES_PER_STEP {
            let whole = unpaid / ALLOC_BYTES_PER_STEP;
            self.volume_paid
                .set(self.volume_paid.get() + whole * ALLOC_BYTES_PER_STEP);
            n = n.saturating_add(whole);
        }
        let total = self.steps.get().saturating_add(n);
        self.steps.set(total);
        match self.limits.max_steps {
            Some(max) if total > max => return Err(LimitExceeded::Steps),
            _ => {}
        }
        if self.memory.exceeded() {
            return Err(LimitExceeded::Memory);
        }
        Ok(())
    }

    /// Charges `units` of scanning work, [`WORK_UNITS_PER_STEP`] to a step.
    /// The remainder is carried over, so many small calls add up.
    pub fn charge_work(&self, units: u64) -> Result<(), LimitExceeded> {
        let pending = self.work_pending.get().saturating_add(units);
        self.work_pending.set(pending % WORK_UNITS_PER_STEP);
        self.charge_steps(pending / WORK_UNITS_PER_STEP)
    }

    /// Charges one step.
    pub fn charge_step(&self) -> Result<(), LimitExceeded> {
        self.charge_steps(1)
    }

    /// Enters a call. On failure the depth is unchanged.
    pub fn enter_call(&self) -> Result<(), LimitExceeded> {
        let d = self.depth.get();
        if d >= self.limits.max_call_depth {
            return Err(LimitExceeded::CallDepth);
        }
        self.depth.set(d + 1);
        Ok(())
    }

    /// Leaves a call.
    pub fn leave_call(&self) {
        self.depth.set(self.depth.get().saturating_sub(1));
    }

    /// Charges an allocation. On failure nothing is charged.
    pub fn alloc(&self, bytes: u64) -> Result<(), LimitExceeded> {
        self.memory
            .try_alloc(bytes)
            .map_err(|_| LimitExceeded::Memory)
    }

    /// Returns memory to the budget.
    pub fn free(&self, bytes: u64) {
        self.memory.free(bytes);
    }

    /// Fails if `bytes` more would not fit, without charging them. Used
    /// before building something whose size is known in advance, so that the
    /// allocation itself never happens.
    pub fn check_room(&self, bytes: u64) -> Result<(), LimitExceeded> {
        match self.memory.limit() {
            Some(l) if self.memory.used().saturating_add(bytes) > l => Err(LimitExceeded::Memory),
            _ => Ok(()),
        }
    }
}
