//! Resource limits as data, and a meter that enforces them.

use std::cell::Cell;
use std::time::{Duration, Instant};
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
    /// Absolute step count a call scope must not pass.
    step_ceiling: Cell<Option<u64>>,
    /// When a call scope ends by itself.
    deadline: Cell<Option<Instant>>,
}

/// What a call scope narrows, see [`Budget::enter_scope`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScopeLimits {
    /// Steps the scope may spend from now on.
    pub max_steps: Option<u64>,
    /// Bytes the scope may add to what is charged now.
    pub max_memory_bytes: Option<u64>,
    /// How long the scope may run.
    pub timeout: Option<Duration>,
}

/// The state [`Budget::enter_scope`] replaced; hand it to
/// [`Budget::leave_scope`].
#[derive(Debug)]
#[must_use = "pass it to Budget::leave_scope"]
pub struct ScopeToken {
    step_ceiling: Option<u64>,
    memory_limit: Option<u64>,
    deadline: Option<Instant>,
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
            step_ceiling: Cell::new(None),
            deadline: Cell::new(None),
        }
    }

    /// Narrows the limits for the duration of one call: `scope.max_steps`
    /// more steps, `scope.max_memory_bytes` more bytes than are charged now,
    /// and a deadline `scope.timeout` from now. A scope never widens what is
    /// already in force (the construction limits or an outer scope), and
    /// leaves the counters alone. Pair with [`leave_scope`](Budget::leave_scope).
    pub fn enter_scope(&self, scope: ScopeLimits) -> ScopeToken {
        let token = ScopeToken {
            step_ceiling: self.step_ceiling.get(),
            memory_limit: self.memory.limit(),
            deadline: self.deadline.get(),
        };
        if let Some(n) = scope.max_steps {
            let ceiling = self.steps.get().saturating_add(n);
            let ceiling = self.step_ceiling.get().map_or(ceiling, |c| c.min(ceiling));
            self.step_ceiling.set(Some(ceiling));
        }
        if let Some(n) = scope.max_memory_bytes {
            let limit = self.memory.used().saturating_add(n);
            let limit = self.memory.limit().map_or(limit, |l| l.min(limit));
            self.memory.set_limit(Some(limit));
        }
        if let Some(t) = scope.timeout {
            let at = Instant::now() + t;
            let at = self.deadline.get().map_or(at, |d| d.min(at));
            self.deadline.set(Some(at));
        }
        token
    }

    /// Restores what [`enter_scope`](Budget::enter_scope) replaced.
    pub fn leave_scope(&self, token: ScopeToken) {
        self.step_ceiling.set(token.step_ceiling);
        self.memory.set_limit(token.memory_limit);
        self.deadline.set(token.deadline);
    }

    /// Whether the deadline of a call scope has passed. A host that wants
    /// the timeout honoured returns an error from `poll_cancel` when this
    /// is true.
    pub fn deadline_passed(&self) -> bool {
        self.deadline.get().is_some_and(|d| Instant::now() >= d)
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

    /// Starts a fresh step allowance: the step counter returns to zero.
    /// Memory (returned by the values that own it) and call depth describe
    /// live state and are left alone.
    pub fn reset_steps(&self) {
        self.steps.set(0);
        self.work_pending.set(0);
    }

    /// Starts over after a session discarded its state: a fresh step
    /// allowance and a call depth of zero. Memory is not zeroed by hand: it
    /// is given back as the values that own it are dropped.
    pub fn reset(&self) {
        self.reset_steps();
        self.depth.set(0);
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
        match self.step_ceiling.get() {
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
