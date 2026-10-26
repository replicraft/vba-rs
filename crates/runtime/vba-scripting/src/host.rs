//! [`ScriptingHost`]: a host decorator that makes `CreateObject` know the
//! native Scripting classes.

use std::cell::RefCell;
use std::rc::Rc;

use vba_automation::{ErrInfo, ObjectRef, ObjectTable};
use vba_host::{
    Budget, Capability, ClockPart, DeclareRequest, EventDispatch, Host, HostMetadata, HostResult,
    Policy, ProjectReference, ResourceLimits, SettingsStore, UiHooks,
};
use vba_value::{Date, Variant};

use crate::dictionary::Dictionary;
use crate::fs::ScriptFs;
use crate::fso::FileSystemObject;

/// Wraps a host and answers `CreateObject("Scripting.Dictionary")` and
/// `CreateObject("Scripting.FileSystemObject")` with the native objects of
/// this crate.
///
/// Everything else is the inner host's: the policy (so creating the objects
/// needs `Capability::ComCreate` for the ProgID, and any file access they do
/// needs `FileRead` / `FileWrite` through the [`ScriptFs`]), the limits, the
/// object table, the globals and every hook. A factory the inner host
/// registers for the same ProgID wins: the native class is used only when
/// the inner host answers error 429 ("no such class").
pub struct ScriptingHost {
    inner: Rc<dyn Host>,
    fs: RefCell<Option<Rc<dyn ScriptFs>>>,
}

impl ScriptingHost {
    /// Decorates `inner`. Until [`ScriptingHost::set_fs`] is called the
    /// `FileSystemObject` is unavailable (error 429).
    pub fn new(inner: Rc<dyn Host>) -> Rc<ScriptingHost> {
        Rc::new(ScriptingHost {
            inner,
            fs: RefCell::new(None),
        })
    }

    /// Provides the file system behind `Scripting.FileSystemObject`.
    pub fn set_fs(&self, fs: Rc<dyn ScriptFs>) {
        *self.fs.borrow_mut() = Some(fs);
    }

    /// The decorated host.
    pub fn inner(&self) -> &Rc<dyn Host> {
        &self.inner
    }

    fn native(&self, table: &ObjectTable, progid: &str) -> Option<Variant> {
        let make = |v: Variant| v;
        if progid.eq_ignore_ascii_case("Scripting.Dictionary") {
            let h = table.insert_obj(Dictionary::new());
            return Some(make(Variant::Object(ObjectRef::Handle(h))));
        }
        if progid.eq_ignore_ascii_case("Scripting.FileSystemObject") {
            let fs = self.fs.borrow().clone()?;
            let h = table.insert_obj(FileSystemObject::new(fs));
            return Some(make(Variant::Object(ObjectRef::Handle(h))));
        }
        None
    }
}

impl Host for ScriptingHost {
    fn metadata(&self) -> &HostMetadata {
        self.inner.metadata()
    }
    fn policy(&self) -> &Policy {
        self.inner.policy()
    }
    fn limits(&self) -> &ResourceLimits {
        self.inner.limits()
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
    fn references(&self) -> &[ProjectReference] {
        self.inner.references()
    }
    fn settings(&self) -> Option<&dyn SettingsStore> {
        self.inner.settings()
    }

    fn lookup_global(&self, name: &str) -> HostResult<Option<Variant>> {
        self.inner.lookup_global(name)
    }
    fn create_object_hook(&self, progid: &str, server: Option<&str>) -> HostResult<Variant> {
        self.inner.create_object_hook(progid, server)
    }
    fn get_object_hook(&self, path: Option<&str>, class: Option<&str>) -> HostResult<Variant> {
        self.inner.get_object_hook(path, class)
    }
    fn declare_hook(&self, req: &mut DeclareRequest) -> HostResult<Variant> {
        self.inner.declare_hook(req)
    }
    fn idle(&self) -> HostResult<()> {
        self.inner.idle()
    }
    fn poll_cancel(&self) -> HostResult<()> {
        self.inner.poll_cancel()
    }
    fn set_clock_hook(&self, part: ClockPart, value: Date) -> HostResult<()> {
        self.inner.set_clock_hook(part, value)
    }
    fn send_keys_hook(&self, keys: &str, wait: bool) -> HostResult<()> {
        self.inner.send_keys_hook(keys, wait)
    }
    fn app_activate_hook(&self, title: &str, wait: bool) -> HostResult<()> {
        self.inner.app_activate_hook(title, wait)
    }

    fn check(&self, cap: Capability, resource: &str) -> HostResult<()> {
        self.inner.check(cap, resource)
    }
    fn resolve_global(&self, name: &str) -> HostResult<Option<Variant>> {
        self.inner.resolve_global(name)
    }
    fn create_object(&self, progid: &str, server: Option<&str>) -> HostResult<Variant> {
        // The inner host checks the policy and consults its factories; only
        // its "no such class" answer falls through to the native classes.
        match self.inner.create_object(progid, server) {
            Err(ErrInfo { number: 429, .. }) if server.is_none_or(str::is_empty) => {
                match self.native(self.inner.objects(), progid) {
                    Some(v) => Ok(v),
                    None => self.inner.create_object(progid, server),
                }
            }
            other => other,
        }
    }
    fn get_object(&self, path: Option<&str>, class: Option<&str>) -> HostResult<Variant> {
        self.inner.get_object(path, class)
    }
    fn call_declare(&self, req: &mut DeclareRequest) -> HostResult<Variant> {
        self.inner.call_declare(req)
    }
    fn set_clock(&self, part: ClockPart, value: Date) -> HostResult<()> {
        self.inner.set_clock(part, value)
    }
    fn send_keys(&self, keys: &str, wait: bool) -> HostResult<()> {
        self.inner.send_keys(keys, wait)
    }
    fn app_activate(&self, title: &str, wait: bool) -> HostResult<()> {
        self.inner.app_activate(title, wait)
    }
    fn do_events(&self) -> HostResult<i32> {
        self.inner.do_events()
    }
}
