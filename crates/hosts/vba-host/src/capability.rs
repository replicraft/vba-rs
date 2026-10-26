//! Capabilities and the deny-by-default permission policy.

use std::collections::BTreeMap;
use std::fmt;
use vba_automation::ErrInfo;

/// Something a VBA program may ask the host for beyond pure computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
/// A grant is either unrestricted or limited to resource prefixes
/// (case-insensitive, for example a directory path or a ProgID namespace
/// such as `"Scripting."`). For `FileRead` and `FileWrite` a prefix matches
/// whole path segments only: `/data` covers `/data` and `/data/x`, not
/// `/data-secret`. Other capabilities match the text prefix. Resources
/// containing a `..` path segment never match a scoped grant. No path
/// canonicalisation is done: embedders that scope file access must pass
/// normalised paths.
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

    /// Grants a capability for resources starting with one of `prefixes`.
    /// Calling it again for the same capability adds prefixes; it never
    /// narrows an unrestricted grant.
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
            Some(Grant::Prefixes(ps)) => {
                let r = resource.to_lowercase();
                let paths = matches!(cap, Capability::FileRead | Capability::FileWrite);
                !has_parent_segment(&r)
                    && ps.iter().any(|p| {
                        let p = p.to_lowercase();
                        if paths {
                            path_prefix_matches(&r.replace('\\', "/"), &p.replace('\\', "/"))
                        } else {
                            r.starts_with(&p)
                        }
                    })
            }
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

/// Path-segment prefix match on paths whose separators are both `/`: `/data` covers `/data` and `/data/x` but not
/// `/data-secret/x`. A prefix that ends in a separator covers everything below
/// it, as before.
fn path_prefix_matches(resource: &str, prefix: &str) -> bool {
    match resource.strip_prefix(prefix) {
        None => false,
        Some(rest) => rest.is_empty() || prefix.ends_with('/') || rest.starts_with('/'),
    }
}

fn has_parent_segment(r: &str) -> bool {
    r.split(['/', '\\']).any(|seg| seg == "..")
}
