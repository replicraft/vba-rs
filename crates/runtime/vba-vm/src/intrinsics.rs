//! Intrinsic functions, `For Each` iteration, `Debug.Print` and file
//! statements.

use vba_ast::OpenMode;
use vba_host::{InputBoxRequest, MsgBoxRequest};
use vba_ir::{CallArg, Function, IoOp, OutItem};
use vba_profile::Bitness;
use vba_sema::Type;
use vba_stdlib::Ctx;
use vba_stdlib::interaction::PrintHelper;
use vba_value::{CompareMode, PointerWidth, VarType, Variant, VbaError, VbaString, convert};

use crate::calls::is_missing_marker;
use crate::error::{Fault, RtErr, VResult};
use crate::heap::elem_vartype;
use crate::printfmt::format_print;
use crate::services::{IoReply, IoRequest, PrintItem, RecordValue, ServiceError, Shape};
use crate::state::*;
use crate::vm::Vm;

fn svc<T>(r: Result<T, ServiceError>) -> VResult<T> {
    r.map_err(|e| match e {
        ServiceError::Vba(i) => Fault::Err(RtErr::from(i)),
        ServiceError::Unsupported(s) => Fault::Unsupported(s),
        ServiceError::Limit(l) => Fault::Limit(l),
    })
}

/// Intrinsics whose object arguments are not replaced by their default
/// member.
const RAW_OBJECT_ARGS: &[&str] = &[
    "typename",
    "vartype",
    "isobject",
    "isnull",
    "isempty",
    "iserror",
    "isarray",
    "ismissing",
    "cvar",
    "choose",
    "iif",
    "switch",
    "array",
    "isdate",
    "isnumeric",
];

impl Vm {
    // ---- arguments ----------------------------------------------------------------

    fn intrinsic_args(
        &mut self,
        args: &[CallArg],
        resolve_objects: bool,
    ) -> VResult<Vec<Option<Variant>>> {
        let mut out = Vec::with_capacity(args.len());
        for a in args {
            let v = match a {
                CallArg::Val(op) => Some(self.val(op)?),
                CallArg::Ref(r) => {
                    let l = self.reg_loc(*r)?;
                    Some(self.load(&l)?)
                }
                CallArg::Missing => None,
            };
            let v = match v {
                Some(v @ Variant::Object(_)) if resolve_objects => Some(self.value_of(v)?),
                other => other,
            };
            out.push(v);
        }
        Ok(out)
    }

    fn text_arg(&mut self, v: &Option<Variant>) -> VResult<String> {
        match v {
            Some(v) => {
                let v = self.value_of(v.clone())?;
                Ok(convert::to_vba_string(&v)?.to_string_lossy())
            }
            None => Err(VbaError::new(449).into()),
        }
    }

    fn opt_text(&mut self, v: &Option<Variant>) -> VResult<Option<String>> {
        match v {
            None | Some(Variant::Empty) => Ok(None),
            Some(_) => self.text_arg(v).map(Some),
        }
    }

    // ---- dispatch ---------------------------------------------------------------------

    pub(crate) fn exec_intrinsic(
        &mut self,
        name: &str,
        dollar: bool,
        args: &[CallArg],
    ) -> VResult<RegVal> {
        let lname = name.to_ascii_lowercase();
        match lname.as_str() {
            "tab" | "spc" => {
                let a = self.intrinsic_args(args, true)?;
                let h = if lname == "tab" {
                    PrintHelper::Tab(match a.first() {
                        Some(Some(v)) => Some(convert::to_long(v)?),
                        _ => None,
                    })
                } else {
                    PrintHelper::Spc(
                        a.first()
                            .and_then(Option::as_ref)
                            .map(convert::to_long)
                            .transpose()?
                            .ok_or(VbaError::new(449))?
                            .max(0),
                    )
                };
                return Ok(RegVal::Print(h));
            }
            "lbound" | "ubound" => return self.exec_bound(&lname, args).map(RegVal::V),
            _ => {}
        }
        let raw = RAW_OBJECT_ARGS.contains(&lname.as_str());
        let a = self.intrinsic_args(args, !raw)?;
        let v = match lname.as_str() {
            "ismissing" => {
                Variant::Boolean(matches!(a.first(), Some(Some(v)) if is_missing_marker(v)))
            }
            "erl" => Variant::Long(self.erl as i32),
            "beep" => Variant::Empty,
            "doevents" => {
                self.host.budget().charge_step().map_err(Fault::from)?;
                self.host
                    .idle()
                    .map_err(|e| Fault::Cancelled(e.description))?;
                self.drain_events()?;
                Variant::Integer(0)
            }
            "createobject" => {
                let progid = self.text_arg(a.first().unwrap_or(&None))?;
                let server = self.opt_text(a.get(1).unwrap_or(&None))?;
                let v = self
                    .host
                    .create_object(&progid, server.as_deref())
                    .map_err(Fault::from)?;
                self.drain_events()?;
                v
            }
            "getobject" => {
                let path = self.opt_text(a.first().unwrap_or(&None))?;
                let class = self.opt_text(a.get(1).unwrap_or(&None))?;
                let v = self
                    .host
                    .get_object(path.as_deref(), class.as_deref())
                    .map_err(Fault::from)?;
                self.drain_events()?;
                v
            }
            "msgbox" => {
                let prompt = self.text_arg(a.first().unwrap_or(&None))?;
                let buttons = match a.get(1) {
                    Some(Some(v)) => convert::to_long(v)?,
                    _ => 0,
                };
                let title = self
                    .opt_text(a.get(2).unwrap_or(&None))?
                    .unwrap_or_else(|| self.cfg.project_name.clone());
                let r = self
                    .host
                    .ui()
                    .msg_box(&MsgBoxRequest {
                        prompt,
                        buttons,
                        title,
                    })
                    .map_err(Fault::from)?;
                Variant::Long(r)
            }
            "inputbox" => {
                let prompt = self.text_arg(a.first().unwrap_or(&None))?;
                let title = self
                    .opt_text(a.get(1).unwrap_or(&None))?
                    .unwrap_or_else(|| self.cfg.project_name.clone());
                let default = self
                    .opt_text(a.get(2).unwrap_or(&None))?
                    .unwrap_or_default();
                let r = self
                    .host
                    .ui()
                    .input_box(&InputBoxRequest {
                        prompt,
                        title,
                        default,
                    })
                    .map_err(Fault::from)?;
                Variant::from_text(&r.unwrap_or_default())
            }
            "midstatement" => {
                self.exec_mid_statement(args, &a)?;
                Variant::Empty
            }
            "midbstatement" => {
                return Err(Fault::Unsupported(
                    "the MidB statement (byte-wise replacement)".into(),
                ));
            }
            "typename" => match a.first() {
                Some(Some(Variant::Object(vba_value::ObjectRef::Handle(h)))) => {
                    Variant::from_text(&self.object_type_name(*h))
                }
                Some(Some(Variant::Object(vba_value::ObjectRef::Nothing))) => {
                    Variant::from_text("Nothing")
                }
                Some(Some(Variant::UserDefined(h))) => {
                    let n = self
                        .udts
                        .get(&h.0)
                        .map(|u| self.ir.program.udts[u.udt.index()].name.to_string())
                        .unwrap_or_default();
                    Variant::from_text(&n)
                }
                _ => self.call_stdlib(name, dollar, &a)?,
            },
            _ => {
                if vba_stdlib::lookup(name).is_some() {
                    self.call_stdlib(name, dollar, &a)?
                } else {
                    svc(self.services.call(&lname, &a))?
                }
            }
        };
        self.check_string(&v)?;
        Ok(RegVal::V(v))
    }

    fn call_stdlib(&mut self, name: &str, dollar: bool, a: &[Option<Variant>]) -> VResult<Variant> {
        let r = vba_stdlib::lookup(name)
            .ok_or_else(|| Fault::Unsupported(format!("unknown function `{name}`")))?;
        let module = self.frames.last().expect("frame").module;
        let opts = &self.ir.program.module(module).options;
        let mut ctx = Ctx::new(&*self.std_host, &mut self.std_state);
        ctx.compare = match opts.compare {
            vba_ast::CompareMode::Binary => CompareMode::Binary,
            _ => CompareMode::Text,
        };
        ctx.array_base = i32::from(opts.base);
        ctx.pointer_width = match self.ir.program.profile.target.bitness {
            Bitness::B64 => PointerWidth::Bits64,
            _ => PointerWidth::Bits32,
        };
        Ok(r.info.invoke(dollar, &mut ctx, a)?)
    }

    fn exec_bound(&mut self, lname: &str, args: &[CallArg]) -> VResult<Variant> {
        let upper = lname == "ubound";
        let dim = match args.get(1) {
            Some(CallArg::Val(op)) => {
                let v = self.val(op)?;
                self.long_of(v)?
            }
            Some(CallArg::Ref(r)) => {
                let l = self.reg_loc(*r)?;
                let v = self.load(&l)?;
                self.long_of(v)?
            }
            _ => 1,
        };
        let loc = match args.first() {
            Some(CallArg::Ref(r)) => Some(self.reg_loc(*r)?),
            Some(CallArg::Val(vba_ir::Operand::Reg(r))) => {
                match &self.frames.last().expect("frame").regs[r.0 as usize] {
                    RegVal::Alias(l) => Some(l.clone()),
                    _ => None,
                }
            }
            Some(CallArg::Missing) | None => return Err(VbaError::new(449).into()),
            _ => None,
        };
        let r = match loc {
            Some(l) => {
                let v = self.peek(&l)?;
                vba_stdlib::info::bound(&v, dim, upper)?
            }
            None => {
                let Some(CallArg::Val(op)) = args.first() else {
                    return Err(VbaError::new(449).into());
                };
                let v = self.val(op)?;
                vba_stdlib::info::bound(&v, dim, upper)?
            }
        };
        Ok(Variant::Long(r))
    }

    fn exec_mid_statement(&mut self, args: &[CallArg], a: &[Option<Variant>]) -> VResult<()> {
        let Some(CallArg::Ref(r)) = args.first() else {
            return Err(Fault::Internal("Mid statement target".into()));
        };
        let loc = self.reg_loc(*r)?;
        let target = self.load(&loc)?;
        let t = convert::to_vba_string(&target)?;
        let start = match a.get(1) {
            Some(Some(v)) => convert::to_long(v)?,
            _ => return Err(VbaError::new(449).into()),
        };
        let value = match a.get(3) {
            Some(Some(v)) => convert::to_vba_string(v)?,
            _ => return Err(VbaError::new(449).into()),
        };
        let len = match a.get(2) {
            Some(Some(v)) => Some(convert::to_long(v)?),
            _ => None,
        };
        if start < 1 || len.is_some_and(|l| l < 0) {
            return Err(VbaError::INVALID_PROCEDURE_CALL.into());
        }
        let mut units = t.as_units().to_vec();
        let start0 = start as usize - 1;
        if start0 >= units.len() {
            // Past the end: nothing to replace (error 5 for a start beyond
            // Len + 1 is documented for Mid$ the function, not the
            // statement; an out-of-range start leaves the string unchanged).
            if start0 > units.len() {
                return Err(VbaError::INVALID_PROCEDURE_CALL.into());
            }
            return Ok(());
        }
        let n = len
            .map_or(value.len(), |l| l as usize)
            .min(value.len())
            .min(units.len() - start0);
        units[start0..start0 + n].copy_from_slice(&value.as_units()[..n]);
        self.store(&loc, Variant::String(VbaString::from_units(units)))
    }

    // ---- For Each -------------------------------------------------------------------------

    fn hold(&mut self, v: Variant) {
        if let Some(f) = self.frames.last_mut() {
            f.held.push(v);
        }
    }

    pub(crate) fn iter_init(&mut self, v: Variant) -> VResult<Iter> {
        match v {
            Variant::Array(a) => {
                if a.dimensions() == 0 {
                    return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE.into());
                }
                Ok(Iter::Array {
                    items: a.iter().cloned().collect(),
                    pos: 0,
                })
            }
            Variant::Object(vba_value::ObjectRef::Handle(h)) => match vm_id(h) {
                Some(id) => match self.objects.get(&id).map(|o| &o.kind) {
                    Some(ObjKind::Collection(_)) => {
                        self.retain_handle(h);
                        self.hold(Variant::Object(vba_value::ObjectRef::Handle(h)));
                        Ok(Iter::Coll { id, pos: 0 })
                    }
                    _ => Err(VbaError::new(438).into()),
                },
                None => {
                    let eh = self.external_enumerator(h)?;
                    self.hold(Variant::Object(vba_value::ObjectRef::Handle(eh)));
                    Ok(Iter::Ext { enumerator: eh })
                }
            },
            Variant::Object(vba_value::ObjectRef::Nothing) | Variant::Empty => {
                Err(VbaError::new(91).into())
            }
            _ => Err(VbaError::TYPE_MISMATCH.into()),
        }
    }

    pub(crate) fn iter_next(&mut self, it: &mut Iter) -> VResult<Option<Variant>> {
        match it {
            Iter::Array { items, pos } => {
                let v = items.get(*pos).cloned();
                if v.is_some() {
                    *pos += 1;
                }
                Ok(v)
            }
            Iter::Coll { id, pos } => {
                let v = match self.objects.get(id).map(|o| &o.kind) {
                    Some(ObjKind::Collection(items)) => items.get(*pos).map(|i| i.value.clone()),
                    _ => None,
                };
                if v.is_some() {
                    *pos += 1;
                }
                Ok(v)
            }
            Iter::Ext { enumerator } => {
                let v = self.external_next(*enumerator)?;
                if let Some(v) = &v {
                    self.own(v);
                }
                Ok(v)
            }
        }
    }

    // ---- Debug.Print ------------------------------------------------------------------------

    pub(crate) fn print_items(&mut self, items: &[OutItem]) -> VResult<Vec<PrintItem>> {
        let mut out = Vec::with_capacity(items.len());
        for it in items {
            out.push(match it {
                OutItem::Semicolon => PrintItem::Semicolon,
                OutItem::Comma => PrintItem::Comma,
                OutItem::Value(op) => {
                    if let vba_ir::Operand::Reg(r) = op
                        && let RegVal::Print(h) =
                            &self.frames.last().expect("frame").regs[r.0 as usize]
                    {
                        match h {
                            PrintHelper::Tab(n) => PrintItem::Tab(*n),
                            PrintHelper::Spc(n) => PrintItem::Spc(*n),
                        }
                    } else {
                        let v = self.val(op)?;
                        let v = self.value_of(v)?;
                        PrintItem::Value(v)
                    }
                }
            });
        }
        Ok(out)
    }

    pub(crate) fn exec_debug_print(&mut self, items: &[OutItem]) -> VResult<()> {
        let items = self.print_items(items)?;
        let text = format_print(&items, &mut self.debug_col);
        self.debug.write(&text);
        Ok(())
    }

    pub(crate) fn debug_write_line(&mut self, text: &str) {
        self.debug.write(text);
        self.debug.write("\n");
        self.debug_col = 0;
    }

    // ---- file statements ------------------------------------------------------------------------

    fn shape_of(&self, t: &Type, cur: &Variant) -> VResult<Shape> {
        let ir = self.ir.clone();
        Ok(match t {
            Type::Byte
            | Type::Boolean
            | Type::Integer
            | Type::Long
            | Type::LongLong
            | Type::Single
            | Type::Double
            | Type::Currency
            | Type::Date
            | Type::Decimal
            | Type::Enum(_)
            | Type::Variant => Shape::Scalar(elem_vartype(t)),
            Type::String => Shape::VarString {
                current_len: match cur {
                    Variant::String(s) => s.len(),
                    _ => 0,
                },
            },
            Type::FixedString(n) => Shape::FixedString(*n),
            Type::Udt(u) => {
                let info = &ir.program.udts[u.index()];
                let vals: Vec<Variant> = match cur {
                    Variant::UserDefined(h) => self
                        .udts
                        .get(&h.0)
                        .map(|x| x.fields.clone())
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                let mut fs = Vec::new();
                for (i, f) in info.fields.iter().enumerate() {
                    let cv = vals.get(i).cloned().unwrap_or(Variant::Empty);
                    fs.push(self.shape_of(&f.ty, &cv)?);
                }
                Shape::Udt(fs)
            }
            Type::Array { elem, .. } => {
                let Variant::Array(a) = cur else {
                    return Err(VbaError::TYPE_MISMATCH.into());
                };
                if !a.is_fixed() {
                    return Err(Fault::Unsupported(
                        "Get/Put of a dynamic array (the array descriptor is not modelled)".into(),
                    ));
                }
                let first = a.iter().next().cloned().unwrap_or(Variant::Empty);
                Shape::Array {
                    elem: Box::new(self.shape_of(elem, &first)?),
                    count: a.len(),
                }
            }
            other => {
                return Err(Fault::Unsupported(format!(
                    "Get/Put of a value of type {other}"
                )));
            }
        })
    }

    fn record_of(&self, v: &Variant) -> VResult<RecordValue> {
        Ok(match v {
            Variant::UserDefined(h) => {
                let u = self
                    .udts
                    .get(&h.0)
                    .ok_or_else(|| Fault::Internal("dangling user-defined type".into()))?;
                RecordValue::Fields(
                    u.fields
                        .iter()
                        .map(|f| self.record_of(f))
                        .collect::<VResult<_>>()?,
                )
            }
            Variant::Array(a) => RecordValue::Fields(
                a.iter()
                    .map(|f| self.record_of(f))
                    .collect::<VResult<_>>()?,
            ),
            other => RecordValue::Scalar(other.clone()),
        })
    }

    fn value_from_record(&mut self, t: &Type, rv: &RecordValue, cur: &Variant) -> VResult<Variant> {
        match (t, rv) {
            (Type::Udt(u), RecordValue::Fields(fs)) => {
                let ir = self.ir.clone();
                let h = self.alloc_udt(*u)?;
                let info = &ir.program.udts[u.index()];
                let cur_fields: Vec<Variant> = match cur {
                    Variant::UserDefined(ch) => self
                        .udts
                        .get(&ch.0)
                        .map(|x| x.fields.clone())
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                for (i, (f, r)) in info.fields.iter().zip(fs).enumerate() {
                    let cv = cur_fields.get(i).cloned().unwrap_or(Variant::Empty);
                    let nv = self.value_from_record(&f.ty, r, &cv)?;
                    if let Some(x) = self.udts.get_mut(&h) {
                        x.fields[i] = nv;
                    }
                }
                Ok(udt_variant(h))
            }
            (Type::Array { elem, .. }, RecordValue::Fields(fs)) => {
                let Variant::Array(a) = cur else {
                    return Err(VbaError::TYPE_MISMATCH.into());
                };
                let mut a = (**a).clone();
                let firsts: Vec<Variant> = a.iter().cloned().collect();
                for ((e, r), c) in a.iter_mut().zip(fs).zip(firsts) {
                    *e = self.value_from_record(elem, r, &c)?;
                }
                Ok(Variant::Array(Box::new(a)))
            }
            (t, RecordValue::Scalar(v)) => self.convert_value(v.clone(), t),
            _ => Err(Fault::Internal("record does not match its shape".into())),
        }
    }

    fn reg_type(func: &Function, r: vba_ir::Reg) -> Type {
        func.regs[r.0 as usize].clone()
    }

    pub(crate) fn exec_io(&mut self, func: &Function, op: &IoOp) -> VResult<()> {
        let file_no = |vm: &mut Vm, o: &vba_ir::Operand| -> VResult<i32> {
            let v = vm.val(o)?;
            vm.long_of(v)
        };
        let opt_i64 = |vm: &mut Vm, o: &Option<vba_ir::Operand>| -> VResult<Option<i64>> {
            match o {
                Some(o) => {
                    let v = vm.val(o)?;
                    let v = vm.value_of(v)?;
                    Ok(Some(convert::to_longlong(&v)?))
                }
                None => Ok(None),
            }
        };
        match op {
            IoOp::Open {
                path,
                mode,
                access,
                lock,
                file,
                record_len,
            } => {
                let p = self.val(path)?;
                let p = self.value_of(p)?;
                let path = convert::to_vba_string(&p)?.to_string_lossy();
                let file = file_no(self, file)?;
                let record_len = match record_len {
                    Some(o) => Some(file_no(self, o)?),
                    None => None,
                };
                let _: OpenMode = *mode;
                svc(self.services.io(IoRequest::Open {
                    path,
                    mode: *mode,
                    access: *access,
                    lock: *lock,
                    file,
                    record_len,
                }))?;
            }
            IoOp::Close(files) => {
                let mut v = Vec::new();
                for f in files {
                    v.push(file_no(self, f)?);
                }
                svc(self.services.io(IoRequest::Close(v)))?;
            }
            IoOp::Print { file, items } | IoOp::Write { file, items } => {
                let file = file_no(self, file)?;
                let items = self.print_items(items)?;
                let req = if matches!(op, IoOp::Print { .. }) {
                    IoRequest::Print { file, items }
                } else {
                    IoRequest::Write { file, items }
                };
                svc(self.services.io(req))?;
            }
            IoOp::Input { file, targets } => {
                let file = file_no(self, file)?;
                let mut tys = Vec::new();
                let mut locs = Vec::new();
                for t in targets {
                    let ty = Self::reg_type(func, *t);
                    tys.push(ty);
                    locs.push(self.reg_loc(*t)?);
                }
                let kinds: Vec<VarType> = tys
                    .iter()
                    .map(|t| match t {
                        Type::FixedString(_) => VarType::String,
                        t => elem_vartype(t),
                    })
                    .collect();
                let reply = svc(self.services.io(IoRequest::Input {
                    file,
                    targets: kinds,
                }))?;
                let IoReply::Values(vals) = reply else {
                    return Err(Fault::Internal("Input reply".into()));
                };
                for ((v, ty), loc) in vals.into_iter().zip(&tys).zip(&locs) {
                    let v = self.convert_value(v, ty)?;
                    self.store(loc, v)?;
                }
            }
            IoOp::LineInput { file, target } => {
                let file = file_no(self, file)?;
                let ty = Self::reg_type(func, *target);
                let loc = self.reg_loc(*target)?;
                let reply = svc(self.services.io(IoRequest::LineInput { file }))?;
                let IoReply::Line(s) = reply else {
                    return Err(Fault::Internal("Line Input reply".into()));
                };
                let v = self.convert_value(Variant::String(s), &ty)?;
                self.store(&loc, v)?;
            }
            IoOp::Get {
                file,
                record,
                target,
            } => {
                let file = file_no(self, file)?;
                let record = opt_i64(self, record)?;
                let Some(t) = target else {
                    return Err(VbaError::new(449).into());
                };
                let ty = Self::reg_type(func, *t);
                let loc = self.reg_loc(*t)?;
                let cur = self.load(&loc)?;
                let shape = self.shape_of(&ty, &cur)?;
                let reply = svc(self.services.io(IoRequest::Get {
                    file,
                    record,
                    shape,
                }))?;
                let IoReply::Record(rv) = reply else {
                    return Err(Fault::Internal("Get reply".into()));
                };
                let v = self.value_from_record(&ty, &rv, &cur)?;
                self.store(&loc, v)?;
            }
            IoOp::Put {
                file,
                record,
                value,
            } => {
                let file = file_no(self, file)?;
                let record = opt_i64(self, record)?;
                let Some(value) = value else {
                    return Err(VbaError::new(449).into());
                };
                let v = self.val(value)?;
                let ty = match value {
                    vba_ir::Operand::Reg(r) => Self::reg_type(func, *r),
                    vba_ir::Operand::Const(_) => Type::Variant,
                };
                let shape = match (&ty, &v) {
                    (Type::Variant, _) if matches!(value, vba_ir::Operand::Reg(_)) => {
                        // A Variant variable is written with its type tag.
                        Shape::Scalar(VarType::Variant)
                    }
                    (Type::Variant, v)
                        if !matches!(v, Variant::UserDefined(_) | Variant::Array(_)) =>
                    {
                        // A constant or Variant is written with its own subtype.
                        match v {
                            Variant::String(s) => Shape::VarString {
                                current_len: s.len(),
                            },
                            other => Shape::Scalar(other.vartype()),
                        }
                    }
                    _ => self.shape_of(&ty, &v)?,
                };
                let rec = self.record_of(&v)?;
                svc(self.services.io(IoRequest::Put {
                    file,
                    record,
                    shape,
                    value: rec,
                }))?;
            }
            IoOp::Seek { file, position } => {
                let file = file_no(self, file)?;
                let p = opt_i64(self, position)?.ok_or(VbaError::new(449))?;
                svc(self.services.io(IoRequest::Seek { file, position: p }))?;
            }
            IoOp::Lock { file, from, to } => {
                let file = file_no(self, file)?;
                let (from, to) = (opt_i64(self, from)?, opt_i64(self, to)?);
                svc(self.services.io(IoRequest::Lock { file, from, to }))?;
            }
            IoOp::Unlock { file, from, to } => {
                let file = file_no(self, file)?;
                let (from, to) = (opt_i64(self, from)?, opt_i64(self, to)?);
                svc(self.services.io(IoRequest::Unlock { file, from, to }))?;
            }
            IoOp::Width { file, width } => {
                let file = file_no(self, file)?;
                let w = opt_i64(self, width)?.unwrap_or(0) as i32;
                svc(self.services.io(IoRequest::Width { file, width: w }))?;
            }
            IoOp::Name { old, new } => {
                let o = self.val(old)?;
                let o = self.value_of(o)?;
                let n = self.val(new)?;
                let n = self.value_of(n)?;
                svc(self.services.io(IoRequest::Name {
                    old: convert::to_vba_string(&o)?.to_string_lossy(),
                    new: convert::to_vba_string(&n)?.to_string_lossy(),
                }))?;
            }
        }
        Ok(())
    }
}
