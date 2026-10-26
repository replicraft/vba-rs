//! The interpreter loop: instructions, terminators and error dispatch.

use vba_ir::{
    BinOp, Block, CallArg, Callee, ErrMode, Function, Inst, InstKind, IrProgram, Operand, Point,
    RefPlace, ResumeKind, StmtId, StoreMode, Terminator, UnOp,
};
use vba_sema::{ModuleId, Type};
use vba_value::ops::{self, CompareOp, CompareOptions, Overflow};
use vba_value::{
    Bounds, CompareMode, ObjectRef, VarType, Variant, VbaArray, VbaError, VbaString, convert,
};

use crate::calls::BoundArg;
use crate::error::{Fault, RtErr, StackFrame, VResult};
use crate::state::*;
use crate::vm::Vm;

/// What the loop does after an instruction.
pub(crate) enum Flow {
    /// Continue with the next instruction of the frame.
    Next,
    /// The program counter was set (jump, call return).
    Jumped,
    /// A frame was pushed.
    Pushed,
    /// The base frame of this run returned.
    Return(Variant),
}

impl Vm {
    /// Runs until the frame stack shrinks to `base`; returns the result of
    /// the last frame popped.
    pub(crate) fn run(&mut self, base: usize) -> VResult<Variant> {
        let ir = self.ir.clone();
        // The address of a local is where the stack is now (it grows toward
        // lower addresses on every supported target, and only the distance
        // matters).
        let marker = 0u8;
        let here = std::ptr::addr_of!(marker) as usize;
        if self.run_depth == 0 {
            self.native_base = here;
        }
        if self.run_depth >= self.cfg.max_native_depth
            || self.native_base.abs_diff(here) > self.cfg.max_native_stack_bytes
        {
            return Err(Fault::Limit(crate::error::LimitKind::CallDepth));
        }
        self.run_depth += 1;
        let r = self.run_loop(&ir, base);
        self.run_depth -= 1;
        r
    }

    fn run_loop(&mut self, ir: &IrProgram, base: usize) -> VResult<Variant> {
        loop {
            if self.udts.len() > self.gc_threshold && self.run_depth == 1 {
                self.collect_udts();
            }
            let fi = self.frames.len() - 1;
            let (fidx, bi, ii) = {
                let f = &self.frames[fi];
                (f.func, f.block as usize, f.idx as usize)
            };
            let func = &ir.functions[fidx];
            let block = &func.blocks[bi];
            let r = if ii < block.insts.len() {
                self.exec_inst(ir, func, &block.insts[ii])
            } else {
                self.exec_term(ir, func, block, base)
            };
            match r {
                Ok(Flow::Next) => self.frames[fi].idx += 1,
                Ok(Flow::Jumped) | Ok(Flow::Pushed) => {}
                Ok(Flow::Return(v)) => return Ok(v),
                Err(Fault::Err(e)) => self.dispatch_error(ir, e, base)?,
                Err(f) => return Err(f),
            }
        }
    }

    // ---- errors -------------------------------------------------------------

    fn record_err(&mut self, e: &RtErr) {
        self.err.number = e.number;
        self.err.description = e.description.clone();
        self.err.source = if e.source.is_empty() {
            self.cfg.project_name.clone()
        } else {
            e.source.clone()
        };
        self.err.help_file = e.help_file.clone();
        self.err.help_context = e.help_context;
    }

    /// The first instruction of the run of the statement containing the
    /// instruction at `idx`; `Resume` re-executes from there, so operands
    /// are evaluated again as in VBA.
    fn stmt_start(func: &Function, block: u32, idx: u32) -> u32 {
        let b = &func.blocks[block as usize];
        let stmt = Self::stmt_at(func, block, idx);
        let mut i = (idx as usize).min(b.insts.len());
        while i > 0 && b.insts[i - 1].stmt == stmt {
            i -= 1;
        }
        i as u32
    }

    fn stmt_at(func: &Function, block: u32, idx: u32) -> StmtId {
        let b = &func.blocks[block as usize];
        match b.insts.get(idx as usize) {
            Some(i) => i.stmt,
            None => b.term_stmt,
        }
    }

    /// Where the error being dispatched was raised: the call stack from the
    /// innermost frame out.
    fn capture_site(&self, ir: &IrProgram) -> Site {
        let frame_of = |f: &Frame| -> StackFrame {
            let func = &ir.functions[f.func];
            let (module, procedure) = match func.name.split_once('.') {
                Some((m, p)) => (m.to_owned(), p.to_owned()),
                None => (String::new(), func.name.clone()),
            };
            let stmt = Self::stmt_at(func, f.block, f.idx);
            let source_line = self
                .source_lines
                .get(f.func)
                .and_then(|l| l.get(stmt.0 as usize))
                .copied()
                .unwrap_or(0);
            StackFrame {
                module,
                procedure,
                line: f.line,
                source_line,
            }
        };
        let frames: Vec<StackFrame> = self.frames.iter().rev().map(frame_of).collect();
        let top = frames.first().cloned().unwrap_or_default();
        let name_of = |f: &StackFrame| format!("{}.{}", f.module, f.procedure);
        Site {
            module: top.module.clone(),
            procedure: name_of(&top),
            line: top.line,
            source_line: top.source_line,
            stack: frames.iter().map(name_of).collect(),
            frames,
        }
    }

    /// Routes a trappable error to the innermost active handler, unwinding
    /// frames above `base`. `Ok` means execution continues in a handler or
    /// after the failing statement; `Err` means it reached `base`.
    fn dispatch_error(&mut self, ir: &IrProgram, e: RtErr, base: usize) -> VResult<()> {
        self.record_err(&e);
        loop {
            if self.frames.len() <= base {
                return Err(Fault::Err(e));
            }
            if self.last_site.is_none() {
                self.last_site = Some(self.capture_site(ir));
            }
            let fi = self.frames.len() - 1;
            let (mode, in_handler, func_idx, block, idx) = {
                let f = &self.frames[fi];
                (f.mode, f.in_handler, f.func, f.block, f.idx)
            };
            let func = &ir.functions[func_idx];
            let stmt = Self::stmt_at(func, block, idx);
            match mode {
                ErrMode::Handler(b) if !in_handler => {
                    let start = Self::stmt_start(func, block, idx);
                    let f = &mut self.frames[fi];
                    f.fail = Some((block, start, stmt));
                    f.in_handler = true;
                    f.block = b.0;
                    f.idx = 0;
                    self.erl = f.line;
                    self.err.site = self.last_site.take();
                    return Ok(());
                }
                ErrMode::ResumeNext => {
                    if let Some(p) = func.stmts[stmt.0 as usize].resume_next {
                        let start = Self::stmt_start(func, block, idx);
                        let f = &mut self.frames[fi];
                        f.fail = Some((block, start, stmt));
                        f.block = p.block.0;
                        f.idx = p.index;
                        self.erl = f.line;
                        self.err.site = self.last_site.take();
                        return Ok(());
                    }
                }
                _ => {}
            }
            match self.pop_frame() {
                Ok(_) | Err(Fault::Err(_)) => {}
                Err(other) => return Err(other),
            }
        }
    }

    // ---- operands -------------------------------------------------------------

    /// Resolves an object used as a value to its default member's value.
    pub(crate) fn value_of(&mut self, v: Variant) -> VResult<Variant> {
        let mut cur = v;
        for _ in 0..32 {
            match cur {
                Variant::Object(ObjectRef::Nothing) => return Err(VbaError::new(91).into()),
                Variant::Object(ObjectRef::Handle(h)) => {
                    let r = self.default_get(h)?;
                    self.own(&r);
                    cur = r;
                }
                other => return Ok(other),
            }
        }
        Err(VbaError::new(28).into())
    }

    fn truthy(&mut self, v: Variant) -> VResult<bool> {
        match v {
            Variant::Boolean(b) => Ok(b),
            Variant::Null => Ok(false),
            other => {
                let other = self.value_of(other)?;
                Ok(convert::to_boolean(&other)?)
            }
        }
    }

    pub(crate) fn long_of(&mut self, v: Variant) -> VResult<i32> {
        let v = self.value_of(v)?;
        Ok(convert::to_long(&v)?)
    }

    fn indices(&mut self, ops_: &[Operand]) -> VResult<Vec<i32>> {
        let mut out = Vec::with_capacity(ops_.len());
        for o in ops_ {
            let v = self.val(o)?;
            out.push(self.long_of(v)?);
        }
        Ok(out)
    }

    /// Converts a value to a declared type.
    pub(crate) fn convert_value(&mut self, v: Variant, to: &Type) -> VResult<Variant> {
        match to {
            Type::Variant | Type::Any | Type::Error => Ok(v),
            Type::Object | Type::Nothing | Type::Library(_) | Type::Class(_) => match &v {
                Variant::Object(ObjectRef::Nothing) => Ok(v),
                Variant::Object(ObjectRef::Handle(h)) => {
                    if let Type::Class(m) = to {
                        match vm_id(*h) {
                            Some(id) if self.instance_of(id, *m) => Ok(v),
                            _ => Err(VbaError::TYPE_MISMATCH.into()),
                        }
                    } else {
                        Ok(v)
                    }
                }
                Variant::Empty => Err(VbaError::new(91).into()),
                Variant::Null => Err(VbaError::INVALID_USE_OF_NULL.into()),
                _ => Err(VbaError::TYPE_MISMATCH.into()),
            },
            Type::Udt(_) | Type::LibRecord(_) => match v {
                Variant::UserDefined(_) => Ok(v),
                _ => Err(VbaError::TYPE_MISMATCH.into()),
            },
            Type::Array { elem, .. } => match v {
                Variant::Array(_) => Ok(v),
                // `b = s`: the UTF-16LE bytes of the string.
                Variant::String(s) if **elem == Type::Byte => Ok(Variant::Array(Box::new(
                    VbaArray::from_bytes(&s.to_bytes_le())?,
                ))),
                _ => Err(VbaError::TYPE_MISMATCH.into()),
            },
            Type::FixedString(n) => {
                let v = self.value_of(v)?;
                let s = convert::to_vba_string(&v)?;
                let mut units = s.into_units();
                units.resize(*n as usize, u16::from(b' '));
                Ok(Variant::String(VbaString::from_units(units)))
            }
            Type::String if matches!(&v, Variant::Array(a) if a.element_type() == VarType::Byte) => {
                // `s = b`: the string whose UTF-16LE bytes are the array's.
                let Variant::Array(a) = &v else {
                    return Err(VbaError::TYPE_MISMATCH.into());
                };
                let bytes = a.to_bytes().ok_or(VbaError::TYPE_MISMATCH)?;
                match VbaString::from_bytes_le(&bytes) {
                    Some(s) => Ok(Variant::String(s)),
                    None => Err(Fault::Unsupported(
                        "a Byte array of odd length converted to a String (the trailing \
                         half code unit has no representation)"
                            .into(),
                    )),
                }
            }
            t => {
                let v = self.value_of(v)?;
                let vt = t.var_type().unwrap_or(VarType::Variant);
                if v.vartype() == vt && !matches!(v, Variant::Array(_)) {
                    return Ok(v);
                }
                Ok(convert::convert_to(&v, vt)?)
            }
        }
    }

    /// Coerces an operator result to the static result type when it differs.
    fn fit(&mut self, out: Variant, ty: &Type) -> VResult<Variant> {
        match ty {
            Type::Variant | Type::Any | Type::Error => Ok(out),
            t => match t.var_type() {
                Some(vt)
                    if out.vartype() != vt
                        && !matches!(out, Variant::Null | Variant::Array(_))
                        && !matches!(vt, VarType::Object | VarType::UserDefinedType) =>
                {
                    Ok(convert::convert_to(&out, vt)?)
                }
                _ => Ok(out),
            },
        }
    }

    fn binary(
        &mut self,
        op: &BinOp,
        ty: &Type,
        overflow: Overflow,
        l: Variant,
        r: Variant,
    ) -> VResult<Variant> {
        if let BinOp::Is = op {
            return match (&l, &r) {
                (Variant::Object(a), Variant::Object(b)) => Ok(Variant::Boolean(a == b)),
                _ => Err(VbaError::new(424).into()),
            };
        }
        let l = self.value_of(l)?;
        let r = self.value_of(r)?;
        let out = match op {
            BinOp::Arith(a) => ops::arith(*a, &l, &r, overflow)?,
            BinOp::Concat => {
                let v = ops::concat(&l, &r)?;
                self.check_string(&v)?;
                v
            }
            BinOp::Compare(c, opts) => ops::compare(*c, &l, &r, *opts)?,
            BinOp::Logic(o) => ops::logic(*o, &l, &r)?,
            BinOp::Like(mode) => {
                if l.is_null() || r.is_null() {
                    Variant::Null
                } else {
                    let text = convert::to_vba_string(&l)?;
                    let pat = convert::to_vba_string(&r)?;
                    let m = match mode {
                        vba_ast::CompareMode::Binary => CompareMode::Binary,
                        _ => CompareMode::Text,
                    };
                    let meter = crate::intrinsics::BudgetWork(&*self.host);
                    let b = vba_stdlib::like::like_metered(&text, &pat, m, &mut |units| {
                        vba_stdlib::WorkMeter::charge(&meter, units)
                    })?;
                    Variant::Boolean(b)
                }
            }
            BinOp::Is => unreachable!("handled above"),
        };
        self.fit(out, ty)
    }

    // ---- instructions -----------------------------------------------------------

    pub(crate) fn me_of_frame(&self) -> Option<u64> {
        self.frames.last().and_then(|f| f.me)
    }

    /// The location of a module variable, from the current frame.
    fn global_loc(&self, g: vba_sema::GlobalId) -> VResult<Loc> {
        let m = self.ir.program.global(g).module;
        if self.ir.program.module(m).is_class_like() {
            let me = self.me_of_frame().ok_or(VbaError::new(91))?;
            let pos = self.field_pos[g.index()];
            if pos == u32::MAX {
                return Err(Fault::Internal("variable is not a field".into()));
            }
            Ok(Loc::Field {
                obj: me,
                index: pos,
            })
        } else {
            Ok(Loc::Global(g.0))
        }
    }

    fn field_loc(&self, obj: &Operand, g: vba_sema::GlobalId) -> VResult<Loc> {
        match self.val(obj)? {
            Variant::Object(ObjectRef::Handle(h)) => match vm_id(h) {
                Some(id) => {
                    let pos = self.field_pos[g.index()];
                    if pos == u32::MAX {
                        return Err(VbaError::new(438).into());
                    }
                    Ok(Loc::Field {
                        obj: id,
                        index: pos,
                    })
                }
                None => Err(VbaError::new(438).into()),
            },
            Variant::Object(ObjectRef::Nothing) | Variant::Empty => Err(VbaError::new(91).into()),
            _ => Err(VbaError::new(424).into()),
        }
    }

    fn make_ref(&mut self, place: &RefPlace) -> VResult<Loc> {
        Ok(match place {
            RefPlace::Local(l) => {
                let fi = self.frames.len() - 1;
                match &self.frames[fi].slots[l.index()] {
                    Slot::Bound(inner) => inner.clone(),
                    Slot::Val(_) => Loc::Local {
                        frame: fi,
                        slot: l.0,
                    },
                }
            }
            RefPlace::Global(g) => self.global_loc(*g)?,
            RefPlace::Field { obj, global } => self.field_loc(obj, *global)?,
            RefPlace::Elem { base, indices } => {
                let b = self.reg_loc(*base)?;
                let idx = self.indices(indices)?;
                Loc::Elem {
                    base: Box::new(b),
                    idx,
                }
            }
            RefPlace::UdtField { base, index, .. } => {
                let b = self.reg_loc(*base)?;
                Loc::UdtField {
                    base: Box::new(b),
                    index: *index,
                }
            }
        })
    }

    /// Stores through a location honouring the store mode.
    fn store_mode(&mut self, loc: &Loc, v: Variant, mode: StoreMode) -> VResult<()> {
        match mode {
            StoreMode::Let | StoreMode::Set => self.store(loc, v),
            StoreMode::SetWithEvents => {
                let old = self.load(loc)?;
                self.store(loc, v.clone())?;
                // The sink is the object that owns the `WithEvents` field,
                // whether the assignment runs inside it or from outside.
                if let Loc::Field { index, obj } = self.resolve_loc(loc) {
                    let ir = self.ir.clone();
                    if let Some(Obj {
                        kind: ObjKind::Class { module, .. },
                        ..
                    }) = self.objects.get(&obj)
                        && let Some(info) = ir.classes[module.index()].as_ref()
                        && let Some(g) = info.fields.get(index as usize)
                    {
                        self.rewire_events(obj, *g, &old, &v)?;
                    }
                }
                Ok(())
            }
            StoreMode::LSet | StoreMode::RSet => {
                let cur = self.load(loc)?;
                match (&cur, &v) {
                    (Variant::String(c), Variant::String(n)) => {
                        let len = c.len();
                        let mut units = n.as_units().to_vec();
                        if mode == StoreMode::LSet {
                            units.resize(len, u16::from(b' '));
                        } else if units.len() < len {
                            let mut pad = vec![u16::from(b' '); len - units.len()];
                            pad.extend(units);
                            units = pad;
                        } else {
                            units.truncate(len);
                        }
                        self.store(loc, Variant::String(VbaString::from_units(units)))
                    }
                    _ => Err(Fault::Unsupported(
                        "LSet/RSet between user-defined types".into(),
                    )),
                }
            }
        }
    }

    fn collect_args(&mut self, args: &[CallArg]) -> VResult<Vec<BoundArg>> {
        let mut out = Vec::with_capacity(args.len());
        for a in args {
            out.push(match a {
                CallArg::Val(op) => BoundArg::Val(self.val(op)?),
                CallArg::Ref(r) => BoundArg::Ref(self.reg_loc(*r)?),
                CallArg::Missing => BoundArg::Missing,
            });
        }
        Ok(out)
    }

    /// Every `POLL_INTERVAL` steps asks the host whether the run was
    /// cancelled, so loops and calls can be stopped from another thread
    /// without `DoEvents`. Counted in steps and not in instructions: an
    /// instruction that builds a large value is charged many steps at once
    /// and must not postpone the next poll by that many instructions. Costs
    /// one read of the step counter per instruction.
    #[inline]
    fn poll(&mut self) -> VResult<()> {
        let steps = self.host.budget().steps_used();
        // A counter that went back (a fresh allowance) starts a new interval.
        if steps < self.polled_at || steps - self.polled_at >= u64::from(vba_host::POLL_INTERVAL) {
            self.polled_at = steps;
            self.host
                .poll_cancel()
                .map_err(|e| Fault::Cancelled(e.description))?;
        }
        Ok(())
    }

    pub(crate) fn exec_inst(
        &mut self,
        ir: &IrProgram,
        func: &Function,
        inst: &Inst,
    ) -> VResult<Flow> {
        self.host.budget().charge_step().map_err(Fault::from)?;
        self.poll()?;
        if self.frames.last().expect("frame").cur_stmt != inst.stmt.0 {
            self.frames.last_mut().expect("frame").cur_stmt = inst.stmt.0;
            if !self.frames.last().expect("frame").owned.is_empty() {
                self.release_owned()?;
            }
        }
        match &inst.kind {
            InstKind::Copy { dst, src } => {
                let v = self.val(src)?;
                self.set_val(*dst, v);
            }
            InstKind::Unary {
                dst,
                op,
                ty,
                overflow,
                src,
            } => {
                let v = self.val(src)?;
                let v = self.value_of(v)?;
                let out = match op {
                    UnOp::Neg => ops::negate(&v, *overflow)?,
                    UnOp::Plus => match v {
                        Variant::String(_) => convert::convert_to(&v, VarType::Double)?,
                        other => other,
                    },
                    UnOp::Not => ops::not(&v)?,
                };
                let out = self.fit(out, ty)?;
                self.set_val(*dst, out);
            }
            InstKind::Binary {
                dst,
                op,
                ty,
                overflow,
                lhs,
                rhs,
                guard,
            } => {
                if let Some(g) = guard
                    && matches!(
                        self.frames.last().expect("frame").regs[g.0 as usize],
                        RegVal::Empty
                    )
                {
                    // `Next` without the `For` initialisation (MS-VBAL 5.4.2.3).
                    return Err(VbaError::new(92).into());
                }
                let l = self.val(lhs)?;
                let r = self.val(rhs)?;
                let out = self.binary(op, ty, *overflow, l, r)?;
                self.set_val(*dst, out);
            }
            InstKind::Convert { dst, src, to } => {
                let v = self.val(src)?;
                let out = self.convert_value(v, to)?;
                self.set_val(*dst, out);
            }
            InstKind::LoadLocal { dst, slot } => {
                let fi = self.frames.len() - 1;
                let rv = match &self.frames[fi].slots[slot.index()] {
                    Slot::Val(Variant::Array(_)) => RegVal::Alias(Loc::Local {
                        frame: fi,
                        slot: slot.0,
                    }),
                    Slot::Val(v) => RegVal::V(v.clone()),
                    Slot::Bound(l) => {
                        let l = l.clone();
                        match self.peek(&l)? {
                            std::borrow::Cow::Borrowed(Variant::Array(_)) => RegVal::Alias(l),
                            other => RegVal::V(other.into_owned()),
                        }
                    }
                };
                self.set_reg(*dst, rv);
            }
            InstKind::StoreLocal { slot, src, mode } => {
                let v = self.val(src)?;
                let fi = self.frames.len() - 1;
                let loc = Loc::Local {
                    frame: fi,
                    slot: slot.0,
                };
                self.store_mode(&loc, v, *mode)?;
            }
            InstKind::LoadGlobal { dst, global } => {
                let loc = self.global_loc(*global)?;
                self.load_into(*dst, &loc)?;
            }
            InstKind::StoreGlobal { global, src, mode } => {
                let v = self.val(src)?;
                let loc = self.global_loc(*global)?;
                self.store_mode(&loc, v, *mode)?;
            }
            InstKind::LoadField { dst, obj, global } => {
                let loc = self.field_loc(obj, *global)?;
                self.load_into(*dst, &loc)?;
            }
            InstKind::StoreField {
                obj,
                global,
                src,
                mode,
            } => {
                let v = self.val(src)?;
                let loc = self.field_loc(obj, *global)?;
                self.store_mode(&loc, v, *mode)?;
            }
            InstKind::LoadMe { dst } => {
                let me = self.me_of_frame().ok_or(VbaError::new(91))?;
                self.set_val(*dst, Variant::Object(ObjectRef::Handle(vm_handle(me))));
            }
            InstKind::LoadDefaultInstance { dst, module } => {
                let id = self.default_instance(*module)?;
                self.set_val(*dst, Variant::Object(ObjectRef::Handle(vm_handle(id))));
            }
            InstKind::LoadLibGlobal { dst, name } => {
                let v = self.lib_global(name)?;
                self.set_val(*dst, v);
            }
            InstKind::MakeRef { dst, place } => {
                let loc = self.make_ref(place)?;
                self.set_reg(*dst, RegVal::Ref(loc));
            }
            InstKind::LoadRef { dst, r } => {
                let loc = self.reg_loc(*r)?;
                self.load_into(*dst, &loc)?;
            }
            InstKind::StoreRef { r, src, mode } => {
                let v = self.val(src)?;
                let loc = self.reg_loc(*r)?;
                self.store_mode(&loc, v, *mode)?;
            }
            InstKind::Index {
                dst,
                array,
                indices,
            } => {
                let idx = self.indices(indices)?;
                let v = self.index_operand(array, &idx)?;
                self.set_val(*dst, v);
            }
            InstKind::UdtField {
                dst, base, index, ..
            } => {
                let b = self.val(base)?;
                let Variant::UserDefined(h) = b else {
                    return Err(VbaError::TYPE_MISMATCH.into());
                };
                let v = self
                    .udts
                    .get(&h.0)
                    .and_then(|u| u.fields.get(*index as usize))
                    .cloned()
                    .ok_or_else(|| Fault::Internal("dangling user-defined type".into()))?;
                self.set_val(*dst, v);
            }
            InstKind::Call {
                dst,
                callee,
                receiver,
                args,
            } => return self.exec_call(ir, *dst, callee, receiver.as_ref(), args),
            InstKind::Invoke {
                dst,
                receiver,
                member,
                kind,
                args,
                dispid,
                ..
            } => {
                // An early-bound result has the declared type whatever the
                // object returns.
                let result_ty = dst
                    .filter(|_| dispid.is_some())
                    .map(|d| &func.regs[d.0 as usize])
                    .filter(|t| {
                        t.var_type().is_some()
                            && !matches!(t, Type::Variant | Type::Any | Type::Error)
                            && !t.is_object()
                            && !t.is_record()
                    });
                let early = crate::invoke::EarlyBind {
                    dispid: *dispid,
                    result_ty,
                };
                return self.exec_invoke(*dst, receiver, member, *kind, args, early);
            }
            InstKind::New { dst, target } => {
                let v = self.create_new(target)?;
                self.own(&v);
                self.set_val(*dst, v);
            }
            InstKind::EnsureNew { r, target } => {
                let loc = self.reg_loc(*r)?;
                if matches!(self.load(&loc)?, Variant::Object(ObjectRef::Nothing)) {
                    let v = self.create_new(target)?;
                    self.own(&v);
                    self.store(&loc, v)?;
                }
            }
            InstKind::AddressOf { .. } => {
                return Err(Fault::Unsupported(
                    "AddressOf (callbacks into VBA from native code)".into(),
                ));
            }
            InstKind::TypeOfIs { dst, src, ty } => {
                let v = self.val(src)?;
                let b = self.type_of_is(&v, ty);
                self.set_val(*dst, Variant::Boolean(b));
            }
            InstKind::ReDim {
                r,
                preserve,
                bounds,
                elem,
            } => self.exec_redim(*r, *preserve, bounds, elem)?,
            InstKind::DimFixed { r, bounds, elem } => {
                let loc = self.reg_loc(*r)?;
                let a = self.make_fixed_array(elem, bounds)?;
                let slot = self.var_mut(&loc)?;
                *slot = Variant::Array(Box::new(a));
            }
            InstKind::Erase { r } => self.exec_erase(func, *r)?,
            InstKind::Undefined { .. } => {}
            InstKind::IterInit { dst, src } => {
                let v = self.val(src)?;
                let it = self.iter_init(v)?;
                self.set_reg(*dst, RegVal::Iter(Box::new(it)));
            }
            InstKind::ForTest {
                dst,
                counter,
                end,
                step,
            } => {
                let c = self.val(counter)?;
                let e = self.val(end)?;
                let s = self.val(step)?;
                let opts = CompareOptions {
                    mode: CompareMode::Binary,
                    string_number: ops::StringNumberRule::VariantOrdering,
                };
                let up = convert::to_boolean(&ops::compare(
                    CompareOp::Ge,
                    &s,
                    &Variant::Integer(0),
                    opts,
                )?)?;
                let cmp = if up { CompareOp::Le } else { CompareOp::Ge };
                let t = convert::to_boolean(&ops::compare(cmp, &c, &e, opts)?)?;
                self.set_val(*dst, Variant::Boolean(t));
            }
            InstKind::SetErrorMode { mode } => {
                let fr = self.frames.last_mut().expect("frame");
                fr.mode = *mode;
                fr.in_handler = false;
            }
            InstKind::ClearError => {
                let fr = self.frames.last_mut().expect("frame");
                fr.in_handler = false;
                fr.fail = None;
                self.err = ErrState {
                    last_dll_error: self.err.last_dll_error,
                    ..ErrState::default()
                };
            }
            InstKind::SetLine { line } => {
                self.frames.last_mut().expect("frame").line = *line;
            }
            InstKind::RaiseEvent { event, args } => {
                let me = self.me_of_frame().ok_or(VbaError::new(91))?;
                let bound = self.collect_args(args)?;
                self.raise_event(me, *event, bound)?;
            }
            InstKind::DebugPrint { items } => self.exec_debug_print(items)?,
            InstKind::Io(op) => self.exec_io(func, op)?,
        }
        Ok(Flow::Next)
    }

    fn load_into(&mut self, dst: vba_ir::Reg, loc: &Loc) -> VResult<()> {
        let rv = match self.peek(loc)? {
            std::borrow::Cow::Borrowed(Variant::Array(_)) => RegVal::Alias(loc.clone()),
            other => RegVal::V(other.into_owned()),
        };
        self.set_reg(dst, rv);
        Ok(())
    }

    fn index_operand(&mut self, array: &Operand, idx: &[i32]) -> VResult<Variant> {
        if let Operand::Reg(r) = array {
            let fr = self.frames.last().ok_or_else(crate::heap::no_frame)?;
            match &fr.regs[r.0 as usize] {
                RegVal::Alias(loc) => {
                    let loc = loc.clone();
                    return match &*self.peek(&loc)? {
                        Variant::Array(a) => Ok(a.get(idx)?.clone()),
                        _ => Err(VbaError::TYPE_MISMATCH.into()),
                    };
                }
                RegVal::V(Variant::Array(a)) => return Ok(a.get(idx)?.clone()),
                _ => {}
            }
        }
        match self.val(array)? {
            Variant::Array(a) => Ok(a.get(idx)?.clone()),
            Variant::Empty => Err(VbaError::SUBSCRIPT_OUT_OF_RANGE.into()),
            _ => Err(VbaError::TYPE_MISMATCH.into()),
        }
    }

    fn type_of_is(&self, v: &Variant, ty: &Type) -> bool {
        let Variant::Object(ObjectRef::Handle(h)) = v else {
            return false;
        };
        match (ty, vm_id(*h)) {
            (Type::Object, _) => true,
            (Type::Class(m), Some(id)) => self.instance_of(id, *m),
            (Type::Library(lt), Some(id)) => matches!(
                (self.objects.get(&id).map(|o| &o.kind), lt.name.as_str()),
                (Some(ObjKind::Collection(_)), "Collection")
                    | (Some(ObjKind::Err), "ErrObject")
                    | (Some(ObjKind::Debug), "DebugObject")
            ),
            (Type::Library(lt), None) => self
                .host
                .objects()
                .get(*h)
                .map(|o| o.class_name().eq_ignore_ascii_case(&lt.name))
                .unwrap_or(false),
            _ => false,
        }
    }

    fn create_new(&mut self, target: &vba_ir::NewTarget) -> VResult<Variant> {
        match target {
            vba_ir::NewTarget::Class(m) => {
                let id = self.new_class_instance(*m)?;
                Ok(Variant::Object(ObjectRef::Handle(vm_handle(id))))
            }
            vba_ir::NewTarget::Library(lt) => {
                if lt.library == "VBA" && lt.name == "Collection" {
                    self.charge_memory(64)?;
                    let id = self.next_obj;
                    self.next_obj += 1;
                    self.objects.insert(
                        id,
                        Obj {
                            kind: ObjKind::Collection(Vec::new()),
                            refs: 1,
                            terminating: false,
                            sinks: Vec::new(),
                            host_base: None,
                        },
                    );
                    return Ok(Variant::Object(ObjectRef::Handle(vm_handle(id))));
                }
                let progid = if lt.library.is_empty() {
                    lt.name.clone()
                } else {
                    format!("{}.{}", lt.library, lt.name)
                };
                let v = self
                    .host
                    .create_object(&progid, None)
                    .map_err(Fault::from)?;
                self.drain_events()?;
                Ok(v)
            }
        }
    }

    fn lib_global(&mut self, name: &str) -> VResult<Variant> {
        if name.eq_ignore_ascii_case("err") {
            return Ok(Variant::Object(ObjectRef::Handle(vm_handle(ERR_ID))));
        }
        if name.eq_ignore_ascii_case("debug") {
            return Ok(Variant::Object(ObjectRef::Handle(vm_handle(DEBUG_ID))));
        }
        match self.host.resolve_global(name).map_err(Fault::from)? {
            Some(v) => {
                self.own(&v);
                Ok(v)
            }
            None => Err(VbaError::new(424).into()),
        }
    }

    fn exec_call(
        &mut self,
        ir: &IrProgram,
        dst: Option<vba_ir::Reg>,
        callee: &Callee,
        receiver: Option<&Operand>,
        args: &[CallArg],
    ) -> VResult<Flow> {
        match callee {
            Callee::Proc(pid) => {
                let recv = match receiver {
                    Some(op) => Some(self.val(op)?),
                    None => None,
                };
                let me = self.me_of_frame();
                let (target, tme) = self.resolve_target(*pid, recv.as_ref(), me)?;
                let bound = self.collect_args(args)?;
                self.push_proc_frame(target, tme, bound, dst)?;
                Ok(Flow::Pushed)
            }
            Callee::Declare(d) => {
                let v = self.exec_declare(ir, *d, args)?;
                if let Some(r) = dst {
                    self.own(&v);
                    self.set_val(r, v);
                }
                Ok(Flow::Next)
            }
            Callee::Intrinsic { name, dollar } => {
                if dst.is_none() && name.eq_ignore_ascii_case("error") {
                    // The `Error n` statement raises error `n`; the function
                    // form (`Error(n)`) returns the description.
                    let a = self.intrinsic_args_for_error(args)?;
                    return Err(RtErr::standard(a).into());
                }
                let rv = self.exec_intrinsic(name, *dollar, args)?;
                if let Some(r) = dst {
                    if let RegVal::V(v) = &rv {
                        self.own(v);
                    }
                    self.set_reg(r, rv);
                } else if let RegVal::V(v) = rv {
                    self.own(&v);
                }
                Ok(Flow::Next)
            }
        }
    }

    fn intrinsic_args_for_error(&mut self, args: &[CallArg]) -> VResult<i32> {
        let Some(CallArg::Val(op)) = args.first() else {
            return Err(VbaError::new(449).into());
        };
        let v = self.val(op)?;
        let n = self.long_of(v)?;
        if !(1..=65535).contains(&n) {
            return Err(VbaError::INVALID_PROCEDURE_CALL.into());
        }
        Ok(n)
    }

    fn exec_redim(
        &mut self,
        r: vba_ir::Reg,
        preserve: bool,
        bounds: &[(Option<Operand>, Operand)],
        elem: &Type,
    ) -> VResult<()> {
        let module: ModuleId = self.frames.last().expect("frame").module;
        let base = i32::from(self.ir.program.module(module).options.base);
        let mut dims = Vec::with_capacity(bounds.len());
        for (lo, hi) in bounds {
            let lo = match lo {
                Some(o) => {
                    let v = self.val(o)?;
                    self.long_of(v)?
                }
                None => base,
            };
            let v = self.val(hi)?;
            let hi = self.long_of(v)?;
            dims.push(Bounds::new(lo, hi));
        }
        let mut total: u64 = 1;
        for d in &dims {
            if d.upper < d.lower {
                return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE.into());
            }
            total = total.saturating_mul(d.len());
        }
        if total > self.cfg.max_array_elements {
            return Err(VbaError::OUT_OF_MEMORY.into());
        }
        let loc = self.reg_loc(r)?;
        let mut arr = match self.load(&loc)? {
            Variant::Array(a) => *a,
            Variant::Empty => VbaArray::new_dynamic(crate::heap::elem_vartype(elem)),
            _ => return Err(VbaError::TYPE_MISMATCH.into()),
        };
        self.check_room(vba_value::ELEMENT_BYTES.saturating_mul(total))?;
        let old: Vec<Variant> = arr.iter().cloned().collect();
        arr.redim(&dims, preserve)?;
        let new_len = arr.len() as u64;
        self.fill_special(&mut arr, elem)?;
        let slot = self.var_mut(&loc)?;
        *slot = Variant::Array(Box::new(arr));
        // Elements that did not survive release their references.
        let dropped = if preserve {
            old.into_iter().skip(new_len as usize).collect()
        } else {
            old
        };
        for v in dropped {
            self.release_value(v)?;
        }
        Ok(())
    }

    fn exec_erase(&mut self, func: &Function, r: vba_ir::Reg) -> VResult<()> {
        let elem = match &func.regs[r.0 as usize] {
            Type::Array { elem, .. } => (**elem).clone(),
            _ => Type::Variant,
        };
        let loc = self.reg_loc(r)?;
        let Variant::Array(mut a) = self.load(&loc)? else {
            return Err(VbaError::TYPE_MISMATCH.into());
        };
        let old: Vec<Variant> = a.iter().cloned().collect();
        let fixed = a.is_fixed();
        a.erase();
        if fixed {
            self.fill_special(&mut a, &elem)?;
        }
        let slot = self.var_mut(&loc)?;
        *slot = Variant::Array(a);
        for v in old {
            self.release_value(v)?;
        }
        Ok(())
    }

    // ---- terminators ----------------------------------------------------------------

    fn goto(&mut self, b: vba_ir::BlockId) {
        let fr = self.frames.last_mut().expect("frame");
        fr.block = b.0;
        fr.idx = 0;
    }

    fn exec_term(
        &mut self,
        ir: &IrProgram,
        func: &Function,
        block: &Block,
        base: usize,
    ) -> VResult<Flow> {
        self.host.budget().charge_step().map_err(Fault::from)?;
        self.poll()?;
        if !self.frames.last().expect("frame").owned.is_empty() {
            self.release_owned()?;
        }
        self.frames.last_mut().expect("frame").cur_stmt = block.term_stmt.0;
        match &block.term {
            Terminator::Jump(b) => {
                self.goto(*b);
                Ok(Flow::Jumped)
            }
            Terminator::Branch {
                cond,
                then_block,
                else_block,
            } => {
                let c = self.val(cond)?;
                let t = self.truthy(c)?;
                self.goto(if t { *then_block } else { *else_block });
                Ok(Flow::Jumped)
            }
            Terminator::Switch {
                selector,
                targets,
                default,
            } => {
                let v = self.val(selector)?;
                let n = self.long_of(v)?;
                if !(0..=255).contains(&n) {
                    return Err(VbaError::INVALID_PROCEDURE_CALL.into());
                }
                let t = if n >= 1 && (n as usize) <= targets.len() {
                    targets[n as usize - 1]
                } else {
                    *default
                };
                self.goto(t);
                Ok(Flow::Jumped)
            }
            Terminator::GoSub { target, ret } => {
                let max = self.cfg.max_gosub_depth;
                let fr = self.frames.last_mut().expect("frame");
                if fr.gosub.len() >= max {
                    return Err(VbaError::new(28).into());
                }
                fr.gosub.push(*ret);
                self.goto(*target);
                Ok(Flow::Jumped)
            }
            Terminator::ReturnFromGoSub => {
                let fr = self.frames.last_mut().expect("frame");
                match fr.gosub.pop() {
                    Some(b) => {
                        self.goto(b);
                        Ok(Flow::Jumped)
                    }
                    None => Err(VbaError::new(3).into()),
                }
            }
            Terminator::ForEachNext {
                iter,
                item,
                body,
                done,
            } => {
                let mut it = match std::mem::take(
                    &mut self.frames.last_mut().expect("frame").regs[iter.0 as usize],
                ) {
                    RegVal::Iter(it) => it,
                    // `Next` of a `For Each` whose enumeration was never started
                    // (control entered the body with a `GoTo`; MS-VBAL 5.4.2.4).
                    RegVal::Empty => return Err(VbaError::new(92).into()),
                    _ => return Err(Fault::Internal("iterator expected".into())),
                };
                let r = self.iter_next(&mut it);
                self.frames.last_mut().expect("frame").regs[iter.0 as usize] = RegVal::Iter(it);
                match r? {
                    Some(v) => {
                        self.set_val(*item, v);
                        self.goto(*body);
                    }
                    None => self.goto(*done),
                }
                Ok(Flow::Jumped)
            }
            Terminator::Resume(kind) => {
                let fr = self.frames.last_mut().expect("frame");
                if !fr.in_handler {
                    return Err(RtErr::new(20, "Resume without error").into());
                }
                let (b, i, stmt) = fr
                    .fail
                    .take()
                    .ok_or(RtErr::new(20, "Resume without error"))?;
                fr.in_handler = false;
                match kind {
                    ResumeKind::Same => {
                        fr.block = b;
                        fr.idx = i;
                    }
                    ResumeKind::Next => {
                        let Point { block, index } = func.stmts[stmt.0 as usize]
                            .resume_next
                            .ok_or(RtErr::new(20, "Resume without error"))?;
                        fr.block = block.0;
                        fr.idx = index;
                        self.err = ErrState {
                            last_dll_error: self.err.last_dll_error,
                            ..ErrState::default()
                        };
                    }
                    ResumeKind::Block(t) => {
                        fr.block = t.0;
                        fr.idx = 0;
                    }
                }
                Ok(Flow::Jumped)
            }
            Terminator::Return => {
                let (ret_dst, in_handler) = {
                    let fr = self.frames.last().expect("frame");
                    (fr.ret_dst, fr.in_handler)
                };
                if in_handler {
                    self.err = ErrState {
                        last_dll_error: self.err.last_dll_error,
                        ..ErrState::default()
                    };
                }
                let result = self.pop_frame()?;
                if self.frames.len() <= base {
                    return Ok(Flow::Return(result));
                }
                match ret_dst {
                    Some(r) => {
                        self.own(&result);
                        self.set_val(r, result);
                    }
                    None => self.own(&result),
                }
                self.frames.last_mut().expect("frame").idx += 1;
                Ok(Flow::Jumped)
            }
            Terminator::End => Err(Fault::End),
            Terminator::Stop => {
                self.stop_reason = Some("Stop statement".into());
                Err(Fault::Stop)
            }
            Terminator::Unreachable => {
                let _ = ir;
                Err(Fault::Internal("reached unreachable code".into()))
            }
        }
    }
}
