//! Capabilities and the deny-by-default permission policy.

use std::collections::BTreeMap;
use std::fmt;
use vba_automation::ErrInfo;

/// Something a VBA program may ask the host for beyond pure computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Capability {
    /// Reading files (`Open ... For Input`, `Dir`, `FileLen`, `GetObject(path)`).
    FileRead,
    /// Creating, writing, deleting or renaming files and directories.
    FileWrite,
    /// Starting processes (`Shell`).
    ProcessSpawn,
    /// Network access.
    Network,
    /// Creating COM/Automation objects by ProgID (`CreateObject`, `GetObject`).
    ComCreate,
    /// Calling native libraries through `Declare`.
    NativeDeclare,
    /// Driving other applications' UI (`SendKeys`, `AppActivate`).
    UiAutomation,
}

impl Capability {
    /// Every capability.
    pub const ALL: [Capability; 7] = [
        Capability::FileRead,
        Capability::FileWrite,
        Capability::ProcessSpawn,
        Capability::Network,
        Capability::ComCreate,
        Capability::NativeDeclare,
        Capability::UiAutomation,
    ];

    /// A short lower-case name for messages.
    pub const fn name(self) -> &'static str {
        match self {
            Capability::FileRead => "file read",
            Capability::FileWrite => "file write",
            Capability::ProcessSpawn => "process spawn",
            Capability::Network => "network",
            Capability::ComCreate => "COM object creation",
            Capability::NativeDeclare => "native Declare",
            Capability::UiAutomation => "UI automation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Grant {
    Any,
    Prefixes(Vec<String>),
}

/// A request that the policy refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionDenied {
    /// The capability requested.
    pub capability: Capability,
    /// The resource (path, ProgID, `library!procedure`, host:port, ...).
    pub resource: String,
}

impl PermissionDenied {
    /// VBA error 70, "Permission denied", with the detail in `Err.Source`.
    pub fn into_err_info(self) -> ErrInfo {
        ErrInfo::standard(70).with_source(format!("vba-host: {self}"))
    }
}

impl fmt::Display for PermissionDenied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} denied for '{}'",
            self.capability.name(),
            self.resource
        )
    }
}

impl std::error::Error for PermissionDenied {}

impl From<PermissionDenied> for ErrInfo {
    fn from(p: PermissionDenied) -> Self {
        p.into_err_info()
    }
}

/// The set of granted capabilities. The default grants nothing.
///
/// A grant is either unrestricted or limited to *scopes*, and what a scope
/// means depends on the capability (the table is in the `scope` module and
/// in `docs/architecture/security-model.md`):
///
/// * `FileRead`, `FileWrite`: a path; it covers itself and everything below
///   it, matched by whole path segments (`/data` covers `/data/x`, not
///   `/data-secret`). `/` and `\` are the same. Paths containing a `..`
///   segment never match. A POSIX path is compared exactly, case included.
///   A drive path (`c:/work`) and a network path (`//server/share`) are
///   compared the way Windows does: case folded, trailing dots and spaces
///   dropped, and a resource naming a stream (`a:b`), a device (`NUL`,
///   `COM1`), a short name (`PROGRA~1`) or a segment of only dots and spaces
///   never matches. The `//?/` and `//./` namespaces never match.
/// * `ProcessSpawn`: a program; it covers that program (`notepad` also covers
///   `notepad.exe`, never `notepad-x.exe`) and, as a directory, the programs
///   below it. Arguments are not part of the match.
/// * `ComCreate`: a ProgID, matched exactly; `Scripting.*` or `Scripting.`
///   covers every ProgID below the namespace `Scripting`, by whole dot
///   segments; `*` covers all.
/// * `NativeDeclare`: `library!procedure`, matched exactly, or `library!*`
///   for every procedure of a library. Library names are compared the way
///   the Win32 provider resolves them (directory and `.dll` dropped).
/// * `UiAutomation`: an action, `SendKeys` or `AppActivate`; `AppActivate:title`
///   covers one window title only.
/// * `Network`: a host, a `host:port`, or `*.domain`.
///
/// Matching of everything except file paths ignores case, and any scope that
/// is not one of these forms matches nothing. No path canonicalisation is done: embedders that scope
/// file access must pass normalised paths.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Policy {
    grants: BTreeMap<Capability, Grant>,
}

impl Policy {
    /// Denies everything.
    pub fn deny_all() -> Self {
        Self::default()
    }

    /// Grants every capability without restriction. For fully trusted
    /// embeddings only.
    pub fn allow_all() -> Self {
        let mut p = Self::default();
        for c in Capability::ALL {
            p.grants.insert(c, Grant::Any);
        }
        p
    }

    /// Grants a capability for all resources.
    pub fn grant(mut self, cap: Capability) -> Self {
        self.grants.insert(cap, Grant::Any);
        self
    }

    /// Grants a capability for the resources named by `scopes` (the form of a
    /// scope depends on the capability, see [`Policy`]). Calling it again for
    /// the same capability adds scopes; it never narrows an unrestricted
    /// grant. The name says "prefixes" for compatibility; only paths and
    /// namespaces behave like prefixes.
    pub fn grant_prefixes<I, S>(mut self, cap: Capability, prefixes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let new: Vec<String> = prefixes.into_iter().map(Into::into).collect();
        match self.grants.get_mut(&cap) {
            Some(Grant::Any) => {}
            Some(Grant::Prefixes(p)) => p.extend(new),
            None => {
                self.grants.insert(cap, Grant::Prefixes(new));
            }
        }
        self
    }

    /// Removes a grant.
    pub fn revoke(mut self, cap: Capability) -> Self {
        self.grants.remove(&cap);
        self
    }

    /// `true` if the capability is granted for at least some resources.
    pub fn is_granted(&self, cap: Capability) -> bool {
        self.grants.contains_key(&cap)
    }

    /// Checks a request. Denied requests become error 70 through
    /// [`PermissionDenied::into_err_info`].
    pub fn check(&self, cap: Capability, resource: &str) -> Result<(), PermissionDenied> {
        let ok = match self.grants.get(&cap) {
            None => false,
            Some(Grant::Any) => true,
            Some(Grant::Prefixes(ps)) => ps
                .iter()
                .any(|scope| crate::scope::scope_matches(cap, scope, resource)),
        };
        if ok {
            Ok(())
        } else {
            Err(PermissionDenied {
                capability: cap,
                resource: resource.to_string(),
            })
        }
    }
}
