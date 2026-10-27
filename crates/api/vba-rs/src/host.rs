//! The host the [`Engine`](crate::Engine) builds for its sessions.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use vba_automation::{ErrInfo, ObjectTable};
use vba_host::{
    Budget, DeclareRequest, EventDispatch, EventRouter, HeadlessUi, Host, HostMetadata, HostResult,
    Policy, ProjectReference, ResourceLimits, UiHooks,
};
use vba_value::Variant;

/// Creates a host object (or any value) on demand. The returned value
/// carries one reference for the receiver (COM's out-parameter rule).
pub type Factory = Rc<dyn Fn(&ObjectTable) -> HostResult<Variant>>;

/// A cancellation request that may be raised from any thread.
///
/// The running session polls it, so a handler is stopped even in a loop
/// that never yields:
///
/// * before every call into the session (`Instance::call`, `run`,
///   `raise_event_on`... and every nested run);
/// * at every `DoEvents`;
/// * every [`POLL_INTERVAL`](vba_host::POLL_INTERVAL)
///   interpreter steps (basic blocks) of running code.
///
/// It cannot interrupt a single long operation inside the interpreter (a
/// huge string built in one expression) or a host member that blocks:
/// bound those with `ResourceLimits` and make host members return. A
/// cancellation ends the call with a classified, untrappable
/// `Error::Run(Cancelled)`; the token stays set until [`CancelToken::reset`],
/// and `Instance::is_usable` is false meanwhile.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token that is not cancelled.
    #[must_use]
    pub fn new() -> CancelToken {
        CancelToken::default()
    }

    /// Asks the sessions that share this token to stop at their next poll
    /// (see the type documentation). Idempotent.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Clears the request so the token can be reused.
    pub fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    /// Whether cancellation was requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// The host used by [`Engine`](crate::Engine): policy and limits from the
/// engine, host objects from registered factories, cooperative cancellation.
/// Everything else fails explicitly, like [`vba_host::HeadlessHost`].
pub(crate) struct EngineHost {
    pub(crate) metadata: HostMetadata,
    pub(crate) policy: Policy,
    pub(crate) budget: Budget,
    pub(crate) table: ObjectTable,
    pub(crate) router: EventRouter,
    pub(crate) globals: BTreeMap<String, Factory>,
    pub(crate) global_cache: RefCell<BTreeMap<String, Variant>>,
    pub(crate) classes: BTreeMap<String, Factory>,
    pub(crate) references: Vec<ProjectReference>,
    pub(crate) cancel: CancelToken,
}

impl EngineHost {
    pub(crate) fn new(
        metadata: HostMetadata,
        policy: Policy,
        limits: ResourceLimits,
        cancel: CancelToken,
    ) -> EngineHost {
        EngineHost {
            metadata,
            policy,
            budget: Budget::new(limits),
            table: ObjectTable::new(),
            router: EventRouter::new(),
            globals: BTreeMap::new(),
            global_cache: RefCell::default(),
            classes: BTreeMap::new(),
            references: Vec::new(),
            cancel,
        }
    }
}

impl Host for EngineHost {
    fn metadata(&self) -> &HostMetadata {
        &self.metadata
    }
    fn policy(&self) -> &Policy {
        &self.policy
    }
    fn budget(&self) -> &Budget {
        &self.budget
    }
    fn objects(&self) -> &ObjectTable {
        &self.table
    }
    fn ui(&self) -> &dyn UiHooks {
        &HeadlessUi
    }
    fn events(&self) -> &dyn EventDispatch {
        &self.router
    }
    fn references(&self) -> &[ProjectReference] {
        &self.references
    }

    fn lookup_global(&self, name: &str) -> HostResult<Option<Variant>> {
        let key = name.to_lowercase();
        let Some(factory) = self.globals.get(&key) else {
            return Ok(None);
        };
        let cached = self.global_cache.borrow().get(&key).cloned();
        if let Some(v) = cached {
            self.table.add_ref_value(&v);
            return Ok(Some(v));
        }
        let v = factory(&self.table)?;
        // One reference for the receiver, one kept by the cache.
        self.table.add_ref_value(&v);
        self.global_cache.borrow_mut().insert(key, v.clone());
        Ok(Some(v))
    }

    fn create_object_hook(&self, progid: &str, _server: Option<&str>) -> HostResult<Variant> {
        match self.classes.get(&progid.to_lowercase()) {
            Some(f) => f(&self.table),
            None => Err(ErrInfo::standard(429)
                .with_source(format!("vba: no host class registered for '{progid}'"))),
        }
    }

    fn get_object_hook(&self, path: Option<&str>, class: Option<&str>) -> HostResult<Variant> {
        if let Some(f) = class
            .filter(|c| !c.is_empty())
            .and_then(|c| self.classes.get(&c.to_lowercase()))
        {
            return f(&self.table);
        }
        let what = path.or(class).unwrap_or_default();
        Err(ErrInfo::standard(432).with_source(format!("vba: cannot get '{what}'")))
    }

    fn declare_hook(&self, req: &mut DeclareRequest) -> HostResult<Variant> {
        Err(ErrInfo::standard(48).with_source(format!(
            "vba: no native library '{}' ({})",
            req.library, req.procedure
        )))
    }

    fn idle(&self) -> HostResult<()> {
        self.poll_cancel()
    }

    fn poll_cancel(&self) -> HostResult<()> {
        if self.cancel.is_cancelled() {
            return Err(ErrInfo::new(18, "cancelled by the embedder"));
        }
        if self.budget.deadline_passed() {
            return Err(ErrInfo::new(18, "the call exceeded its time allowance"));
        }
        Ok(())
    }
}
