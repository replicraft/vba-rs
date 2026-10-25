//! The [`Host`] trait.

use crate::capability::{Capability, Policy};
use crate::events::EventDispatch;
use crate::limits::{Budget, ResourceLimits};
use crate::metadata::HostMetadata;
use crate::ui::UiHooks;
use vba_automation::{Arg, ErrInfo, ObjectTable};
use vba_value::{VarType, Variant};

/// Result of a host operation; errors are the VBA `Err` contents.
pub type HostResult<T> = Result<T, ErrInfo>;

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
    /// Hook for `CreateObject(progid, server)`; only called when ComCreate is
    /// granted.
    fn create_object_hook(&self, progid: &str, server: Option<&str>) -> HostResult<Variant>;
    /// Hook for `GetObject(path, class)`; only called when permitted.
    fn get_object_hook(&self, path: Option<&str>, class: Option<&str>) -> HostResult<Variant>;
    /// Hook for a `Declare`d procedure; only called when NativeDeclare is
    /// granted for `library!procedure`.
    fn declare_hook(&self, req: &mut DeclareRequest) -> HostResult<Variant>;
    /// Hook run at every idle safe point (`DoEvents`); an error aborts the
    /// run (for example a cancellation request).
    fn idle(&self) -> HostResult<()> {
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

    /// `DoEvents`: an idle safe point. Charges one step against the budget,
    /// runs the idle hook and returns 0 (the number of open forms is not
    /// modelled).
    fn do_events(&self) -> HostResult<i32> {
        self.budget().charge_step()?;
        self.idle()?;
        Ok(0)
    }
}
