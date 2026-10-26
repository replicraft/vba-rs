//! The provider trait and the `Declare` resolution registry.

use crate::callback::CallbackHost;
use crate::catalog::{self, Status};
use crate::inventory;
use crate::{CallContext, WinApiError};
use std::collections::HashMap;
use vba_ffi::{EntryCandidate, NativeArg, NativeReturn, ProviderId, ValidatedDeclare};
use vba_profile::OperatingSystem;

/// Describes the first candidate the Win32 inventory knows, as a sentence
/// fragment for an `Unsupported` reason.
fn known_signature(lib: &str, candidates: &[EntryCandidate]) -> Option<String> {
    candidates.iter().find_map(|c| {
        let EntryCandidate::Symbol(s) = c else {
            return None;
        };
        if let Some(e) = inventory::lookup(lib, s) {
            return Some(format!(
                "known Win32 export per Microsoft's metadata, signature {e}"
            ));
        }
        let elsewhere = inventory::lookup_any_dll(s);
        let e = elsewhere.first()?;
        Some(format!(
            "`{s}` is a known Win32 export of `{}` (not `{lib}`) per Microsoft's metadata, signature {e}",
            e.dll
        ))
    })
}

/// A source of Win32 behavior.
pub trait Provider {
    /// Stable identity, used for handle affinity.
    fn id(&self) -> ProviderId;
    /// Every `(library, symbol)` this provider implements; library names are
    /// lower-case without extension.
    fn exports(&self) -> Vec<(&'static str, &'static str)>;
    /// Calls `symbol` of `library`. Only invoked for registered exports.
    fn call(
        &self,
        ctx: &mut CallContext,
        library: &str,
        symbol: &str,
        args: &[NativeArg],
    ) -> Result<NativeReturn, WinApiError>;

    /// Like [`Provider::call`], for a call that may run VBA procedures through
    /// `callbacks` (functions taking an `AddressOf` token). The default
    /// ignores the callback host.
    fn call_with_callbacks(
        &self,
        ctx: &mut CallContext,
        library: &str,
        symbol: &str,
        args: &[NativeArg],
        callbacks: Option<&mut dyn CallbackHost>,
    ) -> Result<NativeReturn, WinApiError> {
        let _ = callbacks;
        self.call(ctx, library, symbol, args)
    }
}

/// The outcome of resolving a `Declare`: which provider, which symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    provider: usize,
    library: String,
    symbol: String,
}

impl Resolved {
    /// The canonical library name.
    pub fn library(&self) -> &str {
        &self.library
    }
    /// The exported symbol that matched.
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}

/// Normalizes a `Lib` string: drops any directory, lower-cases and strips a
/// trailing `.dll`.
pub fn normalize_library(lib: &str) -> String {
    let file = lib
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(lib)
        .to_ascii_lowercase();
    file.strip_suffix(".dll")
        .map(str::to_string)
        .unwrap_or(file)
}

/// Maps `Declare` (library, alias) pairs to providers.
#[derive(Default)]
pub struct Registry {
    providers: Vec<Box<dyn Provider>>,
    index: HashMap<(String, String), usize>,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry containing the portable native shims.
    pub fn with_native_shims() -> Self {
        let mut r = Self::new();
        r.register(Box::new(crate::shims::NativeShims))
            .expect("built-in shims have unique exports");
        r
    }

    /// Registers a provider; no export may already be taken.
    pub fn register(&mut self, provider: Box<dyn Provider>) -> Result<(), WinApiError> {
        let idx = self.providers.len();
        let mut staged = Vec::new();
        for (lib, sym) in provider.exports() {
            let key = (lib.to_string(), sym.to_string());
            if self.index.contains_key(&key) || staged.contains(&key) {
                return Err(WinApiError::DuplicateExport(format!("{lib}!{sym}")));
            }
            staged.push(key);
        }
        for k in staged {
            self.index.insert(k, idx);
        }
        self.providers.push(provider);
        Ok(())
    }

    /// All registered `(library, symbol)` pairs, sorted.
    pub fn exports(&self) -> Vec<(String, String)> {
        let mut v: Vec<_> = self.index.keys().cloned().collect();
        v.sort();
        v
    }

    /// Resolves a validated `Declare`.
    pub fn resolve_declare(&self, decl: &ValidatedDeclare) -> Result<Resolved, WinApiError> {
        self.resolve(
            decl.target().os,
            &decl.spec().library,
            &decl.entry_candidates(),
        )
    }

    /// Resolves a library and ordered entry candidates.
    ///
    /// Failures are always `Unsupported` naming the symbol: vba-rs cannot know
    /// whether the real library would have exported it, so it never claims
    /// "DLL not found" or "entry point not found" on the library's behalf.
    pub fn resolve(
        &self,
        os: OperatingSystem,
        library: &str,
        candidates: &[EntryCandidate],
    ) -> Result<Resolved, WinApiError> {
        let lib = normalize_library(library);
        if os != OperatingSystem::Windows {
            return Err(WinApiError::unsupported(
                format!("{lib}!*"),
                "Win32 libraries are not available for a non-Windows target",
            ));
        }
        let mut first = None;
        for c in candidates {
            match c {
                EntryCandidate::Ordinal(n) => {
                    return Err(WinApiError::unsupported(
                        format!("{lib}!#{n}"),
                        "ordinal entry points are not supported",
                    ));
                }
                EntryCandidate::Symbol(s) => {
                    if let Some(&p) = self.index.get(&(lib.clone(), s.clone())) {
                        return Ok(Resolved {
                            provider: p,
                            library: lib,
                            symbol: s.clone(),
                        });
                    }
                    first.get_or_insert(s.clone());
                }
            }
        }
        let name = first.unwrap_or_default();
        let reason = candidates
            .iter()
            .find_map(|c| match c {
                EntryCandidate::Symbol(s) => catalog::lookup(&lib, s),
                EntryCandidate::Ordinal(_) => None,
            })
            .map(|row| match row.status {
                Status::NotImplemented => {
                    format!("cataloged ({:?}) but not implemented", row.strategy)
                }
                Status::Implemented => {
                    "implemented by a provider that is not registered".to_string()
                }
            })
            .unwrap_or_else(|| "no registered provider exports this symbol".to_string());
        let reason = match known_signature(&lib, candidates) {
            Some(known) => format!("{reason}; {known}"),
            None => reason,
        };
        Err(WinApiError::unsupported(format!("{lib}!{name}"), reason))
    }

    /// Performs a resolved call.
    pub fn call(
        &self,
        ctx: &mut CallContext,
        resolved: &Resolved,
        args: &[NativeArg],
    ) -> Result<NativeReturn, WinApiError> {
        self.providers[resolved.provider].call(ctx, &resolved.library, &resolved.symbol, args)
    }

    /// Performs a resolved call that may call back into VBA through
    /// `callbacks`.
    pub fn call_with_callbacks(
        &self,
        ctx: &mut CallContext,
        resolved: &Resolved,
        args: &[NativeArg],
        callbacks: Option<&mut dyn CallbackHost>,
    ) -> Result<NativeReturn, WinApiError> {
        self.providers[resolved.provider].call_with_callbacks(
            ctx,
            &resolved.library,
            &resolved.symbol,
            args,
            callbacks,
        )
    }

    /// The provider that serves `resolved`.
    pub fn provider_of(&self, resolved: &Resolved) -> ProviderId {
        self.providers[resolved.provider].id()
    }
}
