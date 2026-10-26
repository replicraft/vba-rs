//! `Declare` calls: validation (`vba-ffi`), permission check (`vba-host`),
//! provider resolution (`vba-winapi`) and argument marshaling.
//!
//! Nothing here loads a native library. A call that no provider implements
//! is [`Fault::Unsupported`], never a fabricated result.

use std::collections::HashMap;
use std::rc::Rc;

use vba_automation::Arg as AutoArg;
use vba_ffi::{
    ArgValue, CallConv, Charset, DeclareSpec, FfiError, FieldValue, ParamKind, ParamSpec, Passing,
    PreparedCall, UdtFieldType, UdtLayout, ValidatedDeclare,
};
use vba_host::{Capability, DeclareRequest};
use vba_ir::{CallArg, IrProgram};
use vba_profile::Target;
use vba_sema::{DeclareId, Type, UdtId};
use vba_value::{Currency, Date, UdtHandle, Variant, VbaArray, VbaError, VbaString, convert};
use vba_winapi::{CallContext, Identity, ManualClock, Registry, WinApiError};

use crate::error::{ErrOrigin, Fault, RtErr, VResult};
use crate::state::*;
use crate::vm::Vm;

/// Cached `Declare` machinery of a VM.
pub(crate) struct DeclareState {
    registry: Option<Registry>,
    ctx: Option<CallContext>,
    cache: HashMap<u32, Rc<ValidatedDeclare>>,
}

impl DeclareState {
    pub(crate) fn new() -> DeclareState {
        DeclareState {
            registry: None,
            ctx: None,
            cache: HashMap::new(),
        }
    }
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn bridge(msg: impl Into<String>) -> Fault {
    Fault::Unsupported(msg.into())
}

impl Vm {
    /// Replaces the `Declare` provider registry and call context (clock,
    /// identity, reported Windows version, emulated memory). The default is
    /// the portable native shims with a manual clock at zero.
    pub fn set_native_bridge(&mut self, registry: Registry, ctx: CallContext) {
        self.declare.registry = Some(registry);
        self.declare.ctx = Some(ctx);
    }

    fn ensure_bridge(&mut self, target: Target) {
        if self.declare.registry.is_none() {
            self.declare.registry = Some(Registry::with_native_shims());
        }
        if self.declare.ctx.is_none() {
            let identity = Identity::new("user", "COMPUTER").expect("valid identity");
            self.declare.ctx = Some(CallContext::new(
                target,
                Box::new(ManualClock::at(0)),
                identity,
            ));
        }
    }

    fn field_type(
        &self,
        ir: &IrProgram,
        t: &Type,
        bounds: &Option<Vec<(i32, i32)>>,
    ) -> VResult<UdtFieldType> {
        let base = match t {
            Type::Byte => UdtFieldType::Byte,
            Type::Boolean => UdtFieldType::Boolean,
            Type::Integer => UdtFieldType::Integer,
            Type::Long | Type::Enum(_) => UdtFieldType::Long,
            Type::LongLong => UdtFieldType::LongLong,
            Type::Single => UdtFieldType::Single,
            Type::Double => UdtFieldType::Double,
            Type::Currency => UdtFieldType::Currency,
            Type::Date => UdtFieldType::Date,
            Type::FixedString(n) => UdtFieldType::FixedString(*n as usize),
            Type::Udt(u) => UdtFieldType::Udt(self.udt_layout(ir, *u)?),
            Type::Array { elem, .. } => {
                let count = bounds
                    .as_ref()
                    .map(|b| {
                        b.iter()
                            .map(|(l, u)| (*u as i64 - *l as i64 + 1).max(0) as usize)
                            .product()
                    })
                    .ok_or_else(|| bridge("dynamic array member in a Declare user-defined type"))?;
                return Ok(UdtFieldType::Array {
                    elem: Box::new(self.field_type(ir, elem, &None)?),
                    count,
                });
            }
            other => {
                return Err(bridge(format!(
                    "member type {other} cannot be marshaled in a Declare user-defined type"
                )));
            }
        };
        Ok(base)
    }

    fn udt_layout(&self, ir: &IrProgram, u: UdtId) -> VResult<UdtLayout> {
        let info = &ir.program.udts[u.index()];
        let mut fields = Vec::new();
        for f in &info.fields {
            fields.push((
                f.name.to_string(),
                self.field_type(ir, &f.ty, &f.fixed_bounds)?,
            ));
        }
        UdtLayout::new(ir.program.profile.target, fields).map_err(|e| bridge(e.to_string()))
    }

    fn param_kind(&self, ir: &IrProgram, t: &Type) -> VResult<ParamKind> {
        Ok(match t {
            Type::Byte => ParamKind::Byte,
            Type::Boolean => ParamKind::Boolean,
            Type::Integer => ParamKind::Integer,
            Type::Long | Type::Enum(_) => ParamKind::Long,
            Type::LongLong => ParamKind::LongLong,
            Type::Single => ParamKind::Single,
            Type::Double => ParamKind::Double,
            Type::Currency => ParamKind::Currency,
            Type::Date => ParamKind::Date,
            Type::String | Type::FixedString(_) => ParamKind::String,
            Type::Variant => ParamKind::Variant,
            Type::Any => ParamKind::Any,
            Type::Udt(u) => ParamKind::Udt(self.udt_layout(ir, *u)?),
            Type::Array { elem, .. } => ParamKind::Array(Box::new(self.param_kind(ir, elem)?)),
            other => {
                return Err(bridge(format!(
                    "parameter type {other} is not supported in a Declare statement"
                )));
            }
        })
    }

    fn validated(&mut self, ir: &IrProgram, d: DeclareId) -> VResult<Rc<ValidatedDeclare>> {
        if let Some(v) = self.declare.cache.get(&d.0) {
            return Ok(v.clone());
        }
        let info = &ir.program.declares[d.index()];
        let mut params = Vec::new();
        for p in &info.params {
            let mut kind = self.param_kind(ir, &p.ty)?;
            if p.is_array && !matches!(kind, ParamKind::Array(_)) {
                kind = ParamKind::Array(Box::new(kind));
            }
            params.push(ParamSpec {
                name: p.name.to_string(),
                kind,
                passing: if p.by_val {
                    Passing::ByVal
                } else {
                    Passing::ByRef
                },
            });
        }
        let ret = if info.is_function {
            Some(self.param_kind(ir, &info.ret)?)
        } else {
            None
        };
        let spec = DeclareSpec {
            target: ir.program.profile.target,
            ptr_safe: info.ptr_safe,
            name: info.name.to_string(),
            library: lossy(&info.lib),
            alias: info.alias.as_deref().map(lossy),
            charset: Charset::Ansi,
            conv: CallConv::Stdcall,
            params,
            ret,
        };
        let v = Rc::new(spec.validate().map_err(|e| bridge(e.to_string()))?);
        self.declare.cache.insert(d.0, v.clone());
        Ok(v)
    }

    fn to_arg_value(&self, kind: &ParamKind, v: &Variant) -> VResult<ArgValue> {
        let int = |v: &Variant| -> VResult<ArgValue> {
            Ok(match v {
                Variant::Boolean(b) => ArgValue::Int(if *b { -1 } else { 0 }),
                Variant::Currency(c) => ArgValue::Int(c.scaled()),
                other => ArgValue::Int(convert::to_longlong(other)?),
            })
        };
        Ok(match kind {
            ParamKind::Byte
            | ParamKind::Boolean
            | ParamKind::Integer
            | ParamKind::Long
            | ParamKind::LongLong
            | ParamKind::LongPtr
            | ParamKind::Currency => int(v)?,
            ParamKind::Single | ParamKind::Double | ParamKind::Date => {
                ArgValue::Float(convert::to_double(v)?)
            }
            ParamKind::String => ArgValue::Str(convert::to_vba_string(v)?),
            ParamKind::Any => match v {
                Variant::String(s) => ArgValue::Str(s.clone()),
                Variant::Array(a) => ArgValue::Bytes(
                    a.iter()
                        .map(convert::to_byte)
                        .collect::<Result<Vec<u8>, _>>()?,
                ),
                other => int(other)?,
            },
            ParamKind::Udt(layout) => {
                let Variant::UserDefined(h) = v else {
                    return Err(VbaError::TYPE_MISMATCH.into());
                };
                let fv = self.udt_to_fields(*h)?;
                ArgValue::Bytes(
                    layout
                        .encode(&fv, vba_ffi::Codepage::Windows1252)
                        .map_err(|e| bridge(e.to_string()))?,
                )
            }
            ParamKind::Array(elem) => {
                let Variant::Array(a) = v else {
                    return Err(VbaError::TYPE_MISMATCH.into());
                };
                if **elem == ParamKind::Byte {
                    ArgValue::Bytes(
                        a.iter()
                            .map(convert::to_byte)
                            .collect::<Result<Vec<u8>, _>>()?,
                    )
                } else if let ParamKind::Udt(layout) = &**elem {
                    let mut bytes = Vec::new();
                    for e in a.iter() {
                        let Variant::UserDefined(h) = e else {
                            return Err(VbaError::TYPE_MISMATCH.into());
                        };
                        let fv = self.udt_to_fields(*h)?;
                        bytes.extend(
                            layout
                                .encode(&fv, vba_ffi::Codepage::Windows1252)
                                .map_err(|e| bridge(e.to_string()))?,
                        );
                    }
                    ArgValue::Bytes(bytes)
                } else {
                    ArgValue::Array(
                        a.iter()
                            .map(|e| self.to_arg_value(elem, e))
                            .collect::<VResult<Vec<_>>>()?,
                    )
                }
            }
            ParamKind::Variant => return Err(bridge("Variant arguments in Declare calls")),
        })
    }

    fn udt_to_fields(&self, h: UdtHandle) -> VResult<Vec<FieldValue>> {
        let u = self
            .udts
            .get(&h.0)
            .ok_or_else(|| Fault::Internal("dangling user-defined type".into()))?;
        u.fields.iter().map(|f| self.to_field_value(f)).collect()
    }

    fn to_field_value(&self, v: &Variant) -> VResult<FieldValue> {
        Ok(match v {
            Variant::Boolean(b) => FieldValue::Int(if *b { -1 } else { 0 }),
            Variant::Byte(_) | Variant::Integer(_) | Variant::Long(_) | Variant::LongLong(_) => {
                FieldValue::Int(convert::to_longlong(v)?)
            }
            Variant::Currency(c) => FieldValue::Int(c.scaled()),
            Variant::Single(_) | Variant::Double(_) | Variant::Date(_) => {
                FieldValue::Float(convert::to_double(v)?)
            }
            Variant::String(s) => FieldValue::Str(s.clone()),
            Variant::UserDefined(h) => FieldValue::Struct(self.udt_to_fields(*h)?),
            Variant::Array(a) => FieldValue::Array(
                a.iter()
                    .map(|e| self.to_field_value(e))
                    .collect::<VResult<Vec<_>>>()?,
            ),
            _ => return Err(VbaError::TYPE_MISMATCH.into()),
        })
    }

    fn fields_to_udt(&mut self, id: UdtId, fv: &[FieldValue]) -> VResult<Variant> {
        let h = self.alloc_udt(id)?;
        let ir = self.ir.clone();
        let info = &ir.program.udts[id.index()];
        for (i, (f, v)) in info.fields.iter().zip(fv).enumerate() {
            let nv = self.field_to_variant(&f.ty, v)?;
            if let Some(u) = self.udts.get_mut(&h) {
                if let (Variant::Array(old), Variant::Array(new)) = (&mut u.fields[i], &nv) {
                    for (o, n) in old.iter_mut().zip(new.iter()) {
                        *o = n.clone();
                    }
                } else {
                    u.fields[i] = nv;
                }
            }
        }
        Ok(udt_variant(h))
    }

    fn field_to_variant(&mut self, ty: &Type, v: &FieldValue) -> VResult<Variant> {
        Ok(match (ty, v) {
            (Type::Byte, FieldValue::Int(n)) => Variant::Byte(*n as u8),
            (Type::Boolean, FieldValue::Int(n)) => Variant::Boolean(*n != 0),
            (Type::Integer, FieldValue::Int(n)) => Variant::Integer(*n as i16),
            (Type::Long | Type::Enum(_), FieldValue::Int(n)) => Variant::Long(*n as i32),
            (Type::LongLong, FieldValue::Int(n)) => Variant::LongLong(*n),
            (Type::Currency, FieldValue::Int(n)) => Variant::Currency(Currency::from_scaled(*n)),
            (Type::Single, FieldValue::Float(f)) => Variant::Single(*f as f32),
            (Type::Double, FieldValue::Float(f)) => Variant::Double(*f),
            (Type::Date, FieldValue::Float(f)) => {
                Variant::Date(Date::from_serial(*f).map_err(Fault::from)?)
            }
            (Type::FixedString(_), FieldValue::Str(s)) => Variant::String(s.clone()),
            (Type::Udt(u), FieldValue::Struct(fs)) => self.fields_to_udt(*u, fs)?,
            (Type::Array { elem, .. }, FieldValue::Array(items)) => {
                let mut vals = Vec::new();
                for i in items {
                    vals.push(self.field_to_variant(elem, i)?);
                }
                let mut a = VbaArray::new_fixed(
                    crate::heap::elem_vartype(elem),
                    &[vba_value::Bounds::new(0, vals.len() as i32 - 1)],
                )?;
                for (i, v) in vals.into_iter().enumerate() {
                    a.set(&[i as i32], v)?;
                }
                Variant::Array(Box::new(a))
            }
            _ => return Err(bridge("unexpected marshaled member")),
        })
    }

    fn value_from_arg(&mut self, ty: &Type, a: &ArgValue, current: &Variant) -> VResult<Variant> {
        Ok(match (ty, a) {
            (Type::Byte, ArgValue::Int(n)) => Variant::Byte(*n as u8),
            (Type::Boolean, ArgValue::Int(n)) => Variant::Boolean(*n != 0),
            (Type::Integer, ArgValue::Int(n)) => Variant::Integer(*n as i16),
            (Type::Long | Type::Enum(_), ArgValue::Int(n)) => Variant::Long(*n as i32),
            (Type::LongLong, ArgValue::Int(n)) => Variant::LongLong(*n),
            (Type::Currency, ArgValue::Int(n)) => Variant::Currency(Currency::from_scaled(*n)),
            (Type::Single, ArgValue::Float(f)) => Variant::Single(*f as f32),
            (Type::Double, ArgValue::Float(f)) => Variant::Double(*f),
            (Type::Date, ArgValue::Float(f)) => {
                Variant::Date(Date::from_serial(*f).map_err(Fault::from)?)
            }
            (Type::String, ArgValue::Str(s)) => Variant::String(s.clone()),
            (Type::FixedString(n), ArgValue::Str(s)) => {
                let mut u = s.as_units().to_vec();
                u.resize(*n as usize, u16::from(b' '));
                Variant::String(VbaString::from_units(u))
            }
            (Type::Variant, ArgValue::Int(n)) => Variant::Long(*n as i32),
            (Type::Variant, ArgValue::Float(f)) => Variant::Double(*f),
            (Type::Variant, ArgValue::Str(s)) => Variant::String(s.clone()),
            (Type::Any, ArgValue::Str(s)) => Variant::String(s.clone()),
            (Type::Udt(u), ArgValue::Bytes(b)) => {
                let ir = self.ir.clone();
                let layout = self.udt_layout(&ir, *u)?;
                let fv = layout
                    .decode(b, vba_ffi::Codepage::Windows1252)
                    .map_err(|e| bridge(e.to_string()))?;
                self.fields_to_udt(*u, &fv)?
            }
            (Type::Array { elem, .. }, ArgValue::Bytes(b)) if **elem == Type::Byte => {
                let mut cur = match current {
                    Variant::Array(a) => (**a).clone(),
                    _ => return Err(VbaError::TYPE_MISMATCH.into()),
                };
                for (e, byte) in cur.iter_mut().zip(b) {
                    *e = Variant::Byte(*byte);
                }
                Variant::Array(Box::new(cur))
            }
            (Type::Array { elem, .. }, ArgValue::Array(items)) => {
                let mut cur = match current {
                    Variant::Array(a) => (**a).clone(),
                    _ => return Err(VbaError::TYPE_MISMATCH.into()),
                };
                let placeholder = Variant::Empty;
                for (e, it) in cur.iter_mut().zip(items) {
                    *e = self.value_from_arg(elem, it, &placeholder)?;
                }
                Variant::Array(Box::new(cur))
            }
            (_, ArgValue::Bytes(_)) => current.clone(),
            _ => return Err(bridge("unexpected marshaled argument")),
        })
    }

    /// Calls a `Declare`d procedure.
    pub(crate) fn exec_declare(
        &mut self,
        ir: &IrProgram,
        d: DeclareId,
        args: &[CallArg],
    ) -> VResult<Variant> {
        let info = &ir.program.declares[d.index()];
        let decl = self.validated(ir, d)?;
        let library = lossy(&info.lib);
        let procedure = info
            .alias
            .as_deref()
            .map(lossy)
            .unwrap_or_else(|| info.name.to_string());
        self.host
            .check(Capability::NativeDeclare, &format!("{library}!{procedure}"))
            .map_err(Fault::from)?;
        if args.len() != info.params.len() {
            return Err(VbaError::new(450).into());
        }
        let mut values = Vec::with_capacity(args.len());
        let mut locs: Vec<Option<Loc>> = Vec::with_capacity(args.len());
        for (a, p) in args.iter().zip(&info.params) {
            let (v, loc) = match a {
                CallArg::Val(op) => (self.val(op)?, None),
                CallArg::Ref(r) => {
                    let l = self.reg_loc(*r)?;
                    (self.load(&l)?, Some(l))
                }
                CallArg::Missing => return Err(VbaError::new(449).into()),
            };
            let v = self.value_of(v)?;
            let _ = p;
            values.push(v);
            locs.push(loc);
        }
        let spec_params = decl.spec().params.clone();
        let target = ir.program.profile.target;
        self.ensure_bridge(target);
        let resolved = {
            let reg = self.declare.registry.as_ref().expect("registry");
            reg.resolve_declare(&decl)
        };
        let resolved = match resolved {
            Ok(r) => r,
            Err(WinApiError::Unsupported { symbol, reason }) => {
                return self
                    .declare_via_host(info, &library, &procedure, values, locs, symbol, reason);
            }
            Err(e) => return Err(bridge(e.to_string())),
        };
        let mut avs = Vec::with_capacity(values.len());
        for (v, p) in values.iter().zip(&spec_params) {
            avs.push(self.to_arg_value(&p.kind, v)?);
        }
        let ctx = self.declare.ctx.as_mut().expect("context");
        let codepage = ctx.ansi_codepage;
        let mut prepared = PreparedCall::prepare(&decl, &mut ctx.memory, &avs, codepage)
            .map_err(|e| ffi_fault(&e))?;
        let registry = self.declare.registry.as_ref().expect("registry");
        let ret = match registry.call(ctx, &resolved, prepared.args()) {
            Ok(r) => r,
            Err(e) => {
                prepared.release(&mut ctx.memory);
                return Err(match e {
                    WinApiError::Memory(m) => Fault::Err(
                        RtErr::new(53, format!("emulated memory fault: {m}"))
                            .with_origin(ErrOrigin::Bridge),
                    ),
                    other => bridge(other.to_string()),
                });
            }
        };
        let outcome = prepared
            .finish(&decl, &mut ctx.memory, ret)
            .map_err(|e| ffi_fault(&e))?;
        self.err.last_dll_error = ctx.last_error() as i32;
        // Write ByRef (and String) arguments back.
        for (i, (loc, back)) in locs.iter().zip(&outcome.args).enumerate() {
            let Some(loc) = loc else { continue };
            let p = &info.params[i];
            let kind = &spec_params[i].kind;
            let writes = spec_params[i].passing == Passing::ByRef
                || matches!(kind, ParamKind::String | ParamKind::Any);
            if !writes {
                continue;
            }
            let ty = if p.is_array && !matches!(p.ty, Type::Array { .. }) {
                Type::array_of(p.ty.clone(), None)
            } else {
                p.ty.clone()
            };
            let cur = self.load(loc)?;
            let nv = self.value_from_arg(&ty, back, &cur)?;
            self.store(loc, nv)?;
        }
        match (&info.is_function, outcome.ret) {
            (true, Some(av)) => {
                let cur = Variant::Empty;
                self.value_from_arg(&info.ret, &av, &cur)
            }
            _ => Ok(Variant::Empty),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn declare_via_host(
        &mut self,
        info: &vba_sema::DeclareInfo,
        library: &str,
        procedure: &str,
        values: Vec<Variant>,
        locs: Vec<Option<Loc>>,
        symbol: String,
        reason: String,
    ) -> VResult<Variant> {
        let mut req = DeclareRequest {
            library: library.to_owned(),
            procedure: procedure.to_owned(),
            args: values
                .iter()
                .zip(&locs)
                .map(|(v, l)| {
                    if l.is_some() {
                        AutoArg::by_ref(v.clone())
                    } else {
                        AutoArg::val(v.clone())
                    }
                })
                .collect(),
            return_type: info.is_function.then(|| info.ret.var_type()).flatten(),
        };
        match self.host.declare_hook(&mut req) {
            Ok(v) => {
                for (a, l) in req.args.iter().zip(&locs) {
                    if let (true, Some(l)) = (a.by_ref, l) {
                        self.store(l, a.value.clone())?;
                    }
                }
                Ok(v)
            }
            Err(e) if e.number == 48 => Err(Fault::Unsupported(format!(
                "Declare {symbol} is not implemented: {reason}"
            ))),
            Err(e) => Err(Fault::Err(RtErr::from(e).with_origin(ErrOrigin::Bridge))),
        }
    }
}

fn ffi_fault(e: &FfiError) -> Fault {
    match e {
        FfiError::ArgumentMismatch { .. } => Fault::Err(
            RtErr::new(13, format!("Declare argument: {e}")).with_origin(ErrOrigin::Bridge),
        ),
        other => bridge(other.to_string()),
    }
}
