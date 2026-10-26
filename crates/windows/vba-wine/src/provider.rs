//! `WineProvider`: the opt-in `vba-winapi` provider backed by a Wine session.
//!
//! Gating (nothing is added to `vba-host`; the existing capability is used):
//!
//! * construction requires the policy to grant [`Capability::NativeDeclare`]
//!   for at least some resources, and the embedder must hand over an already
//!   running [`Session`] (the provider never starts a process);
//! * every call re-checks `policy.check(NativeDeclare, "library!symbol")`, so a
//!   prefix-scoped grant such as `"kernel32!"` limits what can be reached;
//! * only the exports the embedder declares ([`WineExport`]) are reachable,
//!   with a fixed signature; arbitrary `Declare`s are never forwarded blindly.
//!
//! `Provider::call` receives `NativeArg`s, where pointers are addresses of the
//! emulated memory. Those mean nothing to a real process, so exports used
//! through `Provider::call` must be scalar-only; buffer-taking exports go
//! through [`WineProvider::call_with_buffers`], which copies bytes explicitly.

use crate::error::WineError;
use crate::protocol::{ProcRef, RetKind, WireArg, WireConv, WireRet};
use crate::session::{CallReply, Session};
use std::collections::HashMap;
use std::sync::Mutex;
use vba_ffi::{Handle, NativeArg, NativeReturn, ProviderId};
use vba_host::{Capability, Policy};
use vba_profile::Bitness;
use vba_winapi::{CallContext, Provider, WinApiError};

/// A parameter of a registered export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// Integer by value (`Long`, `LongPtr` handle ids, ...).
    Int,
    /// `Single` by value.
    F32,
    /// `Double` by value.
    F64,
    /// Read-only buffer (copied).
    BufIn,
    /// Buffer copied in and back.
    BufInOut,
    /// Zero-filled buffer copied back.
    BufOut,
}

/// An export the embedder allows the bridge to forward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WineExport {
    /// Lower-case library name without extension (as `vba-winapi` normalizes).
    pub library: &'static str,
    /// Exact export name.
    pub symbol: &'static str,
    /// Calling convention.
    pub conv: WireConv,
    /// Parameters in order.
    pub params: Vec<ParamKind>,
    /// Return kind.
    pub ret: RetKind,
}

struct State {
    session: Session,
    libs: HashMap<&'static str, Handle>,
    procs: HashMap<(&'static str, &'static str), Handle>,
}

/// The provider. Calls are serialized by an internal mutex (one request in
/// flight per session).
pub struct WineProvider {
    id: ProviderId,
    policy: Policy,
    exports: Vec<WineExport>,
    state: Mutex<State>,
}

impl WineProvider {
    /// Wraps a running session.
    ///
    /// Fails with [`WineError::PermissionDenied`] unless `policy` grants
    /// `NativeDeclare` (the Wine bridge is native code execution), and with
    /// `BadRequest` for a malformed export table.
    pub fn new(
        policy: Policy,
        session: Session,
        exports: Vec<WineExport>,
    ) -> Result<Self, WineError> {
        if !policy.is_granted(Capability::NativeDeclare) {
            return Err(WineError::PermissionDenied(
                "the Wine bridge requires the NativeDeclare capability".into(),
            ));
        }
        for (i, e) in exports.iter().enumerate() {
            if e.library != e.library.to_ascii_lowercase() || e.library.ends_with(".dll") {
                return Err(WineError::BadRequest(format!(
                    "export {i}: library must be lower case without `.dll`"
                )));
            }
            if exports[..i]
                .iter()
                .any(|o| o.library == e.library && o.symbol == e.symbol)
            {
                return Err(WineError::BadRequest(format!(
                    "duplicate export {}!{}",
                    e.library, e.symbol
                )));
            }
        }
        Ok(WineProvider {
            id: session.provider(),
            policy,
            exports,
            state: Mutex::new(State {
                session,
                libs: HashMap::new(),
                procs: HashMap::new(),
            }),
        })
    }

    fn export(&self, library: &str, symbol: &str) -> Option<&WineExport> {
        self.exports
            .iter()
            .find(|e| e.library == library && e.symbol == symbol)
    }

    /// Calls an export with explicit buffers. `args` must match the export's
    /// parameter kinds one to one.
    pub fn call_with_buffers(
        &self,
        library: &str,
        symbol: &str,
        args: &[WireArg],
    ) -> Result<CallReply, WineError> {
        let exp = self.export(library, symbol).ok_or_else(|| {
            WineError::Unsupported(format!(
                "{library}!{symbol} is not a registered Wine export"
            ))
        })?;
        self.policy
            .check(Capability::NativeDeclare, &format!("{library}!{symbol}"))
            .map_err(|d| WineError::PermissionDenied(d.to_string()))?;
        if args.len() != exp.params.len() {
            return Err(WineError::BadRequest(format!(
                "{library}!{symbol} takes {} arguments, got {}",
                exp.params.len(),
                args.len()
            )));
        }
        for (i, (a, p)) in args.iter().zip(&exp.params).enumerate() {
            let ok = matches!(
                (a, p),
                (WireArg::Int(_), ParamKind::Int)
                    | (WireArg::F32(_), ParamKind::F32)
                    | (WireArg::F64(_), ParamKind::F64)
                    | (WireArg::BufIn(_), ParamKind::BufIn)
                    | (WireArg::BufInOut(_), ParamKind::BufInOut)
                    | (WireArg::BufOut(_), ParamKind::BufOut)
            );
            if !ok {
                return Err(WineError::BadRequest(format!(
                    "{library}!{symbol}: argument {i} does not match {p:?}"
                )));
            }
        }
        let mut st = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let st = &mut *st;
        let lib = match st.libs.get(exp.library) {
            Some(h) => *h,
            None => {
                let h = st.session.load_library(exp.library)?;
                st.libs.insert(exp.library, h);
                h
            }
        };
        let proc = match st.procs.get(&(exp.library, exp.symbol)) {
            Some(h) => *h,
            None => {
                let h = st
                    .session
                    .get_proc(&lib, ProcRef::Name(exp.symbol.to_string()))?;
                st.procs.insert((exp.library, exp.symbol), h);
                h
            }
        };
        st.session.call(&proc, exp.conv, exp.ret, args)
    }

    fn helper_bitness(&self) -> u8 {
        self.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .session
            .info()
            .bitness
    }
}

/// Wraps a bridge failure for the `Provider` interface, which has no richer
/// error type. The text always starts with `wine bridge <Kind>` so a failure
/// is never mistaken for an ordinary unimplemented symbol.
fn to_winapi(library: &str, symbol: &str, e: &WineError) -> WinApiError {
    WinApiError::Unsupported {
        symbol: format!("{library}!{symbol}"),
        reason: format!("wine bridge {}: {e}", e.kind()),
    }
}

impl Provider for WineProvider {
    fn id(&self) -> ProviderId {
        self.id
    }

    fn exports(&self) -> Vec<(&'static str, &'static str)> {
        self.exports.iter().map(|e| (e.library, e.symbol)).collect()
    }

    fn call(
        &self,
        ctx: &mut CallContext,
        library: &str,
        symbol: &str,
        args: &[NativeArg],
    ) -> Result<NativeReturn, WinApiError> {
        let fail = |e: WineError| to_winapi(library, symbol, &e);
        let exp = self.export(library, symbol).ok_or_else(|| {
            fail(WineError::Unsupported(
                "not a registered Wine export".into(),
            ))
        })?;
        if exp.params.iter().any(|p| {
            matches!(
                p,
                ParamKind::BufIn | ParamKind::BufInOut | ParamKind::BufOut
            )
        }) {
            return Err(fail(WineError::Unsupported(
                "buffer parameters cannot be forwarded from emulated-memory pointers; \
                 use call_with_buffers"
                    .into(),
            )));
        }
        let want = match self.helper_bitness() {
            32 => Bitness::B32,
            _ => Bitness::B64,
        };
        if ctx.target.bitness != want {
            return Err(fail(WineError::Unsupported(format!(
                "declared target is {:?} but the Wine helper is {want:?}",
                ctx.target.bitness
            ))));
        }
        if args.len() != exp.params.len() {
            return Err(WinApiError::BadArguments {
                symbol: "wine export",
                index: args.len(),
                reason: "wrong number of arguments",
            });
        }
        let mut wire = Vec::with_capacity(args.len());
        for (i, (a, p)) in args.iter().zip(&exp.params).enumerate() {
            wire.push(match (a, p) {
                (NativeArg::Int(v), ParamKind::Int) => WireArg::Int(*v),
                (NativeArg::F32(v), ParamKind::F32) => WireArg::F32(*v),
                (NativeArg::F64(v), ParamKind::F64) => WireArg::F64(*v),
                _ => {
                    return Err(WinApiError::BadArguments {
                        symbol: "wine export",
                        index: i,
                        reason: "argument kind does not match the registered export",
                    });
                }
            });
        }
        let reply = self
            .call_with_buffers(library, symbol, &wire)
            .map_err(fail)?;
        ctx.set_last_error(reply.last_error);
        Ok(match reply.ret {
            WireRet::Void => NativeReturn::Void,
            WireRet::Int(v) => NativeReturn::Int(v),
            WireRet::Float(v) => NativeReturn::Float(v),
        })
    }
}
