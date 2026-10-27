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

/// Source budget of [`ResourceLimits::untrusted`]: total bytes of source
/// over all modules of a project (checked before anything is parsed). Real
/// projects are far smaller; the parse tree and bound tree of this much
/// source stay well inside the memory budget.
pub const UNTRUSTED_MAX_SOURCE_BYTES: u64 = 8 * 1024 * 1024;

/// Module budget of [`ResourceLimits::untrusted`].
pub const UNTRUSTED_MAX_MODULES: u64 = 1024;

/// Analysis budget of [`ResourceLimits::untrusted`]: bound statements and
/// expressions over the whole project.
pub const UNTRUSTED_MAX_ANALYSIS_NODES: u64 = 2_000_000;

/// Lowering budget of [`ResourceLimits::untrusted`]: IR instructions over
/// the whole program.
pub const UNTRUSTED_MAX_IR_INSTRUCTIONS: u64 = 2_000_000;

/// Wall-clock allowance of one call from the embedder that the facade and
/// `vbars run` give code that is not trusted: a minute, about twice what
/// [`UNTRUSTED_MAX_STEPS`] steps take.
pub const UNTRUSTED_CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Limits on one program run. `None` means unlimited.
///
/// The struct is `#[non_exhaustive]` so that new budgets can be added
/// without breaking embedders: start from [`ResourceLimits::unlimited`],
/// [`ResourceLimits::untrusted`] or `default()` and adjust with the
/// `with_*` methods.
///
/// The first three bound the run itself. The last four bound the work done
/// *before* it, when the project is analysed and lowered; a session builder
/// reads them from the host's limits and refuses a project that exceeds one
/// with a typed limit error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResourceLimits {
    /// Fuel: maximum interpreter steps.
    pub max_steps: Option<u64>,
    /// Maximum bytes of VBA-visible memory.
    pub max_memory_bytes: Option<u64>,
    /// Maximum procedure call depth.
    pub max_call_depth: u32,
    /// Maximum total bytes of source over all modules of a project.
    pub max_source_bytes: Option<u64>,
    /// Maximum number of modules in a project.
    pub max_modules: Option<u64>,
    /// Maximum bound statements and expressions in a project.
    pub max_analysis_nodes: Option<u64>,
    /// Maximum IR instructions in the lowered program.
    pub max_ir_instructions: Option<u64>,
}

impl ResourceLimits {
    /// Sets [`max_steps`](ResourceLimits::max_steps).
    #[must_use]
    pub const fn with_max_steps(mut self, n: Option<u64>) -> Self {
        self.max_steps = n;
        self
    }

    /// Sets [`max_memory_bytes`](ResourceLimits::max_memory_bytes).
    #[must_use]
    pub const fn with_max_memory_bytes(mut self, n: Option<u64>) -> Self {
        self.max_memory_bytes = n;
        self
    }

    /// Sets [`max_call_depth`](ResourceLimits::max_call_depth).
    #[must_use]
    pub const fn with_max_call_depth(mut self, n: u32) -> Self {
        self.max_call_depth = n;
        self
    }

    /// Sets [`max_source_bytes`](ResourceLimits::max_source_bytes).
    #[must_use]
    pub const fn with_max_source_bytes(mut self, n: Option<u64>) -> Self {
        self.max_source_bytes = n;
        self
    }

    /// Sets [`max_modules`](ResourceLimits::max_modules).
    #[must_use]
    pub const fn with_max_modules(mut self, n: Option<u64>) -> Self {
        self.max_modules = n;
        self
    }

    /// Sets [`max_analysis_nodes`](ResourceLimits::max_analysis_nodes).
    #[must_use]
    pub const fn with_max_analysis_nodes(mut self, n: Option<u64>) -> Self {
        self.max_analysis_nodes = n;
        self
    }

    /// Sets [`max_ir_instructions`](ResourceLimits::max_ir_instructions).
    #[must_use]
    pub const fn with_max_ir_instructions(mut self, n: Option<u64>) -> Self {
        self.max_ir_instructions = n;
        self
    }

    /// No step or memory limit; only the call depth is bounded. For code the
    /// embedder trusts, and for tests.
    pub const fn unlimited() -> Self {
        ResourceLimits {
            max_steps: None,
            max_memory_bytes: None,
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
            max_source_bytes: None,
            max_modules: None,
            max_analysis_nodes: None,
            max_ir_instructions: None,
        }
    }

    /// Finite limits for running code that is not trusted:
    /// [`UNTRUSTED_MAX_STEPS`] steps, [`UNTRUSTED_MAX_MEMORY_BYTES`] bytes and
    /// [`DEFAULT_MAX_CALL_DEPTH`] nested calls, and the project-size budgets
    /// ([`UNTRUSTED_MAX_SOURCE_BYTES`], [`UNTRUSTED_MAX_MODULES`],
    /// [`UNTRUSTED_MAX_ANALYSIS_NODES`], [`UNTRUSTED_MAX_IR_INSTRUCTIONS`]).
    /// Generous for real programs,
    /// finite for hostile ones. The facade's `Engine::new()` and `vbars run`
    /// start from these.
    pub const fn untrusted() -> Self {
        ResourceLimits {
            max_steps: Some(UNTRUSTED_MAX_STEPS),
            max_memory_bytes: Some(UNTRUSTED_MAX_MEMORY_BYTES),
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
            max_source_bytes: Some(UNTRUSTED_MAX_SOURCE_BYTES),
            max_modules: Some(UNTRUSTED_MAX_MODULES),
            max_analysis_nodes: Some(UNTRUSTED_MAX_ANALYSIS_NODES),
            max_ir_instructions: Some(UNTRUSTED_MAX_IR_INSTRUCTIONS),
        }
    }
}

/// The default is [`ResourceLimits::unlimited`]: the lower-level crates leave
/// the choice to the embedder. Use [`ResourceLimits::untrusted`] for code
/// that is not trusted.
impl Default for ResourceLimits {
    fn default() -> Self {
        ResourceLimits::unlimited()
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
    /// When a call scope ends by itself, on the budget's own clock (see
    /// [`Budget::suspend_deadline`]).
    deadline: Cell<Option<Instant>>,
    /// Wall-clock time the budget's clock was stopped for.
    paused: Cell<Duration>,
}

/// What a call scope narrows, see [`Budget::enter_scope`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScopeLimits {
    /// Steps the scope may spend from now on.
    pub max_steps: Option<u64>,
    /// Bytes the scope may add to what is charged now.
    pub max_memory_bytes: Option<u64>,
    /// How long the scope may run.
    pub timeout: Option<Duration>,
}

impl ScopeLimits {
    /// A scope that narrows nothing.
    pub const fn new() -> Self {
        ScopeLimits {
            max_steps: None,
            max_memory_bytes: None,
            timeout: None,
        }
    }

    /// Sets [`max_steps`](ScopeLimits::max_steps).
    #[must_use]
    pub const fn with_max_steps(mut self, n: Option<u64>) -> Self {
        self.max_steps = n;
        self
    }

    /// Sets [`max_memory_bytes`](ScopeLimits::max_memory_bytes).
    #[must_use]
    pub const fn with_max_memory_bytes(mut self, n: Option<u64>) -> Self {
        self.max_memory_bytes = n;
        self
    }

    /// Sets [`timeout`](ScopeLimits::timeout).
    #[must_use]
    pub const fn with_timeout(mut self, t: Option<Duration>) -> Self {
        self.timeout = t;
        self
    }
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

/// Holds a [`Budget`]'s deadline clock stopped; see
/// [`Budget::suspend_deadline`].
#[derive(Debug)]
#[must_use = "the clock runs again when the guard is dropped"]
pub struct ClockSuspension<'a> {
    budget: &'a Budget,
    since: Instant,
}

impl Drop for ClockSuspension<'_> {
    fn drop(&mut self) {
        let b = self.budget;
        b.paused.set(b.paused.get() + self.since.elapsed());
    }
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
            paused: Cell::new(Duration::ZERO),
        }
    }

    /// The budget's own clock: wall-clock time minus the time spent in
    /// [`suspend_deadline`](Budget::suspend_deadline) guards. Deadlines are
    /// kept on this clock, so a stopped clock does not burn a timeout.
    fn clock(&self) -> Instant {
        let now = Instant::now();
        now.checked_sub(self.paused.get()).unwrap_or(now)
    }

    /// Stops the clock deadlines are measured on until the guard is dropped.
    /// A host that hands control to a person (a debugger paused at a
    /// breakpoint, a modal dialog) holds the guard meanwhile, so the wait
    /// does not count against the call's wall-clock allowance. Steps and
    /// memory are not affected, and cancellation is still polled by the
    /// interpreter as usual.
    pub fn suspend_deadline(&self) -> ClockSuspension<'_> {
        ClockSuspension {
            budget: self,
            since: Instant::now(),
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
            // A timeout too large to represent as an instant is no deadline
            // at all (never earlier than the one already in force).
            if let Some(at) = self.clock().checked_add(t) {
                let at = self.deadline.get().map_or(at, |d| d.min(at));
                self.deadline.set(Some(at));
            }
        }
        token
    }

    /// Restores what [`enter_scope`](Budget::enter_scope) replaced.
    pub fn leave_scope(&self, token: ScopeToken) {
        self.step_ceiling.set(token.step_ceiling);
        self.memory.set_limit(token.memory_limit);
        self.deadline.set(token.deadline);
    }

    /// Whether a call scope set a deadline that is still in force.
    pub fn has_deadline(&self) -> bool {
        self.deadline.get().is_some()
    }

    /// Whether the deadline of a call scope has passed. A host that wants
    /// the timeout honoured returns an error from `poll_cancel` when this
    /// is true.
    pub fn deadline_passed(&self) -> bool {
        self.deadline.get().is_some_and(|d| self.clock() >= d)
    }

    /// Time left before the deadline of the call scope, `None` when no
    /// deadline is set, zero when it has passed. A host member that blocks
    /// (a network read, a child process, a sleep) should cap its wait with
    /// this, because the interpreter can only notice a passed deadline when
    /// the member returns.
    pub fn time_left(&self) -> Option<Duration> {
        self.deadline
            .get()
            .map(|d| d.saturating_duration_since(self.clock()))
    }

    /// The limits.
    pub fn limits(&self) -> &ResourceLimits {
        &self.limits
    }

    /// Steps charged so far.
    #[inline]
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
    #[inline]
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
        let prev = self.steps.get();
        let total = prev.saturating_add(n);
        let cap = match (self.limits.max_steps, self.step_ceiling.get()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        if let Some(cap) = cap
            && total > cap
        {
            // A bulk charge (work units, allocation volume) can jump far past
            // the limit: record at most one step over it, so the counter
            // never reads more than `limit + 1` however much was asked for.
            self.steps.set(total.min(cap.saturating_add(1)).max(prev));
            return Err(LimitExceeded::Steps);
        }
        self.steps.set(total);
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
    #[inline]
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
