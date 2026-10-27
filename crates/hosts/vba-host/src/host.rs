//! The [`Host`] trait.

use crate::capability::{Capability, Policy};
use crate::events::EventDispatch;
use crate::limits::{Budget, ResourceLimits};
use crate::metadata::HostMetadata;
use crate::settings::SettingsStore;
use crate::ui::UiHooks;
use vba_automation::{Arg, ErrInfo, ObjectTable};
use vba_value::{Date, VarType, Variant};

/// How many interpreter steps pass between two calls of
/// [`Host::poll_cancel`].
pub const POLL_INTERVAL: u32 = 1024;

/// Result of a host operation; errors are the VBA `Err` contents.
pub type HostResult<T> = Result<T, ErrInfo>;

/// Which part of the system clock the `Date =` and `Time =` statements set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockPart {
    /// `Date = value`
    Date,
    /// `Time = value`
    Time,
}

/// A project-level reference (`References` collection entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectReference {
    /// Reference name, e.g. `"Scripting"`.
    pub name: String,
    /// Library GUID, if known.
    pub guid: Option<String>,
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
}

/// A `Declare` call, opaque to this crate: the host decides what the library
/// and procedure names mean.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclareRequest {
    /// Library name from `Lib "..."`.
    pub library: String,
    /// Procedure name (after `Alias`, if any).
    pub procedure: String,
    /// Arguments; ByRef ones are written back by the host.
    pub args: Vec<Arg>,
    /// Declared return type; `None` for a `Sub`.
    pub return_type: Option<VarType>,
}

/// What an embedder implements.
///
/// Required methods are state accessors and hooks. The provided methods are
/// the entry points the interpreter calls: they enforce the [`Policy`] before
/// any hook runs, so deny-by-default cannot be bypassed by a hook that
/// forgets to check.
pub trait Host {
    /// Name, version and VBA profile.
    fn metadata(&self) -> &HostMetadata;
    /// The permission policy.
    fn policy(&self) -> &Policy;
    /// The resource limits.
    fn limits(&self) -> &ResourceLimits {
        self.budget().limits()
    }
    /// The meter enforcing the limits.
    fn budget(&self) -> &Budget;
    /// The automation object table for this host.
    fn objects(&self) -> &ObjectTable;
    /// MsgBox and InputBox.
    fn ui(&self) -> &dyn UiHooks;
    /// Event delivery.
    fn events(&self) -> &dyn EventDispatch;
    /// Project-level references.
    fn references(&self) -> &[ProjectReference] {
        &[]
    }

    /// Hook: resolves a global name (`Application`, host intrinsics, objects
    /// exported by project references). `Ok(None)` means "not a global". An
    /// object result carries a reference owned by the caller.
    fn lookup_global(&self, name: &str) -> HostResult<Option<Variant>>;
    /// Hook: calls a global function the embedding application provides
    /// (a function its `TypeProvider` reports as a global function, such
    /// as `Nz` or `DLookup`). Called for a name that is neither a project
    /// procedure nor a built-in function; `args` holds one entry per
    /// declared parameter in order, `None` for an omitted one. `Ok(None)`
    /// means "not a host function" and the call fails as an unknown
    /// function. The default provides none.
    fn call_function(&self, _name: &str, _args: &[Option<Variant>]) -> HostResult<Option<Variant>> {
        Ok(None)
    }
    /// Hook for `CreateObject(progid, server)`; only called when ComCreate is
    /// granted.
    fn create_object_hook(&self, progid: &str, server: Option<&str>) -> HostResult<Variant>;
    /// Hook for `GetObject(path, class)`; only called when permitted.
    fn get_object_hook(&self, path: Option<&str>, class: Option<&str>) -> HostResult<Variant>;
    /// Hook for a `Declare`d procedure; only called when NativeDeclare is
    /// granted for `library!procedure`.
    fn declare_hook(&self, req: &mut DeclareRequest) -> HostResult<Variant>;
    /// The persistent store behind `SaveSetting` and friends. `None` (the
    /// default) makes the interpreter use its own in-memory store.
    fn settings(&self) -> Option<&dyn SettingsStore> {
        None
    }
    /// Hook for the `Date =` / `Time =` statements. The default refuses with
    /// error 70: a sandboxed program must not change the system clock.
    fn set_clock_hook(&self, part: ClockPart, _value: Date) -> HostResult<()> {
        Err(ErrInfo::new(
            70,
            format!(
                "setting the system {} is not available",
                match part {
                    ClockPart::Date => "date",
                    ClockPart::Time => "time",
                }
            ),
        )
        .with_source("vba-host: system clock is read-only"))
    }
    /// Hook for `SendKeys`; only called when [`Capability::UiAutomation`] is
    /// granted. The default has no UI to drive and fails with error 70; the
    /// interpreter never sends input on its own.
    fn send_keys_hook(&self, _keys: &str, _wait: bool) -> HostResult<()> {
        Err(
            ErrInfo::new(70, "SendKeys is not available: the host has no UI to drive")
                .with_source("vba-host: no UI automation provider"),
        )
    }
    /// Hook for `AppActivate`; see [`Host::send_keys_hook`].
    fn app_activate_hook(&self, _title: &str, _wait: bool) -> HostResult<()> {
        Err(ErrInfo::new(
            70,
            "AppActivate is not available: the host has no UI to drive",
        )
        .with_source("vba-host: no UI automation provider"))
    }
    /// Hook run at every idle safe point (`DoEvents`); an error aborts the
    /// run (for example a cancellation request).
    fn idle(&self) -> HostResult<()> {
        Ok(())
    }
    /// Hook polled by the interpreter every [`POLL_INTERVAL`] steps, whatever
    /// the program is doing, so that a loop without `DoEvents` can be
    /// cancelled from another thread. It must be cheap (an atomic load); an
    /// error aborts the run. Unlike [`Host::idle`] it carries no other
    /// meaning, so a host that cancels in `idle` should cancel here too. The interpreter also polls it once at the
    /// start of every call made by the embedder, nested ones included. The default
    /// honours the time allowance of a call scope
    /// ([`Budget::deadline_passed`]); library calls that run long poll it
    /// between chunks of work too.
    fn poll_cancel(&self) -> HostResult<()> {
        if self.budget().deadline_passed() {
            return Err(ErrInfo::new(18, "the call exceeded its time allowance"));
        }
        Ok(())
    }

    /// Checks a capability request; denial is error 70.
    fn check(&self, cap: Capability, resource: &str) -> HostResult<()> {
        self.policy()
            .check(cap, resource)
            .map_err(|d| d.into_err_info())
    }

    /// Resolves a global name.
    fn resolve_global(&self, name: &str) -> HostResult<Option<Variant>> {
        self.lookup_global(name)
    }

    /// `CreateObject`: requires [`Capability::ComCreate`] for the ProgID, and
    /// additionally [`Capability::Network`] for a remote `server`.
    fn create_object(&self, progid: &str, server: Option<&str>) -> HostResult<Variant> {
        self.check(Capability::ComCreate, progid)?;
        if let Some(s) = server.filter(|s| !s.is_empty()) {
            self.check(Capability::Network, s)?;
        }
        self.create_object_hook(progid, server)
    }

    /// `GetObject`: a file path requires [`Capability::FileRead`], a class
    /// name requires [`Capability::ComCreate`]. With neither it is error 5.
    fn get_object(&self, path: Option<&str>, class: Option<&str>) -> HostResult<Variant> {
        let path = path.filter(|p| !p.is_empty());
        let class = class.filter(|c| !c.is_empty());
        if path.is_none() && class.is_none() {
            return Err(ErrInfo::standard(5));
        }
        if let Some(p) = path {
            self.check(Capability::FileRead, p)?;
        }
        if let Some(c) = class {
            self.check(Capability::ComCreate, c)?;
        }
        self.get_object_hook(path, class)
    }

    /// Calls a `Declare`d procedure: requires [`Capability::NativeDeclare`]
    /// for the resource `library!procedure`.
    fn call_declare(&self, req: &mut DeclareRequest) -> HostResult<Variant> {
        self.check(
            Capability::NativeDeclare,
            &format!("{}!{}", req.library, req.procedure),
        )?;
        self.declare_hook(req)
    }

    /// `Date = value` / `Time = value`: requires nothing from the policy
    /// (changing the clock is not one of its capabilities) but is refused
    /// by the default hook.
    fn set_clock(&self, part: ClockPart, value: Date) -> HostResult<()> {
        self.set_clock_hook(part, value)
    }

    /// `SendKeys`: requires [`Capability::UiAutomation`].
    fn send_keys(&self, keys: &str, wait: bool) -> HostResult<()> {
        self.check(Capability::UiAutomation, "SendKeys")?;
        self.send_keys_hook(keys, wait)
    }

    /// `AppActivate`: requires [`Capability::UiAutomation`] for the resource
    /// `AppActivate:title` (a grant naming the action `AppActivate` covers
    /// every title).
    fn app_activate(&self, title: &str, wait: bool) -> HostResult<()> {
        self.check(Capability::UiAutomation, &format!("AppActivate:{title}"))?;
        self.app_activate_hook(title, wait)
    }

    /// `DoEvents`: an idle safe point. Charges one step against the budget,
    /// runs the idle hook and returns 0 (the number of open forms is not
    /// modelled).
    fn do_events(&self) -> HostResult<i32> {
        self.budget().charge_step()?;
        self.idle()?;
        Ok(0)
    }
}
