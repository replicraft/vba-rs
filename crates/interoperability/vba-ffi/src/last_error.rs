//! Emulated `GetLastError` / `SetLastError`.
//!
//! The host's own last-error value is never read or written. Each logical VBA
//! thread has an independent slot, starting at 0 (`ERROR_SUCCESS`).

use std::collections::HashMap;

/// Identifier of a logical VBA thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LogicalThread(pub u32);

/// Per-logical-thread last-error values.
#[derive(Debug, Default)]
pub struct LastErrorStore {
    slots: HashMap<LogicalThread, u32>,
}

impl LastErrorStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// `SetLastError`.
    pub fn set(&mut self, thread: LogicalThread, code: u32) {
        self.slots.insert(thread, code);
    }

    /// `GetLastError`.
    pub fn get(&self, thread: LogicalThread) -> u32 {
        self.slots.get(&thread).copied().unwrap_or(0)
    }
}
