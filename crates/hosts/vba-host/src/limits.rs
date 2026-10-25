//! Resource limits as data, and a meter that enforces them.

use std::cell::Cell;
use vba_automation::ErrInfo;

/// Default maximum call depth.
pub const DEFAULT_MAX_CALL_DEPTH: u32 = 1000;

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
    memory: Cell<u64>,
    depth: Cell<u32>,
}

impl Budget {
    /// A fresh meter.
    pub fn new(limits: ResourceLimits) -> Self {
        Budget {
            limits,
            steps: Cell::new(0),
            memory: Cell::new(0),
            depth: Cell::new(0),
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

    /// Bytes currently charged.
    pub fn memory_used(&self) -> u64 {
        self.memory.get()
    }

    /// Charges `n` steps.
    pub fn charge_steps(&self, n: u64) -> Result<(), LimitExceeded> {
        let total = self.steps.get().saturating_add(n);
        self.steps.set(total);
        match self.limits.max_steps {
            Some(max) if total > max => Err(LimitExceeded::Steps),
            _ => Ok(()),
        }
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
        let total = self.memory.get().saturating_add(bytes);
        if self.limits.max_memory_bytes.is_some_and(|m| total > m) {
            return Err(LimitExceeded::Memory);
        }
        self.memory.set(total);
        Ok(())
    }

    /// Returns memory to the budget.
    pub fn free(&self, bytes: u64) {
        self.memory.set(self.memory.get().saturating_sub(bytes));
    }
}
