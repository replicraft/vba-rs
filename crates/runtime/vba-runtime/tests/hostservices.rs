//! Host-provided services behind `Date =`/`Time =`, the settings statements,
//! `SendKeys` and `AppActivate`: the default host refuses; an embedder's hooks
//! are reached only through the policy.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::*;
use vba_automation::ObjectTable;
use vba_host::{
    Budget, Capability, ClockPart, DeclareRequest, EventDispatch, HeadlessHost, Host, HostMetadata,
    HostResult, MemorySettings, Policy, SettingsStore, UiHooks,
};
use vba_runtime::Session;
use vba_value::{Date, Variant};
use vba_vm::BufferSink;

/// A headless host that records what its hooks receive.
struct Recording {
    inner: HeadlessHost,
    store: MemorySettings,
    use_store: bool,
    log: RefCell<Vec<String>>,
}

impl Recording {
    fn new(policy: Policy, use_store: bool) -> Recording {
        Recording {
            inner: HeadlessHost::new().with_policy(policy),
            store: MemorySettings::new(),
            use_store,
            log: RefCell::default(),
        }
    }
}

impl Host for Recording {
    fn metadata(&self) -> &HostMetadata {
        self.inner.metadata()
    }
    fn policy(&self) -> &Policy {
        self.inner.policy()
    }
    fn budget(&self) -> &Budget {
        self.inner.budget()
    }
    fn objects(&self) -> &ObjectTable {
        self.inner.objects()
    }
    fn ui(&self) -> &dyn UiHooks {
        self.inner.ui()
    }
    fn events(&self) -> &dyn EventDispatch {
        self.inner.events()
    }
    fn lookup_global(&self, name: &str) -> HostResult<Option<Variant>> {
        self.inner.lookup_global(name)
    }
    fn create_object_hook(&self, p: &str, s: Option<&str>) -> HostResult<Variant> {
        self.inner.create_object_hook(p, s)
    }
    fn get_object_hook(&self, p: Option<&str>, c: Option<&str>) -> HostResult<Variant> {
        self.inner.get_object_hook(p, c)
    }
    fn declare_hook(&self, r: &mut DeclareRequest) -> HostResult<Variant> {
        self.inner.declare_hook(r)
    }
    fn settings(&self) -> Option<&dyn SettingsStore> {
        self.use_store.then_some(&self.store as &dyn SettingsStore)
    }
    fn set_clock_hook(&self, part: ClockPart, value: Date) -> HostResult<()> {
        self.log
            .borrow_mut()
            .push(format!("{part:?} {}", value.serial()));
        Ok(())
    }
    fn send_keys_hook(&self, keys: &str, wait: bool) -> HostResult<()> {
        self.log.borrow_mut().push(format!("keys {keys} {wait}"));
        Ok(())
    }
    fn app_activate_hook(&self, title: &str, wait: bool) -> HostResult<()> {
        self.log
            .borrow_mut()
            .push(format!("activate {title} {wait}"));
        Ok(())
    }
}

fn run(host: Rc<Recording>, body: &str) -> Result<String, String> {
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .host(host)
        .debug_sink(sink.clone())
        .build_sources(&[std_module(&format!("Public Sub Main()\n{body}\nEnd Sub\n"))])
        .unwrap_or_else(|e| panic!("{e}"));
    s.trust();
    match s.run_procedure("Main", vec![]) {
        Ok(_) => Ok(sink.contents()),
        Err(e) => Err(e.to_string()),
    }
}

#[test]
fn clock_statements_reach_the_host_hook() {
    let h = Rc::new(Recording::new(Policy::deny_all(), false));
    run(
        h.clone(),
        "Date = DateSerial(2001, 2, 3)\nTime = TimeSerial(10, 0, 0)",
    )
    .unwrap();
    let log = h.log.borrow();
    assert_eq!(log.len(), 2);
    assert!(log[0].starts_with("Date "), "{log:?}");
    assert!(log[1].starts_with("Time "), "{log:?}");
}

#[test]
fn default_host_refuses_the_clock_with_error_70() {
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .debug_sink(sink)
        .build_sources(&[std_module(
            "Public Sub Main()\nDate = #2/3/2001#\nEnd Sub\n",
        )])
        .unwrap();
    s.trust();
    let e = s.run_procedure("Main", vec![]).unwrap_err().to_string();
    assert!(e.contains("70"), "{e}");
}

#[test]
fn ui_automation_needs_the_grant_and_a_hook() {
    // Denied by policy: the hook is not reached.
    let denied = Rc::new(Recording::new(Policy::deny_all(), false));
    let e = run(denied.clone(), "SendKeys \"abc\", True").unwrap_err();
    assert!(e.contains("70"), "{e}");
    assert!(denied.log.borrow().is_empty());
    // Granted: forwarded to the embedder's hook, nothing else happens.
    let granted = Rc::new(Recording::new(
        Policy::deny_all().grant(Capability::UiAutomation),
        false,
    ));
    run(
        granted.clone(),
        "SendKeys \"abc\", True\nAppActivate \"Notepad\"",
    )
    .unwrap();
    assert_eq!(
        *granted.log.borrow(),
        vec![
            "keys abc true".to_owned(),
            "activate Notepad false".to_owned()
        ]
    );
    // Granted but the default hook has no UI: error 70, no invented success.
    let mut s = Session::builder()
        .host(Rc::new(HeadlessHost::new().with_policy(
            Policy::deny_all().grant(Capability::UiAutomation),
        )))
        .build_sources(&[std_module("Public Sub Main()\nSendKeys \"x\"\nEnd Sub\n")])
        .unwrap();
    s.trust();
    let e = s.run_procedure("Main", vec![]).unwrap_err().to_string();
    assert!(e.contains("70"), "{e}");
}

#[test]
fn settings_use_the_hosts_store_when_it_has_one() {
    let h = Rc::new(Recording::new(Policy::deny_all(), true));
    let out = run(
        h.clone(),
        "SaveSetting \"App\", \"Sec\", \"k\", 42\nDebug.Print GetSetting(\"app\", \"sec\", \"K\")",
    )
    .unwrap();
    assert_eq!(out, "42\n");
    assert_eq!(
        h.store.get("App", "Sec", "k").unwrap().as_deref(),
        Some("42")
    );
}
