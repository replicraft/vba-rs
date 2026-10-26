//! Portable native shims: Win32 functions whose behavior is reproducible and
//! testable identically on every host OS.

mod callbacks;
mod convert;
mod memory;
mod string;
mod system;
mod time;

use crate::callback::{CallbackError, CallbackHost, CallbackSig};
use crate::{CallContext, Provider, WinApiError};
use std::cell::RefCell;
use vba_ffi::{NativeArg, NativeReturn, ProviderId};

/// Identity of the built-in shim provider.
pub const NATIVE_SHIMS: ProviderId = ProviderId("native-shim");

pub(crate) type ShimFn = fn(&mut CallContext, &Args) -> Result<NativeReturn, WinApiError>;

pub(crate) struct Shim {
    pub dll: &'static str,
    pub symbol: &'static str,
    pub arity: usize,
    pub run: ShimFn,
}

/// Validated view of a call's arguments.
pub(crate) struct Args<'a, 'c> {
    symbol: &'static str,
    args: &'a [NativeArg],
    callbacks: RefCell<Option<&'c mut dyn CallbackHost>>,
}

impl Args<'_, '_> {
    /// Checks that `token` is a procedure pointer fit for `sig`.
    pub(crate) fn check_callback(
        &self,
        index: usize,
        token: i64,
        sig: &CallbackSig,
    ) -> Result<(), WinApiError> {
        let mut slot = self.callbacks.borrow_mut();
        let host = slot.as_mut().ok_or_else(|| self.no_callbacks())?;
        host.validate(token, sig).map_err(|e| match e {
            CallbackError::Rejected(reason) => WinApiError::BadCallback {
                symbol: self.symbol,
                reason: format!("argument {index}: {reason}"),
            },
            CallbackError::Aborted => WinApiError::CallbackAborted {
                symbol: self.symbol,
            },
        })
    }

    /// Runs the VBA procedure behind `token`.
    pub(crate) fn callback(
        &self,
        ctx: &mut CallContext,
        token: i64,
        sig: &CallbackSig,
        args: &[i64],
    ) -> Result<i64, WinApiError> {
        let mut slot = self.callbacks.borrow_mut();
        let host = slot.as_mut().ok_or_else(|| self.no_callbacks())?;
        host.invoke(ctx, token, sig, args).map_err(|e| match e {
            CallbackError::Rejected(reason) => WinApiError::BadCallback {
                symbol: self.symbol,
                reason,
            },
            CallbackError::Aborted => WinApiError::CallbackAborted {
                symbol: self.symbol,
            },
        })
    }

    fn no_callbacks(&self) -> WinApiError {
        WinApiError::Unsupported {
            symbol: self.symbol.to_string(),
            reason: "this call needs the interpreter to run a procedure pointer, and no callback host was provided".into(),
        }
    }

    pub(crate) fn bad(&self, index: usize, reason: &'static str) -> WinApiError {
        WinApiError::BadArguments {
            symbol: self.symbol,
            index,
            reason,
        }
    }

    pub(crate) fn int(&self, i: usize) -> Result<i64, WinApiError> {
        match self.args[i] {
            NativeArg::Int(v) => Ok(v),
            _ => Err(self.bad(i, "expected an integer or pointer argument")),
        }
    }

    /// A `DWORD`/`Long` taken modulo 2^32 (so -1 is 0xFFFFFFFF).
    pub(crate) fn u32(&self, i: usize) -> Result<u32, WinApiError> {
        Ok(self.int(i)? as u32)
    }

    /// A signed 32-bit `int`.
    pub(crate) fn i32(&self, i: usize) -> Result<i32, WinApiError> {
        Ok(self.int(i)? as i32)
    }

    /// A pointer; negative values are 32-bit sign-extended addresses.
    pub(crate) fn ptr(&self, i: usize) -> Result<u64, WinApiError> {
        let v = self.int(i)?;
        Ok(if v < 0 { u64::from(v as u32) } else { v as u64 })
    }

    /// A `SIZE_T`/length argument; negative is rejected.
    pub(crate) fn len(&self, i: usize) -> Result<usize, WinApiError> {
        let v = self.int(i)?;
        usize::try_from(v).map_err(|_| self.bad(i, "negative length"))
    }
}

use windows_sys::Win32::Foundation::ERROR_INVALID_PARAMETER;

pub(crate) fn invalid_parameter(ctx: &mut CallContext) -> NativeReturn {
    ctx.set_last_error(ERROR_INVALID_PARAMETER);
    NativeReturn::Int(0)
}

fn table() -> Vec<Shim> {
    let mut t = Vec::new();
    t.extend(time::SHIMS);
    t.extend(string::SHIMS);
    t.extend(convert::SHIMS);
    t.extend(system::SHIMS);
    t.extend(memory::SHIMS);
    t.extend(callbacks::SHIMS);
    t
}

/// The built-in portable shim provider.
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeShims;

impl Provider for NativeShims {
    fn id(&self) -> ProviderId {
        NATIVE_SHIMS
    }

    fn exports(&self) -> Vec<(&'static str, &'static str)> {
        table().iter().map(|s| (s.dll, s.symbol)).collect()
    }

    fn call(
        &self,
        ctx: &mut CallContext,
        library: &str,
        symbol: &str,
        args: &[NativeArg],
    ) -> Result<NativeReturn, WinApiError> {
        self.call_with_callbacks(ctx, library, symbol, args, None)
    }

    fn call_with_callbacks(
        &self,
        ctx: &mut CallContext,
        library: &str,
        symbol: &str,
        args: &[NativeArg],
        callbacks: Option<&mut dyn CallbackHost>,
    ) -> Result<NativeReturn, WinApiError> {
        let shim = table()
            .into_iter()
            .find(|s| s.dll == library && s.symbol == symbol)
            .ok_or_else(|| WinApiError::Unsupported {
                symbol: format!("{library}!{symbol}"),
                reason: "not exported by the native-shim provider".into(),
            })?;
        if args.len() != shim.arity {
            return Err(WinApiError::BadArguments {
                symbol: shim.symbol,
                index: args.len(),
                reason: "wrong number of arguments",
            });
        }
        (shim.run)(
            ctx,
            &Args {
                symbol: shim.symbol,
                args,
                callbacks: RefCell::new(callbacks),
            },
        )
    }
}
