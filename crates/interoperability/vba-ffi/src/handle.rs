//! Opaque handles with provider affinity.
//!
//! A [`Handle`] can only be created by [`HandleTable::mint`]; its fields are
//! private, so neither VBA code nor another provider can forge one. The table
//! additionally records which provider minted each token and rejects any
//! other provider that presents it, whether as a `Handle` or as the bare
//! integer VBA code sees ([`Handle::value`]).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

static NEXT_TABLE: AtomicU64 = AtomicU64::new(1);

/// Identifies a provider (for example `"native-shim"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProviderId(pub &'static str);

/// Why a handle was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HandleError {
    /// The handle was minted by a different provider.
    #[error("handle belongs to provider `{owner}`, not `{caller}`")]
    WrongProvider {
        /// Provider that minted it.
        owner: &'static str,
        /// Provider that presented it.
        caller: &'static str,
    },
    /// The handle belongs to a different table (different VBA process).
    #[error("handle belongs to a different handle table")]
    ForeignTable,
    /// The token is unknown or has been closed.
    #[error("invalid or closed handle")]
    Invalid,
}

/// An opaque handle. Not constructible outside [`HandleTable::mint`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle {
    table: u64,
    provider: ProviderId,
    token: u64,
}

impl Handle {
    /// The integer VBA code sees (a `LongPtr`-sized, non-zero multiple of 4).
    pub const fn value(&self) -> u64 {
        self.token
    }

    /// The minting provider.
    pub const fn provider(&self) -> ProviderId {
        self.provider
    }
}

#[derive(Debug)]
struct Entry {
    provider: ProviderId,
    kind: &'static str,
    /// Charged to the run's memory meter while the handle is live.
    _charge: vba_core::Charge,
}

/// Per-process table of live handles.
#[derive(Debug)]
pub struct HandleTable {
    id: u64,
    next: u64,
    live: HashMap<u64, Entry>,
}

impl Default for HandleTable {
    fn default() -> Self {
        Self::new()
    }
}

impl HandleTable {
    /// Creates an empty table.
    pub fn new() -> Self {
        HandleTable {
            id: NEXT_TABLE.fetch_add(1, Ordering::Relaxed),
            next: 1,
            live: HashMap::new(),
        }
    }

    /// Mints a handle of `kind` owned by `provider`. Tokens are never reused.
    pub fn mint(&mut self, provider: ProviderId, kind: &'static str) -> Handle {
        let token = 0x100 + self.next * 4;
        self.next += 1;
        self.live.insert(
            token,
            Entry {
                provider,
                kind,
                _charge: vba_core::Charge::new(64),
            },
        );
        Handle {
            table: self.id,
            provider,
            token,
        }
    }

    /// Validates `handle` for `caller` and returns its kind.
    pub fn resolve(
        &self,
        caller: ProviderId,
        handle: &Handle,
    ) -> Result<&'static str, HandleError> {
        if handle.table != self.id {
            return Err(HandleError::ForeignTable);
        }
        self.resolve_value(caller, handle.token)
    }

    /// Validates the bare integer a VBA program passed back.
    pub fn resolve_value(
        &self,
        caller: ProviderId,
        value: u64,
    ) -> Result<&'static str, HandleError> {
        let e = self.live.get(&value).ok_or(HandleError::Invalid)?;
        if e.provider != caller {
            return Err(HandleError::WrongProvider {
                owner: e.provider.0,
                caller: caller.0,
            });
        }
        Ok(e.kind)
    }

    /// Closes a handle (only its owner may).
    pub fn close(&mut self, caller: ProviderId, handle: &Handle) -> Result<(), HandleError> {
        self.resolve(caller, handle)?;
        self.live.remove(&handle.token);
        Ok(())
    }

    /// Number of live handles.
    pub fn len(&self) -> usize {
        self.live.len()
    }

    /// Whether no handle is live.
    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }
}
