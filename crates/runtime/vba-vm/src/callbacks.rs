//! `AddressOf` and callbacks into VBA from `Declare` shims.
//!
//! `AddressOf Proc` evaluates to a **token**: an integer that names a slot of
//! the session's [`CallbackTable`](vba_ffi::CallbackTable). It is not an
//! address, nothing native can call it, and it stops being valid when the
//! session is reset (`Vm::reset`, `End`). Only a shim of `vba-winapi` can turn
//! a token into a call: while a `Declare` call runs, the shim is handed a
//! [`CallbackHost`] that validates the token and the procedure's signature and
//! then runs the procedure on this interpreter (a nested run, under the same
//! step, call-depth and native-stack budgets as any other call). The
//! capability policy is applied by the `Declare` that received the token (the
//! same `NativeDeclare` grant); timers, which outlive that call, are checked
//! again when they are delivered.
//!
//! What goes wrong inside a callback never reaches the shim as a value. A
//! run-time error that the procedure does not handle, a resource limit,
//! cancellation, `End` or `Stop` is kept in [`Vm`]'s hands, the shim is told
//! to stop (`CallbackError::Aborted`), and the `Declare` statement that made
//! the call then fails with exactly that fault: a trappable error stays
//! trappable, a limit stays untrappable.

use vba_ffi::TokenError;
use vba_host::Capability;
use vba_profile::Bitness;
use vba_sema::{ProcId, Type};
use vba_value::{Variant, VbaError, convert};
use vba_winapi::{
    CallContext, CallbackError, CallbackHost, CallbackSig, CbRet, CbSlot, Identity, ManualClock,
    dispatch_timers,
};

use crate::calls::BoundArg;
use crate::error::{ErrOrigin, Fault, RtErr, VResult};
use crate::vm::Vm;

/// The error number VBA raises when a DLL call leaves the stack unbalanced,
/// which is what a callback with the wrong parameter list does there.
const BAD_DLL_CALLING_CONVENTION: i32 = 49;

fn rejected(number: i32, msg: String) -> Fault {
    Fault::from(
        RtErr::new(number, format!("Declare callback: {msg}")).with_origin(ErrOrigin::Bridge),
    )
}

impl Vm {
    fn is_64bit(&self) -> bool {
        self.ir.program.profile.target.bitness == Bitness::B64
    }

    /// Evaluates `AddressOf p`: the session token of `p` as a `LongPtr`.
    pub(crate) fn address_of(&mut self, p: ProcId) -> VResult<Variant> {
        let token = self.declare.callbacks.mint(p).map_err(|e| match e {
            TokenError::Full => Fault::from(VbaError::new(7)),
            other => rejected(5, other.to_string()),
        })?;
        Ok(if self.is_64bit() {
            Variant::LongLong(token)
        } else {
            Variant::Long(token as i32)
        })
    }

    /// The procedure a token names, if it is a live token of this session
    /// whose procedure fits `sig`. The error is the VBA fault to raise.
    fn callback_target(&self, token: i64, sig: &CallbackSig) -> Result<ProcId, Fault> {
        let pid = self
            .declare
            .callbacks
            .resolve(token)
            .map_err(|e| rejected(5, format!("{e}")))?;
        self.check_signature(pid, sig)
            .map_err(|m| rejected(BAD_DLL_CALLING_CONVENTION, m))?;
        Ok(pid)
    }

    /// Arity, `ByVal` parameter types and result of `pid` against `sig`.
    fn check_signature(&self, pid: ProcId, sig: &CallbackSig) -> Result<(), String> {
        let p = self.ir.program.proc(pid);
        let name = &p.name;
        if p.params.len() != sig.params.len() {
            return Err(format!(
                "`{name}` takes {} parameter(s), but {} needs {}",
                p.params.len(),
                sig.name,
                sig.params.len()
            ));
        }
        let ptr_ty = if self.is_64bit() {
            Type::LongLong
        } else {
            Type::Long
        };
        for (i, (q, slot)) in p.params.iter().zip(sig.params).enumerate() {
            let (want, text) = match slot {
                CbSlot::Int32 => (Type::Long, "Long"),
                CbSlot::Ptr => (ptr_ty.clone(), "LongPtr"),
            };
            if !q.by_val || q.is_array || q.optional || q.param_array || q.ty != want {
                return Err(format!(
                    "parameter {} of `{name}` must be `ByVal {} As {text}` for {}",
                    i + 1,
                    q.name,
                    sig.name
                ));
            }
        }
        if sig.ret == CbRet::Int32 && !(p.has_result && p.ret == Type::Long) {
            return Err(format!(
                "`{name}` must be a Function returning Long for {}",
                sig.name
            ));
        }
        Ok(())
    }

    /// Runs the procedure behind a validated token with the given arguments.
    fn run_callback(&mut self, pid: ProcId, sig: &CallbackSig, args: &[i64]) -> VResult<i64> {
        let wide = self.is_64bit();
        let bound = sig
            .params
            .iter()
            .zip(args)
            .map(|(slot, a)| {
                BoundArg::Val(match slot {
                    CbSlot::Ptr if wide => Variant::LongLong(*a),
                    _ => Variant::Long(*a as i32),
                })
            })
            .collect();
        let v = self.call_nested(pid, None, bound)?;
        match sig.ret {
            CbRet::Void => Ok(0),
            CbRet::Int32 => {
                let v = self.value_of(v)?;
                Ok(convert::to_longlong(&v)?)
            }
        }
    }

    /// Runs `f` with the `Declare` context taken out of the VM and a callback
    /// host that can run VBA procedures. Returns what `f` returned and the
    /// fault, if any, that a callback ended with (to be raised by the caller
    /// in preference to whatever `f` returned).
    pub(crate) fn with_callback_host<R>(
        &mut self,
        f: impl FnOnce(&mut CallContext, &mut dyn CallbackHost) -> R,
    ) -> VResult<(R, Option<Fault>)> {
        let mut ctx = self
            .declare
            .ctx
            .take()
            .ok_or_else(|| Fault::Internal("no Declare context".into()))?;
        let mut host = VmCallbacks {
            vm: self,
            fault: None,
            spare: None,
        };
        let r = f(&mut ctx, &mut host);
        let fault = host.fault.take();
        self.declare.ctx = Some(ctx);
        Ok((r, fault))
    }

    /// Delivers due timers (the stand-in for a message pump). Called by
    /// `DoEvents`.
    pub(crate) fn pump_timers(&mut self) -> VResult<()> {
        match &self.declare.ctx {
            Some(c) if !c.timers.is_empty() => {}
            _ => return Ok(()),
        }
        // The timers were set by a granted Declare; policy is asked again
        // because it is the policy at delivery time that counts.
        self.host
            .check(Capability::NativeDeclare, "user32!SetTimer")
            .map_err(Fault::from)?;
        let (r, fault) = self.with_callback_host(|ctx, host| dispatch_timers(ctx, host))?;
        if let Some(f) = fault {
            return Err(f);
        }
        match r {
            Ok(_) => Ok(()),
            Err(CallbackError::Rejected(m)) => Err(rejected(5, m)),
            Err(CallbackError::Aborted) => Err(Fault::Internal(
                "a timer callback aborted without a fault".into(),
            )),
        }
    }
}

/// The [`CallbackHost`] a shim sees for the length of one `Declare` call.
struct VmCallbacks<'a> {
    vm: &'a mut Vm,
    /// What the first failing callback ended with.
    fault: Option<Fault>,
    /// A context to leave in the shim's hands while the real one is in the VM.
    spare: Option<CallContext>,
}

impl VmCallbacks<'_> {
    /// Records a refused token or signature as the fault of the `Declare`.
    fn fail(&mut self, f: Fault) -> CallbackError {
        let msg = match &f {
            Fault::Err(e) => e.description.clone(),
            other => format!("{other:?}"),
        };
        if self.fault.is_none() {
            self.fault = Some(f);
        }
        CallbackError::Rejected(msg)
    }
}

impl CallbackHost for VmCallbacks<'_> {
    fn validate(&mut self, token: i64, sig: &CallbackSig) -> Result<(), CallbackError> {
        match self.vm.callback_target(token, sig) {
            Ok(_) => Ok(()),
            Err(f) => Err(self.fail(f)),
        }
    }

    fn invoke(
        &mut self,
        ctx: &mut CallContext,
        token: i64,
        sig: &CallbackSig,
        args: &[i64],
    ) -> Result<i64, CallbackError> {
        if self.fault.is_some() {
            // An earlier callback already failed; nothing else may run.
            return Err(CallbackError::Aborted);
        }
        let pid = match self.vm.callback_target(token, sig) {
            Ok(p) => p,
            Err(f) => return Err(self.fail(f)),
        };
        // The procedure may itself use Declare: it must find the context in
        // the VM, so the real one goes there for the duration of the run and
        // the shim keeps a placeholder.
        let spare = match self.spare.take() {
            Some(s) => s,
            None => CallContext::new(
                ctx.target,
                Box::new(ManualClock::at(0)),
                Identity::new("user", "COMPUTER").expect("valid identity"),
            ),
        };
        let real = std::mem::replace(ctx, spare);
        self.vm.declare.ctx = Some(real);
        let r = self.vm.run_callback(pid, sig, args);
        match self.vm.declare.ctx.take() {
            Some(real) => self.spare = Some(std::mem::replace(ctx, real)),
            None => {
                self.fault = Some(Fault::Internal("the Declare context was lost".into()));
                return Err(CallbackError::Aborted);
            }
        }
        match r {
            Ok(v) => Ok(v),
            Err(f) => {
                // Whatever the procedure ended with, the shim only learns
                // that it must stop; the Declare statement raises the fault.
                if self.fault.is_none() {
                    self.fault = Some(f);
                }
                Err(CallbackError::Aborted)
            }
        }
    }
}
