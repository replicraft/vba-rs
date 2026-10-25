//! A configurable host for tests.

use crate::capability::Policy;
use crate::events::{EventDispatch, EventRouter};
use crate::host::{DeclareRequest, Host, HostResult, ProjectReference};
use crate::limits::{Budget, ResourceLimits};
use crate::metadata::HostMetadata;
use crate::ui::{InputBoxRequest, MsgBoxRequest, UiHooks};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use vba_automation::{ErrInfo, ObjectTable};
use vba_value::Variant;

type Factory = Rc<dyn Fn(&ObjectTable) -> HostResult<Variant>>;
type DeclareFn = Rc<dyn Fn(&mut DeclareRequest) -> HostResult<Variant>>;

/// Scripted UI answers, consumed in order. When a queue is empty the call
/// fails with error 5 rather than guessing.
#[derive(Default)]
pub struct ScriptedUi {
    msg_answers: RefCell<VecDeque<i32>>,
    input_answers: RefCell<VecDeque<Option<String>>>,
    /// Every prompt shown, in order.
    pub prompts: RefCell<Vec<String>>,
}

impl UiHooks for ScriptedUi {
    fn msg_box(&self, req: &MsgBoxRequest) -> HostResult<i32> {
        self.prompts.borrow_mut().push(req.prompt.clone());
        self.msg_answers
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| ErrInfo::new(5, "ScriptedHost has no scripted MsgBox answer left"))
    }

    fn input_box(&self, req: &InputBoxRequest) -> HostResult<Option<String>> {
        self.prompts.borrow_mut().push(req.prompt.clone());
        self.input_answers
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| ErrInfo::new(5, "ScriptedHost has no scripted InputBox answer left"))
    }
}

/// A host whose behaviour is configured by the test: globals, object
/// factories per ProgID, `Declare` implementations, UI answers, an idle
/// cancellation point, and a log of every hook call.
pub struct ScriptedHost {
    metadata: HostMetadata,
    policy: Policy,
    budget: Budget,
    table: ObjectTable,
    router: EventRouter,
    ui: ScriptedUi,
    globals: BTreeMap<String, Variant>,
    factories: BTreeMap<String, Factory>,
    objects_by_name: BTreeMap<String, Factory>,
    declares: BTreeMap<String, DeclareFn>,
    references: Vec<ProjectReference>,
    cancel_after_idles: Option<u32>,
    idles: RefCell<u32>,
    log: RefCell<Vec<String>>,
}

impl Default for ScriptedHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptedHost {
    /// Headless-like defaults: deny-all policy, nothing configured.
    pub fn new() -> Self {
        ScriptedHost {
            metadata: HostMetadata::new("scripted", "0", HostMetadata::headless().profile),
            policy: Policy::deny_all(),
            budget: Budget::new(ResourceLimits::default()),
            table: ObjectTable::new(),
            router: EventRouter::new(),
            ui: ScriptedUi::default(),
            globals: BTreeMap::new(),
            factories: BTreeMap::new(),
            objects_by_name: BTreeMap::new(),
            declares: BTreeMap::new(),
            references: Vec::new(),
            cancel_after_idles: None,
            idles: RefCell::new(0),
            log: RefCell::default(),
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

    /// Replaces the limits.
    pub fn with_limits(mut self, l: ResourceLimits) -> Self {
        self.budget = Budget::new(l);
        self
    }

    /// Defines a global; objects should already hold the host's reference.
    pub fn with_global(mut self, name: &str, v: Variant) -> Self {
        self.globals.insert(name.to_lowercase(), v);
        self
    }

    /// Adds a project reference.
    pub fn with_reference(mut self, r: ProjectReference) -> Self {
        self.references.push(r);
        self
    }

    /// Registers a `CreateObject` factory for a ProgID.
    pub fn with_factory(
        mut self,
        progid: &str,
        f: impl Fn(&ObjectTable) -> HostResult<Variant> + 'static,
    ) -> Self {
        self.factories.insert(progid.to_lowercase(), Rc::new(f));
        self
    }

    /// Registers a `GetObject` target (a path or a class name).
    pub fn with_named_object(
        mut self,
        name: &str,
        f: impl Fn(&ObjectTable) -> HostResult<Variant> + 'static,
    ) -> Self {
        self.objects_by_name.insert(name.to_lowercase(), Rc::new(f));
        self
    }

    /// Registers a `Declare` implementation for `library!procedure`.
    pub fn with_declare(
        mut self,
        library: &str,
        procedure: &str,
        f: impl Fn(&mut DeclareRequest) -> HostResult<Variant> + 'static,
    ) -> Self {
        self.declares
            .insert(format!("{library}!{procedure}").to_lowercase(), Rc::new(f));
        self
    }

    /// Queues `MsgBox` answers.
    pub fn with_msg_box_answers(self, answers: impl IntoIterator<Item = i32>) -> Self {
        self.ui.msg_answers.borrow_mut().extend(answers);
        self
    }

    /// Queues `InputBox` answers (`None` = cancel).
    pub fn with_input_box_answers(self, answers: impl IntoIterator<Item = Option<String>>) -> Self {
        self.ui.input_answers.borrow_mut().extend(answers);
        self
    }

    /// Makes the `n`-th idle safe point (and later ones) fail with error 18,
    /// simulating a user cancellation.
    pub fn cancel_after_idles(mut self, n: u32) -> Self {
        self.cancel_after_idles = Some(n);
        self
    }

    /// The event router.
    pub fn router(&self) -> &EventRouter {
        &self.router
    }

    /// The scripted UI, to inspect prompts.
    pub fn scripted_ui(&self) -> &ScriptedUi {
        &self.ui
    }

    /// Every hook call so far, e.g. `"CreateObject:Scripting.Dictionary"`.
    pub fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    fn note(&self, s: String) {
        self.log.borrow_mut().push(s);
    }
}

impl Host for ScriptedHost {
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
        &self.ui
    }
    fn events(&self) -> &dyn EventDispatch {
        &self.router
    }
    fn references(&self) -> &[ProjectReference] {
        &self.references
    }

    fn lookup_global(&self, name: &str) -> HostResult<Option<Variant>> {
        self.note(format!("Global:{name}"));
        let v = self.globals.get(&name.to_lowercase()).cloned();
        if let Some(v) = &v {
            self.table.add_ref_value(v);
        }
        Ok(v)
    }

    fn create_object_hook(&self, progid: &str, _: Option<&str>) -> HostResult<Variant> {
        self.note(format!("CreateObject:{progid}"));
        match self.factories.get(&progid.to_lowercase()) {
            Some(f) => f(&self.table),
            None => Err(ErrInfo::standard(429)),
        }
    }

    fn get_object_hook(&self, path: Option<&str>, class: Option<&str>) -> HostResult<Variant> {
        let key = path.or(class).unwrap_or_default();
        self.note(format!("GetObject:{key}"));
        match self.objects_by_name.get(&key.to_lowercase()) {
            Some(f) => f(&self.table),
            None => Err(ErrInfo::standard(432)),
        }
    }

    fn declare_hook(&self, req: &mut DeclareRequest) -> HostResult<Variant> {
        let key = format!("{}!{}", req.library, req.procedure);
        self.note(format!("Declare:{key}"));
        match self.declares.get(&key.to_lowercase()) {
            Some(f) => f(req),
            None => Err(ErrInfo::standard(453)),
        }
    }

    fn idle(&self) -> HostResult<()> {
        self.note("Idle".into());
        let mut n = self.idles.borrow_mut();
        *n += 1;
        match self.cancel_after_idles {
            Some(limit) if *n >= limit => Err(ErrInfo::standard(18)),
            _ => Ok(()),
        }
    }
}
