//! Intrinsic functions, `For Each` iteration, `Debug.Print` and file
//! statements.

use vba_ast::OpenMode;
use vba_host::{InputBoxRequest, MsgBoxRequest};
use vba_ir::{CallArg, Function, IoOp, OutItem};
use vba_profile::Bitness;
use vba_sema::Type;
use vba_stdlib::{Ctx, StdError, StdResult, WorkMeter};

/// Accounts the scanning work of built-ins against the step budget.
/// Work of at least this many units also asks the host whether the run was
/// cancelled, so a long library call is not deaf to cancellation and to the
/// time allowance of a call scope.
const POLL_WORK_UNITS: u64 = 1024;

pub(crate) struct BudgetWork<'a>(pub(crate) &'a dyn vba_host::Host);

impl WorkMeter for BudgetWork<'_> {
    fn charge(&self, units: u64) -> StdResult<()> {
        self.0
            .budget()
            .charge_work(units)
            .map_err(|_| StdError::WorkExceeded)?;
        if units >= POLL_WORK_UNITS {
            self.0
                .poll_cancel()
                .map_err(|e| StdError::Cancelled(e.description))?;
        }
        Ok(())
    }
}
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
        ServiceError::Vba(i) => Fault::from(RtErr::from(i)),
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
    "callbyname",
    "setcheckvariant",
    "setcheckobject",
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

    fn opt_bool(&mut self, v: &Option<Variant>) -> VResult<bool> {
        match v {
            None | Some(Variant::Empty) => Ok(false),
            Some(v) => Ok(convert::to_boolean(v)?),
        }
    }

    /// `SaveSetting`, `GetSetting`, `GetAllSettings` and `DeleteSetting`
    /// against the host's store, or the VM's in-memory one.
    fn exec_settings(&mut self, name: &str, a: &[Option<Variant>]) -> VResult<Variant> {
        let app = self.text_arg(a.first().unwrap_or(&None))?;
        let section = self.opt_text(a.get(1).unwrap_or(&None))?;
        let key = match name {
            "savesetting" | "getsetting" => Some(self.text_arg(a.get(2).unwrap_or(&None))?),
            "deletesetting" => self.opt_text(a.get(2).unwrap_or(&None))?,
            _ => None,
        };
        let value = match (name, a.get(3)) {
            ("savesetting", _) => Some(self.text_arg(a.get(3).unwrap_or(&None))?),
            ("getsetting", Some(Some(v))) => Some(convert::to_vba_string(v)?.to_string_lossy()),
            _ => None,
        };
        let host = self.host.clone();
        let store: &dyn vba_host::SettingsStore = match host.settings() {
            Some(s) => s,
            None => &self.settings,
        };
        let need = |section: Option<String>| section.ok_or(VbaError::new(449));
        let key_or_empty = key.clone().unwrap_or_default();
        match name {
            "savesetting" => {
                store
                    .save(
                        &app,
                        &need(section)?,
                        &key_or_empty,
                        &value.unwrap_or_default(),
                    )
                    .map_err(Fault::from)?;
                Ok(Variant::Empty)
            }
            "getsetting" => {
                let got = store
                    .get(&app, &need(section)?, &key_or_empty)
                    .map_err(Fault::from)?;
                Ok(Variant::from_text(&got.or(value).unwrap_or_default()))
            }
            "getallsettings" => {
                let rows = store.get_all(&app, &need(section)?).map_err(Fault::from)?;
                // **doc**: an uninitialized Variant when the application or
                // section does not exist.
                let Some(rows) = rows.filter(|r| !r.is_empty()) else {
                    return Ok(Variant::Empty);
                };
                let upper = i32::try_from(rows.len() - 1).map_err(|_| VbaError::OUT_OF_MEMORY)?;
                let mut arr = vba_value::VbaArray::new_dynamic(vba_value::VarType::Variant);
                arr.redim(
                    &[
                        vba_value::Bounds::new(0, upper),
                        vba_value::Bounds::new(0, 1),
                    ],
                    false,
                )?;
                for (i, (k, v)) in rows.iter().enumerate() {
                    let i = i as i32;
                    arr.set(&[i, 0], Variant::from_text(k))?;
                    arr.set(&[i, 1], Variant::from_text(v))?;
                }
                Ok(Variant::Array(Box::new(arr)))
            }
            _ => {
                store
                    .delete(&app, section.as_deref(), key.as_deref())
                    .map_err(Fault::from)?;
                Ok(Variant::Empty)
            }
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
            "savesetting" | "getsetting" | "getallsettings" | "deletesetting" => {
                self.exec_settings(&lname, &a)?
            }
            "sendkeys" => {
                let keys = self.text_arg(a.first().unwrap_or(&None))?;
                let wait = self.opt_bool(a.get(1).unwrap_or(&None))?;
                self.host.send_keys(&keys, wait).map_err(Fault::from)?;
                Variant::Empty
            }
            "appactivate" => {
                let title = self.text_arg(a.first().unwrap_or(&None))?;
                let wait = self.opt_bool(a.get(1).unwrap_or(&None))?;
                self.host.app_activate(&title, wait).map_err(Fault::from)?;
                Variant::Empty
            }
            "datestatement" | "timestatement" => {
                let Some(Some(v)) = a.first() else {
                    return Err(VbaError::new(449).into());
                };
                let d = convert::to_date(v)?;
                let part = if lname == "datestatement" {
                    vba_host::ClockPart::Date
                } else {
                    vba_host::ClockPart::Time
                };
                self.host.set_clock(part, d).map_err(Fault::from)?;
                Variant::Empty
            }
            "callbyname" => {
                // **doc**: vbMethod 1, vbGet 2, vbLet 4, vbSet 8.
                let Some(Some(recv)) = a.first() else {
                    return Err(VbaError::new(449).into());
                };
                let name = self.text_arg(a.get(1).unwrap_or(&None))?;
                let call_type = match a.get(2) {
                    Some(Some(v)) => convert::to_long(v)?,
                    _ => return Err(VbaError::new(449).into()),
                };
                let kind = match call_type {
                    1 | 2 => vba_ir::InvokeKind::Get,
                    4 => vba_ir::InvokeKind::Put,
                    8 => vba_ir::InvokeKind::PutRef,
                    _ => return Err(VbaError::INVALID_PROCEDURE_CALL.into()),
                };
                let largs: Vec<(Option<String>, crate::calls::BoundArg)> = a
                    .iter()
                    .skip(3)
                    .map(|v| {
                        let b = match v {
                            Some(v) => crate::calls::BoundArg::Val(v.clone()),
                            None => crate::calls::BoundArg::Missing,
                        };
                        (None, b)
                    })
                    .collect();
                self.invoke_value(
                    recv.clone(),
                    &vba_ir::Member::Named(name),
                    kind,
                    largs,
                    true,
                )?
            }
            "reset" => {
                // **doc**: closes every open file and writes the buffers.
                self.services.reset();
                Variant::Empty
            }
            "beep" => Variant::Empty,
            "doevents" => {
                self.host.budget().charge_step().map_err(Fault::from)?;
                self.host
                    .idle()
                    .map_err(|e| Fault::Cancelled(e.description))?;
                self.drain_events()?;
                self.pump_timers()?;
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
                let path = match path {
                    Some(p) if !p.is_empty() => Some(svc(self.services.resolve_object_path(&p))?),
                    other => other,
                };
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
            "len" | "lenb" if matches!(a.first(), Some(Some(Variant::UserDefined(_)))) => {
                let Some(Some(cur)) = a.first() else {
                    return Err(VbaError::new(449).into());
                };
                let Variant::UserDefined(h) = cur else {
                    return Err(VbaError::TYPE_MISMATCH.into());
                };
                let udt = self
                    .udts
                    .get(&h.0)
                    .map(|u| u.udt)
                    .ok_or_else(|| Fault::Internal("dangling user-defined type".into()))?;
                let n = self.record_len(&Type::Udt(udt), cur)?;
                Variant::Long(i32::try_from(n).map_err(|_| VbaError::OVERFLOW)?)
            }
            "lensize" => {
                let n: i32 = match a.first() {
                    Some(Some(Variant::Boolean(_) | Variant::Integer(_))) => 2,
                    Some(Some(Variant::Byte(_))) => 1,
                    Some(Some(Variant::Long(_) | Variant::Single(_))) => 4,
                    Some(Some(
                        Variant::LongLong(_)
                        | Variant::Double(_)
                        | Variant::Currency(_)
                        | Variant::Date(_),
                    )) => 8,
                    _ => return Err(VbaError::TYPE_MISMATCH.into()),
                };
                Variant::Long(n)
            }
            "midstatement" => {
                self.exec_mid_statement(args, &a)?;
                Variant::Empty
            }
            "midbstatement" => {
                self.exec_midb_statement(args, &a)?;
                Variant::Empty
            }
            "setcheckvariant" | "setcheckobject" => match a.into_iter().next().flatten() {
                // MS-VBAL 5.5.2.2.2: a non-object value is Type mismatch into a
                // Variant and Object required into a class or `Object`.
                Some(v @ Variant::Object(_)) => v,
                Some(Variant::Null) if lname == "setcheckobject" => Variant::Null,
                _ if lname == "setcheckvariant" => return Err(VbaError::TYPE_MISMATCH.into()),
                _ => return Err(VbaError::new(424).into()),
            },
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
                        .map(|u| self.ir.program.udt_name(u.udt))
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
        let meter = BudgetWork(&*self.host);
        let mut ctx = Ctx::new(&*self.std_host, &mut self.std_state);
        ctx.work = Some(&meter);
        ctx.compare = match opts.compare {
            vba_ast::CompareMode::Binary => CompareMode::Binary,
            _ => CompareMode::Text,
        };
        ctx.array_base = i32::from(opts.base);
        ctx.max_string_units = self.cfg.max_string_units;
        ctx.max_array_elements = usize::try_from(self.cfg.max_array_elements).unwrap_or(usize::MAX);
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
            // MS-VBAL 5.4.3.5: a start beyond the length is error 5.
            return Err(VbaError::INVALID_PROCEDURE_CALL.into());
        }
        let n = len
            .map_or(value.len(), |l| l as usize)
            .min(value.len())
            .min(units.len() - start0);
        units[start0..start0 + n].copy_from_slice(&value.as_units()[..n]);
        self.store(&loc, Variant::String(VbaString::from_units(units)))
    }

    /// The `MidB` statement: `Mid` on the bytes of the string (MS-VBAL 5.4.3.5).
    fn exec_midb_statement(&mut self, args: &[CallArg], a: &[Option<Variant>]) -> VResult<()> {
        let Some(CallArg::Ref(r)) = args.first() else {
            return Err(Fault::Internal("MidB statement target".into()));
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
        let mut bytes = t.to_bytes_le();
        let vb = value.to_bytes_le();
        let start0 = start as usize - 1;
        if start0 >= bytes.len() {
            return Err(VbaError::INVALID_PROCEDURE_CALL.into());
        }
        let n = len
            .map_or(vb.len(), |l| l as usize)
            .min(vb.len())
            .min(bytes.len() - start0);
        bytes[start0..start0 + n].copy_from_slice(&vb[..n]);
        // The string keeps its length, which is even, so the bytes always
        // form whole code units.
        let s = VbaString::from_bytes_le(&bytes)
            .ok_or_else(|| Fault::Internal("MidB result is not whole code units".into()))?;
        self.store(&loc, Variant::String(s))
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
                    Some(ObjKind::Class { module, .. }) => {
                        let module = *module;
                        self.class_enumerator(id, module)
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

    /// `For Each` over an instance of a class that declares its enumerator
    /// member (`VB_UserMemId = -4`): the member returns the enumerator, a
    /// collection's `_NewEnum` or a host object's.
    fn class_enumerator(&mut self, id: u64, module: vba_sema::ModuleId) -> VResult<Iter> {
        let ir = self.ir.clone();
        let Some(name) = ir.program.module(module).enumerator_member.as_ref() else {
            return Err(VbaError::new(438).into());
        };
        let pick = self
            .procs_by_name
            .get(&(module, name.to_string().to_ascii_lowercase()))
            .and_then(|c| {
                c.iter().copied().find(|p| {
                    matches!(
                        ir.program.proc(*p).kind,
                        vba_ast::ProcKind::Function | vba_ast::ProcKind::PropertyGet
                    )
                })
            })
            .ok_or(VbaError::new(438))?;
        let r = self.call_nested(pick, Some(id), Vec::new())?;
        // The result carries a reference, released when the frame ends.
        self.hold(r.clone());
        match r {
            Variant::Object(vba_value::ObjectRef::Handle(eh)) => match vm_id(eh) {
                Some(cid) => match self.objects.get(&cid).map(|o| &o.kind) {
                    Some(ObjKind::Collection(_)) => Ok(Iter::Coll { id: cid, pos: 0 }),
                    _ => Err(VbaError::new(438).into()),
                },
                None => {
                    let host = self.host.clone();
                    if host
                        .objects()
                        .get(eh)
                        .is_ok_and(|o| o.enumerator().is_some())
                    {
                        Ok(Iter::Ext { enumerator: eh })
                    } else {
                        Err(VbaError::new(438).into())
                    }
                }
            },
            Variant::Object(vba_value::ObjectRef::Nothing) => Err(VbaError::new(91).into()),
            _ => Err(VbaError::new(438).into()),
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

    /// The number of bytes a value of static type `t` occupies in a `Put`
    /// record of a Random file: the size `Len` reports for a user-defined
    /// type (a variable-length `String` counts its two-byte length and its
    /// ANSI bytes, a `Variant` its two-byte subtype and data). The sizes of
    /// the numeric types are the documented ones; the treatment of
    /// variable-length members is unverified.
    fn record_len(&self, t: &Type, cur: &Variant) -> VResult<u64> {
        let ir = self.ir.clone();
        Ok(match t {
            Type::Byte => 1,
            Type::Boolean | Type::Integer => 2,
            Type::Long | Type::Enum(_) | Type::Single => 4,
            Type::LongLong | Type::Double | Type::Currency | Type::Date => 8,
            Type::Decimal => 16,
            Type::FixedString(n) => u64::from(*n),
            Type::String => match cur {
                Variant::String(s) => 2 + s.len() as u64,
                _ => 2,
            },
            Type::Variant => {
                2 + match cur {
                    Variant::Empty | Variant::Null => 0,
                    Variant::Byte(_) => 1,
                    Variant::Boolean(_) | Variant::Integer(_) => 2,
                    Variant::Long(_) | Variant::Single(_) => 4,
                    Variant::String(s) => 2 + s.len() as u64,
                    Variant::Decimal(_) => 16,
                    Variant::Error(_) => 4,
                    Variant::Array(_) | Variant::Object(_) | Variant::UserDefined(_) => {
                        return Err(Fault::Unsupported(
                            "Len of a user-defined type with a Variant holding an array, \
                             object or user-defined type"
                                .into(),
                        ));
                    }
                    _ => 8,
                }
            }
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
                let mut total = 0;
                for (i, f) in info.fields.iter().enumerate() {
                    let cv = vals.get(i).cloned().unwrap_or(Variant::Empty);
                    total += self.record_len(&f.ty, &cv)?;
                }
                total
            }
            Type::Array { elem, .. } => {
                let Variant::Array(a) = cur else {
                    return Err(VbaError::TYPE_MISMATCH.into());
                };
                if !a.is_fixed() {
                    return Err(Fault::Unsupported(
                        "Len of a user-defined type with a dynamic array member".into(),
                    ));
                }
                let mut total = 0;
                for e in a.iter() {
                    total += self.record_len(elem, e)?;
                }
                total
            }
            other => {
                return Err(Fault::Unsupported(format!(
                    "Len of a user-defined type with a member of type {other}"
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
            let v = vm.value_of(v)?;
            // MS-VBAL 5.4.5.1.1: a value that does not coerce to Integer is
            // error 52, not a type mismatch.
            convert::to_integer(&v)
                .map(i32::from)
                .map_err(|_| VbaError::new(52).into())
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
                    Some(o) => {
                        let v = self.val(o)?;
                        Some(self.long_of(v)?)
                    }
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
                let w = opt_i64(self, width)?.unwrap_or(0);
                // MS-VBAL 5.4.5.7: the width is 0 to 255.
                if !(0..=255).contains(&w) {
                    return Err(VbaError::INVALID_PROCEDURE_CALL.into());
                }
                svc(self.services.io(IoRequest::Width {
                    file,
                    width: w as i32,
                }))?;
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
