//! Late-bound member access (`IDispatch`-style) and the built-in objects
//! `Err`, `Debug` and `Collection`.
//!
//! Every function here that returns a value returns it *owned*: an object
//! result carries one reference the caller must hand to the statement's
//! temporaries (`Vm::own`) or store.

use vba_ast::ProcKind;
use vba_automation::call::{self, CallArgs, Final};
use vba_automation::{Arg as AutoArg, DispId, InvokeFlags};
use vba_ir::{CallArg, InvokeKind, LateArg, Member, Operand};
use vba_sema::{ModuleId, ProcId};
use vba_value::{ObjectHandle, ObjectRef, Variant, VbaError, convert};

use crate::calls::BoundArg;
use crate::error::{ErrOrigin, Fault, RtErr, VResult};
use crate::state::*;
use crate::vm::Vm;

type LateArgs = Vec<(Option<String>, BoundArg)>;

/// What the analysis resolved for an access on a library type.
#[derive(Clone, Copy)]
pub(crate) struct EarlyBind<'a> {
    /// The member id the library declares.
    pub dispid: Option<i32>,
    /// The declared scalar type of the result, which the result is
    /// converted to.
    pub result_ty: Option<&'a vba_sema::Type>,
}

impl Vm {
    fn late_args(&mut self, args: &[LateArg]) -> VResult<LateArgs> {
        let mut out = Vec::with_capacity(args.len());
        for a in args {
            let b = match &a.arg {
                CallArg::Val(op) => BoundArg::Val(self.val(op)?),
                CallArg::Ref(r) => BoundArg::Ref(self.reg_loc(*r)?),
                CallArg::Missing => BoundArg::Missing,
            };
            out.push((a.name.clone(), b));
        }
        Ok(out)
    }

    fn bound_value(&self, b: &BoundArg) -> VResult<Option<Variant>> {
        Ok(match b {
            BoundArg::Val(v) => Some(v.clone()),
            BoundArg::Ref(l) => Some(self.load(l)?),
            BoundArg::Missing => None,
        })
    }

    /// Executes an `Invoke` instruction. A call of a procedure of a class
    /// instance pushes a frame (no native recursion); everything else
    /// completes here.
    pub(crate) fn exec_invoke(
        &mut self,
        dst: Option<vba_ir::Reg>,
        receiver: &Operand,
        member: &Member,
        kind: InvokeKind,
        args: &[LateArg],
        early: EarlyBind<'_>,
    ) -> VResult<crate::exec::Flow> {
        let recv = self.invoke_receiver(receiver)?;
        if matches!(member, Member::Default)
            && kind != InvokeKind::Get
            && matches!(recv, Variant::Array(_))
            && let Operand::Reg(r) = receiver
            && let Ok(loc) = self.reg_loc(*r)
        {
            let largs = self.late_args(args)?;
            self.store_variant_array_element(&loc, &recv, largs)?;
            return Ok(crate::exec::Flow::Next);
        }
        let largs = self.late_args(args)?;
        self.push_request = Some(dst);
        self.push_done = false;
        let result =
            self.invoke_value_early(recv, member, kind, largs, dst.is_some(), early.dispid);
        self.push_request = None;
        let result = result?;
        if std::mem::take(&mut self.push_done) {
            return Ok(crate::exec::Flow::Pushed);
        }
        let result = match early.result_ty {
            Some(t) => self.convert_value(result, t)?,
            None => result,
        };
        match dst {
            Some(r) => {
                self.own(&result);
                self.set_val(r, result);
            }
            None => self.own(&result),
        }
        Ok(crate::exec::Flow::Next)
    }

    /// `v(i) = x` where the `Variant` variable at `loc` holds an array: the
    /// value is converted to the array's element type and stored in place.
    fn store_variant_array_element(
        &mut self,
        loc: &Loc,
        array: &Variant,
        mut args: LateArgs,
    ) -> VResult<()> {
        let Variant::Array(a) = array else {
            return Err(VbaError::new(13).into());
        };
        let (_, last) = args.pop().ok_or(VbaError::new(450))?;
        let v = self.bound_value(&last)?.ok_or(VbaError::new(449))?;
        let mut idx = Vec::new();
        for (_, b) in &args {
            let x = self.bound_value(b)?.ok_or(VbaError::new(449))?;
            idx.push(self.long_of(x)?);
        }
        if idx.len() != a.dimensions() {
            return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE.into());
        }
        let v = self.value_of(v)?;
        let v = match a.element_type() {
            vba_value::VarType::Variant => v,
            t => convert::convert_to(&v, t)?,
        };
        self.store(
            &Loc::Elem {
                base: Box::new(loc.clone()),
                idx,
            },
            v,
        )
    }

    fn invoke_receiver(&mut self, op: &Operand) -> VResult<Variant> {
        self.val(op)
    }

    /// Invokes a member on any receiver value, by name.
    pub(crate) fn invoke_value(
        &mut self,
        recv: Variant,
        member: &Member,
        kind: InvokeKind,
        args: LateArgs,
        want_value: bool,
    ) -> VResult<Variant> {
        self.invoke_value_early(recv, member, kind, args, want_value, None)
    }

    /// Invokes a member on any receiver value; `dispid` is the member id
    /// the library declares when the access is early-bound.
    pub(crate) fn invoke_value_early(
        &mut self,
        recv: Variant,
        member: &Member,
        kind: InvokeKind,
        args: LateArgs,
        want_value: bool,
        dispid: Option<i32>,
    ) -> VResult<Variant> {
        let push = self.push_request.take();
        match recv {
            Variant::Array(a) => match (member, kind) {
                (Member::Default, InvokeKind::Get) => {
                    let mut idx = Vec::new();
                    for (_, b) in &args {
                        let v = self.bound_value(b)?.ok_or(VbaError::new(449))?;
                        idx.push(self.long_of(v)?);
                    }
                    let v = a.get(&idx)?.clone();
                    self.retain_value(&v);
                    Ok(v)
                }
                (Member::Default, _) => Err(Fault::Unsupported(
                    "assignment to an element of an array held in a Variant by late binding".into(),
                )),
                _ => Err(VbaError::new(424).into()),
            },
            Variant::Object(ObjectRef::Handle(h)) => match vm_id(h) {
                Some(id) => self.invoke_vm_object(id, member, kind, args, push),
                None => self.invoke_external(h, member, kind, args, want_value, dispid),
            },
            Variant::Object(ObjectRef::Nothing) | Variant::Empty => Err(VbaError::new(91).into()),
            _ => Err(VbaError::new(424).into()),
        }
    }

    fn invoke_vm_object(
        &mut self,
        id: u64,
        member: &Member,
        kind: InvokeKind,
        args: LateArgs,
        push: Option<Option<vba_ir::Reg>>,
    ) -> VResult<Variant> {
        let k = match self.objects.get(&id).map(|o| &o.kind) {
            Some(ObjKind::Class { module, .. }) => Some(*module),
            Some(_) => None,
            None => return Err(VbaError::new(91).into()),
        };
        if let Some(m) = k {
            return self.invoke_class(id, m, member, kind, args, push);
        }
        match self.objects.get(&id).map(|o| &o.kind) {
            Some(ObjKind::Err) => self.invoke_err(member, kind, args),
            Some(ObjKind::Debug) => self.invoke_debug(member, kind, args),
            Some(ObjKind::Collection(_)) => self.invoke_collection(id, member, kind, args),
            _ => Err(VbaError::new(438).into()),
        }
    }

    // ---- class instances -------------------------------------------------------

    fn arrange_args(&self, pid: ProcId, args: LateArgs) -> VResult<Vec<BoundArg>> {
        let p = self.ir.program.proc(pid);
        let mut out: Vec<BoundArg> = Vec::new();
        let mut named = Vec::new();
        for (n, b) in args {
            match n {
                None => out.push(b),
                Some(n) => named.push((n, b)),
            }
        }
        for (n, b) in named {
            let Some(i) = p
                .params
                .iter()
                .position(|q| q.name.to_string().eq_ignore_ascii_case(&n))
            else {
                return Err(VbaError::new(448).into());
            };
            while out.len() <= i {
                out.push(BoundArg::Missing);
            }
            if !matches!(out[i], BoundArg::Missing) {
                return Err(VbaError::new(446).into());
            }
            out[i] = b;
        }
        Ok(out)
    }

    fn invoke_class(
        &mut self,
        id: u64,
        module: ModuleId,
        member: &Member,
        kind: InvokeKind,
        args: LateArgs,
        push: Option<Option<vba_ir::Reg>>,
    ) -> VResult<Variant> {
        let ir = self.ir.clone();
        let mi = ir.program.module(module);
        let name = match member {
            Member::Named(n) => n.clone(),
            Member::Default => mi
                .default_member
                .as_ref()
                .map(|n| n.to_string())
                .ok_or(VbaError::new(438))?,
        };
        let lname = name.to_ascii_lowercase();
        if let Some(cands) = self.procs_by_name.get(&(module, lname)) {
            let pick = cands.iter().copied().find(|p| {
                let k = ir.program.proc(*p).kind;
                match kind {
                    InvokeKind::Get => {
                        matches!(
                            k,
                            ProcKind::Sub | ProcKind::Function | ProcKind::PropertyGet
                        )
                    }
                    InvokeKind::Put => k == ProcKind::PropertyLet,
                    InvokeKind::PutRef => k == ProcKind::PropertySet,
                }
            });
            if let Some(pid) = pick {
                let p = ir.program.proc(pid);
                if p.visibility == vba_ast::Visibility::Private {
                    return Err(VbaError::new(438).into());
                }
                let bound = self.arrange_args(pid, args)?;
                if let Some(ret_dst) = push {
                    self.push_proc_frame(pid, Some(id), bound, ret_dst)?;
                    self.push_done = true;
                    return Ok(Variant::Empty);
                }
                return self.call_nested(pid, Some(id), bound);
            }
        }
        // A public variable.
        let field = ir
            .program
            .globals
            .iter()
            .find(|g| {
                g.module == module
                    && g.static_of.is_none()
                    && g.name.to_string().eq_ignore_ascii_case(&name)
            })
            .map(|g| (g.id, g.visibility));
        if let Some((g, vis)) = field {
            if vis == vba_ast::Visibility::Private {
                return Err(VbaError::new(438).into());
            }
            let loc = Loc::Field {
                obj: id,
                index: self.field_pos[g.index()],
            };
            match kind {
                InvokeKind::Get => {
                    let v = self.load(&loc)?;
                    let v = if args.is_empty() {
                        v
                    } else {
                        let Variant::Array(a) = &v else {
                            return Err(VbaError::new(13).into());
                        };
                        let mut idx = Vec::new();
                        for (_, b) in &args {
                            let x = self.bound_value(b)?.ok_or(VbaError::new(449))?;
                            idx.push(self.long_of(x)?);
                        }
                        a.get(&idx)?.clone()
                    };
                    self.retain_value(&v);
                    return Ok(v);
                }
                InvokeKind::Put | InvokeKind::PutRef => {
                    let (_, last) = args.last().ok_or(VbaError::new(450))?;
                    let v = self.bound_value(last)?.ok_or(VbaError::new(449))?;
                    let v = if kind == InvokeKind::Put {
                        self.value_of(v)?
                    } else {
                        v
                    };
                    self.store(&loc, v)?;
                    return Ok(Variant::Empty);
                }
            }
        }
        Err(VbaError::new(438).into())
    }

    // ---- default member ----------------------------------------------------------

    /// The value of an object's default member (owned).
    pub(crate) fn default_get(&mut self, h: ObjectHandle) -> VResult<Variant> {
        self.invoke_value(
            Variant::Object(ObjectRef::Handle(h)),
            &Member::Default,
            InvokeKind::Get,
            Vec::new(),
            true,
        )
    }

    // ---- Err -----------------------------------------------------------------------

    fn invoke_err(
        &mut self,
        member: &Member,
        kind: InvokeKind,
        args: LateArgs,
    ) -> VResult<Variant> {
        let name = match member {
            Member::Named(n) => n.to_ascii_lowercase(),
            Member::Default => "number".to_owned(),
        };
        let first = |vm: &Vm| -> VResult<Option<Variant>> {
            match args.last() {
                Some((_, b)) => vm.bound_value(b),
                None => Ok(None),
            }
        };
        match (kind, name.as_str()) {
            (InvokeKind::Get, "number") => Ok(Variant::Long(self.err.number)),
            (InvokeKind::Get, "description") => Ok(Variant::from_text(&self.err.description)),
            (InvokeKind::Get, "source") => Ok(Variant::from_text(&self.err.source)),
            (InvokeKind::Get, "helpfile") => Ok(Variant::from_text(&self.err.help_file)),
            (InvokeKind::Get, "helpcontext") => Ok(Variant::Long(self.err.help_context)),
            (InvokeKind::Get, "lastdllerror") => Ok(Variant::Long(self.err.last_dll_error)),
            (InvokeKind::Get, "erl") => Ok(Variant::Long(self.erl as i32)),
            (InvokeKind::Get, "clear") => {
                self.err = ErrState {
                    last_dll_error: self.err.last_dll_error,
                    ..ErrState::default()
                };
                Ok(Variant::Empty)
            }
            (InvokeKind::Get, "raise") => self.err_raise(args),
            (InvokeKind::Put, p) => {
                let v = first(self)?.ok_or(VbaError::new(450))?;
                let v = self.value_of(v)?;
                match p {
                    "number" => self.err.number = convert::to_long(&v)?,
                    "description" => {
                        self.err.description = convert::to_vba_string(&v)?.to_string_lossy()
                    }
                    "source" => self.err.source = convert::to_vba_string(&v)?.to_string_lossy(),
                    "helpfile" => {
                        self.err.help_file = convert::to_vba_string(&v)?.to_string_lossy()
                    }
                    "helpcontext" => self.err.help_context = convert::to_long(&v)?,
                    _ => return Err(VbaError::new(438).into()),
                }
                Ok(Variant::Empty)
            }
            _ => Err(VbaError::new(438).into()),
        }
    }

    fn err_raise(&mut self, args: LateArgs) -> VResult<Variant> {
        const NAMES: [&str; 5] = ["number", "source", "description", "helpfile", "helpcontext"];
        let mut slots: [Option<Variant>; 5] = Default::default();
        let mut pos = 0;
        for (n, b) in &args {
            let i = match n {
                Some(n) => NAMES
                    .iter()
                    .position(|x| x.eq_ignore_ascii_case(n))
                    .ok_or(VbaError::new(448))?,
                None => {
                    pos += 1;
                    pos - 1
                }
            };
            if i >= 5 {
                return Err(VbaError::new(450).into());
            }
            slots[i] = self.bound_value(b)?;
        }
        let number = match &slots[0] {
            Some(v) => {
                let v = self.value_of(v.clone())?;
                convert::to_long(&v)?
            }
            None => return Err(VbaError::new(449).into()),
        };
        if number == 0 {
            return Err(VbaError::INVALID_PROCEDURE_CALL.into());
        }
        let text = |vm: &mut Vm, v: &Option<Variant>| -> VResult<Option<String>> {
            match v {
                Some(Variant::Empty) | None => Ok(None),
                Some(v) => {
                    let v = vm.value_of(v.clone())?;
                    Ok(Some(convert::to_vba_string(&v)?.to_string_lossy()))
                }
            }
        };
        let source = text(self, &slots[1])?;
        let description = text(self, &slots[2])?;
        let help_file = text(self, &slots[3])?;
        let help_context = match &slots[4] {
            Some(v) if !matches!(v, Variant::Empty) => convert::to_long(v)?,
            _ => 0,
        };
        Err(Fault::Err(RtErr {
            number,
            description: description.unwrap_or_else(|| crate::error::describe(number)),
            source: source.unwrap_or_default(),
            help_file: help_file.unwrap_or_default(),
            help_context,
            origin: ErrOrigin::Vba,
        }))
    }

    // ---- Debug ---------------------------------------------------------------------

    fn invoke_debug(
        &mut self,
        member: &Member,
        kind: InvokeKind,
        args: LateArgs,
    ) -> VResult<Variant> {
        let Member::Named(n) = member else {
            return Err(VbaError::new(438).into());
        };
        if kind != InvokeKind::Get {
            return Err(VbaError::new(438).into());
        }
        match n.to_ascii_lowercase().as_str() {
            "assert" => {
                let (_, b) = args.first().ok_or(VbaError::new(449))?;
                let v = self.bound_value(b)?.ok_or(VbaError::new(449))?;
                let v = self.value_of(v)?;
                if !convert::to_boolean(&v)? {
                    self.stop_reason = Some("Debug.Assert failed".into());
                    return Err(Fault::Stop);
                }
                Ok(Variant::Empty)
            }
            "print" => {
                self.debug_write_line("");
                Ok(Variant::Empty)
            }
            _ => Err(VbaError::new(438).into()),
        }
    }

    // ---- Collection ------------------------------------------------------------------

    fn coll_items(&mut self, id: u64) -> VResult<&mut Vec<CollItem>> {
        match self.objects.get_mut(&id).map(|o| &mut o.kind) {
            Some(ObjKind::Collection(items)) => Ok(items),
            _ => Err(VbaError::new(91).into()),
        }
    }

    /// Resolves an `Index` argument (position or key) to a zero-based
    /// position.
    fn coll_position(&mut self, id: u64, index: &Variant) -> VResult<usize> {
        let index = self.value_of(index.clone())?;
        let items = self.coll_items(id)?;
        match &index {
            Variant::String(s) => {
                let key = s.to_string_lossy().to_lowercase();
                items
                    .iter()
                    .position(|i| i.key.as_deref() == Some(key.as_str()))
                    .ok_or_else(|| VbaError::INVALID_PROCEDURE_CALL.into())
            }
            other => {
                let n = convert::to_long(other)?;
                if n >= 1 && (n as usize) <= items.len() {
                    Ok(n as usize - 1)
                } else {
                    Err(VbaError::SUBSCRIPT_OUT_OF_RANGE.into())
                }
            }
        }
    }

    fn invoke_collection(
        &mut self,
        id: u64,
        member: &Member,
        kind: InvokeKind,
        args: LateArgs,
    ) -> VResult<Variant> {
        let name = match member {
            Member::Named(n) => n.to_ascii_lowercase(),
            Member::Default => "item".to_owned(),
        };
        if kind != InvokeKind::Get {
            return Err(VbaError::new(438).into());
        }
        const ADD: [&str; 4] = ["item", "key", "before", "after"];
        match name.as_str() {
            "count" => Ok(Variant::Long(self.coll_items(id)?.len() as i32)),
            "item" | "remove" => {
                let (_, b) = args.first().ok_or(VbaError::new(449))?;
                let idx = self.bound_value(b)?.ok_or(VbaError::new(449))?;
                let pos = self.coll_position(id, &idx)?;
                if name == "item" {
                    let v = self.coll_items(id)?[pos].value.clone();
                    self.retain_value(&v);
                    Ok(v)
                } else {
                    let it = self.coll_items(id)?.remove(pos);
                    self.release_value(it.value)?;
                    Ok(Variant::Empty)
                }
            }
            "add" => {
                let mut slots: [Option<Variant>; 4] = Default::default();
                let mut pos = 0;
                for (n, b) in &args {
                    let i = match n {
                        Some(n) => ADD
                            .iter()
                            .position(|x| x.eq_ignore_ascii_case(n))
                            .ok_or(VbaError::new(448))?,
                        None => {
                            pos += 1;
                            pos - 1
                        }
                    };
                    if i >= 4 {
                        return Err(VbaError::new(450).into());
                    }
                    slots[i] = self.bound_value(b)?;
                }
                let item = slots[0].take().ok_or(VbaError::new(449))?;
                let key = match slots[1].take() {
                    Some(Variant::Empty) | None => None,
                    Some(k) => {
                        let k = self.value_of(k)?;
                        Some(convert::to_vba_string(&k)?.to_string_lossy().to_lowercase())
                    }
                };
                if let Some(k) = &key
                    && self
                        .coll_items(id)?
                        .iter()
                        .any(|i| i.key.as_deref() == Some(k.as_str()))
                {
                    return Err(VbaError::new(457).into());
                }
                let before = slots[2].take().filter(|v| !matches!(v, Variant::Empty));
                let after = slots[3].take().filter(|v| !matches!(v, Variant::Empty));
                let at = match (before, after) {
                    (Some(_), Some(_)) => return Err(VbaError::INVALID_PROCEDURE_CALL.into()),
                    (Some(b), None) => self.coll_position(id, &b)?,
                    (None, Some(a)) => self.coll_position(id, &a)? + 1,
                    (None, None) => self.coll_items(id)?.len(),
                };
                let value = self.adopt(item)?;
                self.charge_memory(64)?;
                self.coll_items(id)?.insert(at, CollItem { key, value });
                Ok(Variant::Empty)
            }
            _ => Err(VbaError::new(438).into()),
        }
    }

    // ---- objects outside the VM ---------------------------------------------------------

    fn invoke_external(
        &mut self,
        h: ObjectHandle,
        member: &Member,
        kind: InvokeKind,
        args: LateArgs,
        want_value: bool,
        dispid: Option<i32>,
    ) -> VResult<Variant> {
        let host = self.host.clone();
        let table = host.objects();
        let mut args = args;
        let put_value = match kind {
            InvokeKind::Get => None,
            _ => {
                let (_, last) = args.pop().ok_or(VbaError::new(450))?;
                Some(self.bound_value(&last)?.ok_or(VbaError::new(449))?)
            }
        };
        let mut ca = CallArgs::none();
        let mut positional_locs: Vec<Option<Loc>> = Vec::new();
        let mut named_locs: Vec<Option<Loc>> = Vec::new();
        for (n, b) in &args {
            let (arg, loc) = match b {
                BoundArg::Val(v) => (AutoArg::val(v.clone()), None),
                BoundArg::Ref(l) => (AutoArg::by_ref(self.load(l)?), Some(l.clone())),
                BoundArg::Missing => (AutoArg::missing(), None),
            };
            match n {
                None => {
                    ca.positional.push(arg);
                    positional_locs.push(loc);
                }
                Some(n) => {
                    ca.named.push((n.clone(), arg));
                    named_locs.push(loc);
                }
            }
        }
        let fin = match (kind, put_value) {
            (InvokeKind::Get, _) => {
                if want_value {
                    Final::Get
                } else {
                    Final::Call
                }
            }
            (InvokeKind::Put, Some(v)) => Final::Let(v),
            (InvokeKind::PutRef, Some(v)) => Final::Set(v),
            _ => return Err(VbaError::new(450).into()),
        };
        let result = match member {
            Member::Named(n) => {
                let early = dispid
                    .and_then(|id| call::invoke_early(table, h, DispId(id), n, &mut ca, &fin));
                match early {
                    Some(r) => {
                        self.bind_stats.early += 1;
                        r
                    }
                    None => {
                        self.bind_stats.by_name += 1;
                        self.bind_stats.fallbacks += u64::from(dispid.is_some());
                        call::invoke_member(table, h, n, &mut ca, &fin)
                    }
                }
            }
            Member::Default => {
                self.bind_stats.by_name += 1;
                call::invoke_default(table, h, &mut ca, &fin)
            }
        };
        for (arg, loc) in ca.positional.iter().zip(&positional_locs) {
            if let (true, Some(l)) = (arg.by_ref, loc) {
                self.store(l, arg.value.clone())?;
            }
        }
        for ((_, arg), loc) in ca.named.iter().zip(&named_locs) {
            if let (true, Some(l)) = (arg.by_ref, loc) {
                self.store(l, arg.value.clone())?;
            }
        }
        let applied = self.apply_foreign_refs();
        let v = result.map_err(|e| {
            let mut r = RtErr::from(e);
            if r.origin == ErrOrigin::Vba {
                r.origin = ErrOrigin::Host;
            }
            Fault::Err(r)
        })?;
        applied?;
        self.drain_events()?;
        Ok(v)
    }

    /// Starts enumerating an external object with `DISPID_NEWENUM`.
    pub(crate) fn external_enumerator(&mut self, h: ObjectHandle) -> VResult<ObjectHandle> {
        let host = self.host.clone();
        let table = host.objects();
        let obj = table.get(h)?;
        let mut dp = vba_automation::DispParams::new();
        let r = obj
            .invoke(
                table,
                DispId::NEWENUM,
                InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
                &mut dp,
            )
            .map_err(|e| Fault::Err(RtErr::from(e.into_err_info(&obj.class_name()))))?;
        match r {
            Variant::Object(ObjectRef::Handle(eh)) => {
                let eo = table.get(eh)?;
                if eo.enumerator().is_none() {
                    let _ = table.release(eh);
                    return Err(VbaError::new(438).into());
                }
                Ok(eh)
            }
            other => {
                table.release_value(&other);
                Err(VbaError::new(438).into())
            }
        }
    }

    /// Advances an enumerator object outside the VM.
    pub(crate) fn external_next(&mut self, enumerator: ObjectHandle) -> VResult<Option<Variant>> {
        let host = self.host.clone();
        let table = host.objects();
        let eo = table.get(enumerator)?;
        let en = eo.enumerator().ok_or(VbaError::new(438))?;
        let r = en
            .next(table)
            .map_err(|e| Fault::Err(RtErr::from(e.into_err_info(&eo.class_name()))));
        self.apply_foreign_refs()?;
        r
    }

    /// The `TypeName` of an object.
    pub(crate) fn object_type_name(&self, h: ObjectHandle) -> String {
        match vm_id(h) {
            Some(id) => match self.objects.get(&id).map(|o| &o.kind) {
                Some(ObjKind::Class { module, .. }) => {
                    self.ir.program.module(*module).name.to_string()
                }
                Some(ObjKind::Collection(_)) => "Collection".to_owned(),
                Some(ObjKind::Err) => "ErrObject".to_owned(),
                Some(ObjKind::Debug) => "DebugObject".to_owned(),
                None => "Nothing".to_owned(),
            },
            None => self
                .host
                .objects()
                .get(h)
                .map(|o| o.class_name())
                .unwrap_or_else(|_| "Object".to_owned()),
        }
    }
}
