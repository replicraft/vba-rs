//! A host with no application behind it.

use crate::capability::Policy;
use crate::events::{EventDispatch, EventRouter};
use crate::host::{DeclareRequest, Host, HostResult, ProjectReference};
use crate::limits::{Budget, ResourceLimits};
use crate::metadata::HostMetadata;
use crate::ui::{HeadlessUi, UiHooks};
use std::cell::RefCell;
use std::collections::BTreeMap;
use vba_automation::{ErrInfo, ObjectTable};
use vba_value::Variant;

/// A complete host that grants nothing by default and fails explicitly for
/// everything it cannot do: no objects can be created (429), no files opened
/// (432), no `Declare` resolved (48) and no UI shown (70). Only configured
/// globals resolve. Useful for running pure computation safely.
pub struct HeadlessHost {
    metadata: HostMetadata,
    policy: Policy,
    budget: Budget,
    table: ObjectTable,
    router: EventRouter,
    globals: RefCell<BTreeMap<String, Variant>>,
    references: Vec<ProjectReference>,
}

impl Default for HeadlessHost {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadlessHost {
    /// Default metadata, deny-all policy, default limits.
    pub fn new() -> Self {
        HeadlessHost {
            metadata: HostMetadata::headless(),
            policy: Policy::deny_all(),
            budget: Budget::new(ResourceLimits::default()),
            table: ObjectTable::new(),
            router: EventRouter::new(),
            globals: RefCell::default(),
            references: Vec::new(),
        }
    }

    /// Replaces the metadata.
    pub fn with_metadata(mut self, m: HostMetadata) -> Self {
        self.metadata = m;
        self
    }

    /// Replaces the policy.
    pub fn with_policy(mut self, p: Policy) -> Self {
        self.policy = p;
        self
    }

    /// Replaces the limits (resets the meter).
    pub fn with_limits(mut self, l: ResourceLimits) -> Self {
        self.budget = Budget::new(l);
        self
    }

    /// Defines a global value (name matched case-insensitively). An object
    /// value must already hold the reference the host keeps.
    pub fn with_global(self, name: &str, value: Variant) -> Self {
        self.globals.borrow_mut().insert(name.to_lowercase(), value);
        self
    }

    /// Adds a project reference.
    pub fn with_reference(mut self, r: ProjectReference) -> Self {
        self.references.push(r);
        self
    }

    /// The event router, to register handlers.
    pub fn router(&self) -> &EventRouter {
        &self.router
    }
}

impl Host for HeadlessHost {
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
        let v = self.globals.borrow().get(&name.to_lowercase()).cloned();
        if let Some(v) = &v {
            self.table.add_ref_value(v);
        }
        Ok(v)
    }

    fn create_object_hook(&self, progid: &str, _: Option<&str>) -> HostResult<Variant> {
        Err(ErrInfo::standard(429).with_source(format!("vba-host: no factory for '{progid}'")))
    }

    fn get_object_hook(&self, path: Option<&str>, class: Option<&str>) -> HostResult<Variant> {
        let what = path.or(class).unwrap_or_default();
        Err(ErrInfo::standard(432).with_source(format!("vba-host: cannot get '{what}'")))
    }

    fn declare_hook(&self, req: &mut DeclareRequest) -> HostResult<Variant> {
        Err(ErrInfo::standard(48).with_source(format!(
            "vba-host: no native library '{}' ({})",
            req.library, req.procedure
        )))
    }
}
