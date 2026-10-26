//! Calls made by the embedder: argument preparation (`ByRef` slots, named
//! and omitted arguments) and write-back.

use vba_sema::ProcId;
use vba_value::Variant;

use crate::calls::BoundArg;
use crate::error::{RtErr, RuntimeError, VResult, VmError};
use crate::state::Loc;
use crate::vm::{Arg, Vm};

/// Where the final value of a [`Arg::ByRef`] argument is read from.
pub(crate) enum ByRefOut {
    /// A temporary variable the callee was bound to.
    Slot(usize),
    /// The argument was not bound by reference (`ByVal` parameter or
    /// `ParamArray`); the value to report.
    Orig(Variant),
}

impl Vm {
    /// Binds the embedder's arguments: values, omitted arguments, and
    /// `ByRef` variables, which live in temporary variable slots above the
    /// program's module variables until the call has finished.
    pub(crate) fn prepare_args(
        &mut self,
        pid: ProcId,
        args: Vec<Arg>,
    ) -> VResult<(Vec<BoundArg>, Vec<ByRefOut>)> {
        let ir = self.ir.clone();
        let p = ir.program.proc(pid);
        let has_pa = p.params.last().is_some_and(|x| x.param_array);
        let fixed = p.params.len() - usize::from(has_pa);
        let temp_base = self.globals.len();
        let mut bound = Vec::with_capacity(args.len());
        let mut outs = Vec::new();
        for (i, a) in args.into_iter().enumerate() {
            let step: VResult<()> = (|| {
                match a {
                    Arg::Value(v) => bound.push(BoundArg::Val(v)),
                    Arg::Missing => bound.push(BoundArg::Missing),
                    Arg::ByRef(v) => match p.params.get(i).filter(|_| i < fixed) {
                        Some(param) if !param.by_val => {
                            let v = self.coerce_arg(v, &param.ty)?;
                            let v = self.adopt(v)?;
                            let idx = self.globals.len();
                            self.globals.push(v);
                            bound.push(BoundArg::Ref(Loc::Global(idx as u32)));
                            outs.push(ByRefOut::Slot(idx));
                        }
                        _ => {
                            outs.push(ByRefOut::Orig(v.clone()));
                            bound.push(BoundArg::Val(v));
                        }
                    },
                    #[allow(unreachable_patterns)]
                    _ => {}
                }
                Ok(())
            })();
            if let Err(f) = step {
                self.collect_byref(temp_base, outs, false);
                return Err(f);
            }
        }
        Ok((bound, outs))
    }

    /// Takes the final values out of the temporary slots and removes them.
    /// With `keep` false the values are released instead of returned.
    pub(crate) fn collect_byref(
        &mut self,
        temp_base: usize,
        outs: Vec<ByRefOut>,
        keep: bool,
    ) -> Vec<Variant> {
        let mut values = Vec::with_capacity(outs.len());
        for o in outs {
            values.push(match o {
                ByRefOut::Slot(i) if i < self.globals.len() => {
                    std::mem::replace(&mut self.globals[i], Variant::Empty)
                }
                ByRefOut::Slot(_) => Variant::Empty,
                ByRefOut::Orig(v) => v,
            });
        }
        self.globals.truncate(temp_base.min(self.globals.len()));
        if keep {
            values
        } else {
            for v in values {
                let _ = self.release_value(v);
            }
            Vec::new()
        }
    }

    /// Arranges positional and named arguments of an embedder's call into
    /// the parameter order of `proc_`, filling gaps with [`Arg::Missing`].
    ///
    /// Errors, as in VBA: an unknown or `ParamArray` parameter name is
    /// 448, a parameter given twice is 446.
    pub fn arrange_call_args(
        &self,
        proc_: ProcId,
        positional: Vec<Arg>,
        named: Vec<(String, Arg)>,
    ) -> Result<Vec<Arg>, VmError> {
        let p = self.ir.program.proc(proc_);
        let fixed = p.params.len() - usize::from(p.params.last().is_some_and(|x| x.param_array));
        let mut out = positional;
        for (n, a) in named {
            let Some(i) = p
                .params
                .iter()
                .take(fixed)
                .position(|q| q.name.to_string().eq_ignore_ascii_case(&n))
            else {
                return Err(plain_error(448));
            };
            while out.len() <= i {
                out.push(Arg::Missing);
            }
            if !matches!(out[i], Arg::Missing) {
                return Err(plain_error(446));
            }
            out[i] = a;
        }
        Ok(out)
    }
}

/// An unhandled VBA error raised by the embedding layer itself (no frame is
/// running).
pub(crate) fn plain_error(number: i32) -> VmError {
    VmError::Runtime(Box::new(RuntimeError::new(RtErr::standard(number))))
}
