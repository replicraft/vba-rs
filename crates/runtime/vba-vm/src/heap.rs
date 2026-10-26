//! Values, locations, object and user-defined-type storage.

use std::borrow::Cow;
use std::collections::BTreeSet;

use vba_ir::Operand;
use vba_sema::bound::Constant;
use vba_sema::{Type, UdtId};
use vba_value::{
    Bounds, Currency, Date, Decimal, ObjectHandle, ObjectRef, UdtHandle, VarType, Variant,
    VbaArray, VbaError, VbaString, convert,
};

use crate::error::{Fault, LimitKind, VResult};
use crate::state::*;
use crate::vm::Vm;

/// The element type used to store values of a static type in an array.
pub(crate) fn elem_vartype(t: &Type) -> VarType {
    t.var_type().unwrap_or(VarType::Variant)
}

/// Whether arrays of this element type may hold references that need
/// counting or copying.
fn has_refs(t: VarType) -> bool {
    matches!(
        t,
        VarType::Variant | VarType::Object | VarType::DataObject | VarType::UserDefinedType
    )
}

impl Vm {
    // ---- defaults ---------------------------------------------------------

    /// The initial value of a variable of static type `ty`.
    pub(crate) fn default_for(&mut self, ty: &Type) -> VResult<Variant> {
        Ok(match ty {
            Type::Boolean => Variant::Boolean(false),
            Type::Byte => Variant::Byte(0),
            Type::Integer => Variant::Integer(0),
            Type::Long | Type::Enum(_) => Variant::Long(0),
            Type::LongLong => Variant::LongLong(0),
            Type::Single => Variant::Single(0.0),
            Type::Double => Variant::Double(0.0),
            Type::Currency => Variant::Currency(Currency::ZERO),
            Type::Date => Variant::Date(Date::default()),
            Type::Decimal => Variant::Decimal(Decimal::ZERO),
            Type::String => Variant::String(VbaString::new()),
            Type::FixedString(n) => nul_string(*n),
            Type::Object | Type::Class(_) | Type::Library(_) | Type::Nothing => {
                Variant::Object(ObjectRef::Nothing)
            }
            Type::Udt(u) => udt_variant(self.alloc_udt(*u)?),
            Type::Array { elem, .. } => {
                Variant::Array(Box::new(VbaArray::new_dynamic(elem_vartype(elem))))
            }
            Type::Variant | Type::Any | Type::Error => Variant::Empty,
        })
    }

    /// Allocates a user-defined-type value with default fields.
    pub(crate) fn alloc_udt(&mut self, id: UdtId) -> VResult<u64> {
        let ir = self.ir.clone();
        let info = &ir.program.udts[id.index()];
        self.charge_memory(32 + 32 * info.fields.len() as u64)?;
        let mut fields = Vec::with_capacity(info.fields.len());
        for f in &info.fields {
            let v = match (&f.ty, &f.fixed_bounds) {
                (Type::Array { elem, .. }, Some(b)) => {
                    Variant::Array(Box::new(self.make_fixed_array(elem, b)?))
                }
                (t, _) => self.default_for(t)?,
            };
            fields.push(v);
        }
        let h = self.next_udt;
        self.next_udt += 1;
        self.udts.insert(h, UdtVal { udt: id, fields });
        Ok(h)
    }

    /// Creates a fixed-size array with default elements.
    pub(crate) fn make_fixed_array(
        &mut self,
        elem: &Type,
        bounds: &[(i32, i32)],
    ) -> VResult<VbaArray> {
        let dims: Vec<Bounds> = bounds.iter().map(|(l, u)| Bounds::new(*l, *u)).collect();
        let total = dims
            .iter()
            .try_fold(1u64, |a, b| a.checked_mul(b.len()))
            .ok_or(Fault::Limit(LimitKind::ArraySize))?;
        if total > self.cfg.max_array_elements {
            return Err(VbaError::OUT_OF_MEMORY.into());
        }
        self.charge_memory(32 * total)?;
        let mut a = VbaArray::new_fixed(elem_vartype(elem), &dims)?;
        self.fill_special(&mut a, elem)?;
        Ok(a)
    }

    /// Gives elements whose default is not `VarType::default_value` their
    /// proper initial value (user-defined types, fixed-length strings).
    pub(crate) fn fill_special(&mut self, a: &mut VbaArray, elem: &Type) -> VResult<()> {
        match elem {
            Type::Udt(u) => {
                for e in a.iter_mut() {
                    if matches!(e, Variant::Empty) {
                        *e = udt_variant(self.alloc_udt(*u)?);
                    }
                }
            }
            Type::FixedString(n) => {
                for e in a.iter_mut() {
                    *e = nul_string(*n);
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ---- memory accounting ------------------------------------------------

    pub(crate) fn charge_memory(&self, bytes: u64) -> VResult<()> {
        self.host.budget().alloc(bytes).map_err(Fault::from)
    }

    pub(crate) fn free_memory(&self, bytes: u64) {
        self.host.budget().free(bytes);
    }

    /// Fails if `s` exceeds the configured string limit.
    pub(crate) fn check_string(&self, v: &Variant) -> VResult<()> {
        if let Variant::String(s) = v
            && s.len() > self.cfg.max_string_units
        {
            return Err(Fault::Limit(LimitKind::StringLength));
        }
        Ok(())
    }

    // ---- constants and operands -------------------------------------------

    pub(crate) fn constant(&self, c: &Constant) -> VResult<Variant> {
        match c {
            Constant::Value(v) => Ok(v.clone()),
            Constant::DateText(b) => {
                let text = String::from_utf8_lossy(b).into_owned();
                Ok(convert::convert_to(
                    &Variant::from_text(&text),
                    VarType::Date,
                )?)
            }
        }
    }

    /// The value of an operand (registers holding aliases are loaded).
    pub(crate) fn val(&self, op: &Operand) -> VResult<Variant> {
        match op {
            Operand::Const(c) => self.constant(c),
            Operand::Reg(r) => {
                let fr = self.frames.last().ok_or_else(no_frame)?;
                match &fr.regs[r.0 as usize] {
                    RegVal::V(v) => Ok(v.clone()),
                    RegVal::Alias(loc) => self.load(loc),
                    RegVal::Empty => Err(Fault::Internal(format!(
                        "register r{} read before it was defined",
                        r.0
                    ))),
                    _ => Err(Fault::Internal(format!(
                        "register r{} does not hold a value",
                        r.0
                    ))),
                }
            }
        }
    }

    /// The location a register refers to.
    pub(crate) fn reg_loc(&self, r: vba_ir::Reg) -> VResult<Loc> {
        let fr = self.frames.last().ok_or_else(no_frame)?;
        match &fr.regs[r.0 as usize] {
            RegVal::Ref(l) | RegVal::Alias(l) => Ok(l.clone()),
            _ => Err(Fault::Internal(format!(
                "register r{} does not hold a reference",
                r.0
            ))),
        }
    }

    pub(crate) fn set_reg(&mut self, r: vba_ir::Reg, v: RegVal) {
        let fr = self.frames.last_mut().expect("frame");
        fr.regs[r.0 as usize] = v;
    }

    pub(crate) fn set_val(&mut self, r: vba_ir::Reg, v: Variant) {
        self.set_reg(r, RegVal::V(v));
    }

    // ---- locations --------------------------------------------------------

    /// Reads a location without copying where possible.
    pub(crate) fn peek<'a>(&'a self, loc: &Loc) -> VResult<Cow<'a, Variant>> {
        match loc {
            Loc::Local { frame, slot } => {
                match self
                    .frames
                    .get(*frame)
                    .and_then(|f| f.slots.get(*slot as usize))
                {
                    Some(Slot::Val(v)) => Ok(Cow::Borrowed(v)),
                    Some(Slot::Bound(l)) => self.peek(l),
                    None => Err(Fault::Internal("dangling local reference".into())),
                }
            }
            Loc::Global(g) => self
                .globals
                .get(*g as usize)
                .map(Cow::Borrowed)
                .ok_or_else(|| Fault::Internal("bad global".into())),
            Loc::Field { obj, index } => match self.objects.get(obj).map(|o| &o.kind) {
                Some(ObjKind::Class { fields, .. }) => fields
                    .get(*index as usize)
                    .map(Cow::Borrowed)
                    .ok_or_else(|| Fault::Internal("bad field".into())),
                _ => Err(VbaError::new(91).into()),
            },
            Loc::Elem { base, idx } => {
                let b = self.peek(base)?;
                match &*b {
                    Variant::Array(a) => Ok(Cow::Owned(a.get(idx)?.clone())),
                    Variant::Empty => Err(VbaError::SUBSCRIPT_OUT_OF_RANGE.into()),
                    _ => Err(VbaError::TYPE_MISMATCH.into()),
                }
            }
            Loc::UdtField { base, index } => {
                let h = match &*self.peek(base)? {
                    Variant::UserDefined(h) => *h,
                    _ => return Err(VbaError::TYPE_MISMATCH.into()),
                };
                self.udts
                    .get(&h.0)
                    .and_then(|u| u.fields.get(*index as usize))
                    .map(Cow::Borrowed)
                    .ok_or_else(|| Fault::Internal("dangling user-defined type".into()))
            }
        }
    }

    pub(crate) fn load(&self, loc: &Loc) -> VResult<Variant> {
        self.peek(loc).map(Cow::into_owned)
    }

    /// Mutable access to a location that is stored as a plain `Variant`
    /// (everything except array elements).
    pub(crate) fn var_mut(&mut self, loc: &Loc) -> VResult<&mut Variant> {
        match loc {
            Loc::Local { frame, slot } => {
                let bound = match self
                    .frames
                    .get(*frame)
                    .and_then(|f| f.slots.get(*slot as usize))
                {
                    Some(Slot::Bound(l)) => Some(l.clone()),
                    Some(Slot::Val(_)) => None,
                    None => return Err(Fault::Internal("dangling local reference".into())),
                };
                if let Some(l) = bound {
                    return self.var_mut(&l);
                }
                match &mut self.frames[*frame].slots[*slot as usize] {
                    Slot::Val(v) => Ok(v),
                    Slot::Bound(_) => unreachable!("checked above"),
                }
            }
            Loc::Global(g) => self
                .globals
                .get_mut(*g as usize)
                .ok_or_else(|| Fault::Internal("bad global".into())),
            Loc::Field { obj, index } => match self.objects.get_mut(obj).map(|o| &mut o.kind) {
                Some(ObjKind::Class { fields, .. }) => fields
                    .get_mut(*index as usize)
                    .ok_or_else(|| Fault::Internal("bad field".into())),
                _ => Err(VbaError::new(91).into()),
            },
            Loc::UdtField { base, index } => {
                let h = match self.load(base)? {
                    Variant::UserDefined(h) => h,
                    _ => return Err(VbaError::TYPE_MISMATCH.into()),
                };
                self.udts
                    .get_mut(&h.0)
                    .and_then(|u| u.fields.get_mut(*index as usize))
                    .ok_or_else(|| Fault::Internal("dangling user-defined type".into()))
            }
            Loc::Elem { .. } => Err(Fault::Unsupported(
                "taking a reference to an element of an array nested in an array".into(),
            )),
        }
    }

    /// Follows `ByRef` bindings of frame slots to the location they stand for.
    pub(crate) fn resolve_loc(&self, loc: &Loc) -> Loc {
        let mut cur = loc.clone();
        while let Loc::Local { frame, slot } = &cur {
            match self
                .frames
                .get(*frame)
                .and_then(|f| f.slots.get(*slot as usize))
            {
                Some(Slot::Bound(l)) => cur = l.clone(),
                _ => break,
            }
        }
        cur
    }

    fn array_mut(&mut self, base: &Loc) -> VResult<&mut VbaArray> {
        match self.var_mut(base)? {
            Variant::Array(a) => Ok(a),
            Variant::Empty => Err(VbaError::SUBSCRIPT_OUT_OF_RANGE.into()),
            _ => Err(VbaError::TYPE_MISMATCH.into()),
        }
    }

    /// Stores `v` at `loc`: user-defined types and object references are
    /// copied or retained, the previous value is released.
    pub(crate) fn store(&mut self, loc: &Loc, v: Variant) -> VResult<()> {
        let resolved = self.resolve_loc(loc);
        let loc = &resolved;
        let new = self.adopt(v)?;
        match loc {
            Loc::Elem { base, idx } => {
                let res = (|| -> VResult<Variant> {
                    let arr = self.array_mut(base)?;
                    let old = arr.get(idx)?.clone();
                    arr.set(idx, new.clone())?;
                    Ok(old)
                })();
                match res {
                    Ok(old) => self.release_value(old),
                    Err(e) => {
                        let _ = self.release_value(new);
                        Err(e)
                    }
                }
            }
            _ => match self.var_mut(loc) {
                Ok(slot) => {
                    let old = std::mem::replace(slot, new);
                    self.release_value(old)
                }
                Err(e) => {
                    let _ = self.release_value(new);
                    Err(e)
                }
            },
        }
    }

    // ---- ownership --------------------------------------------------------

    /// Prepares a value for storage: copies user-defined types (value
    /// semantics), retains object references, and does both for arrays of
    /// such elements.
    pub(crate) fn adopt(&mut self, v: Variant) -> VResult<Variant> {
        match v {
            Variant::UserDefined(h) => {
                let src = self
                    .udts
                    .get(&h.0)
                    .cloned()
                    .ok_or_else(|| Fault::Internal("dangling user-defined type".into()))?;
                self.charge_memory(32 + 32 * src.fields.len() as u64)?;
                let mut fields = Vec::with_capacity(src.fields.len());
                for f in src.fields {
                    fields.push(self.adopt(f)?);
                }
                let id = self.next_udt;
                self.next_udt += 1;
                self.udts.insert(
                    id,
                    UdtVal {
                        udt: src.udt,
                        fields,
                    },
                );
                Ok(Variant::UserDefined(UdtHandle(id)))
            }
            Variant::Object(ObjectRef::Handle(h)) => {
                self.retain_handle(h);
                Ok(Variant::Object(ObjectRef::Handle(h)))
            }
            Variant::Array(mut a) => {
                if has_refs(a.element_type()) {
                    for e in a.iter_mut() {
                        let old = std::mem::replace(e, Variant::Empty);
                        *e = self.adopt(old)?;
                    }
                }
                Ok(Variant::Array(a))
            }
            other => Ok(other),
        }
    }

    pub(crate) fn retain_handle(&mut self, h: ObjectHandle) {
        match vm_id(h) {
            Some(id) => {
                if let Some(o) = self.objects.get_mut(&id) {
                    o.refs = o.refs.saturating_add(1);
                }
            }
            None => {
                let _ = self.host.objects().add_ref(h);
            }
        }
    }

    /// Adds a reference for every object inside `v` (without copying).
    pub(crate) fn retain_value(&mut self, v: &Variant) {
        match v {
            Variant::Object(ObjectRef::Handle(h)) => self.retain_handle(*h),
            Variant::Array(a) if has_refs(a.element_type()) => {
                for e in a.iter() {
                    self.retain_value(e);
                }
            }
            _ => {}
        }
    }

    /// Drops the references held by `v`.
    pub(crate) fn release_value(&mut self, v: Variant) -> VResult<()> {
        match v {
            Variant::Object(ObjectRef::Handle(h)) => self.release_handle(h),
            Variant::Array(a) if has_refs(a.element_type()) => {
                let mut first: VResult<()> = Ok(());
                for e in a.iter() {
                    if let Err(f) = self.release_value(e.clone())
                        && first.is_ok()
                    {
                        first = Err(f);
                    }
                }
                first
            }
            _ => Ok(()),
        }
    }

    /// Like [`Vm::release_value`] but swallows trappable errors (used when
    /// a frame is torn down).
    pub(crate) fn release_quiet(&mut self, v: Variant) -> VResult<()> {
        match self.release_value(v) {
            Err(Fault::Err(_)) => Ok(()),
            other => other,
        }
    }

    /// Whether a value carries references that need releasing later.
    pub(crate) fn needs_release(v: &Variant) -> bool {
        match v {
            Variant::Object(ObjectRef::Handle(_)) => true,
            Variant::Array(a) => has_refs(a.element_type()),
            _ => false,
        }
    }

    // ---- user-defined type collection -------------------------------------

    fn mark(&self, v: &Variant, marks: &mut BTreeSet<u64>) {
        match v {
            Variant::UserDefined(h) => {
                if marks.insert(h.0)
                    && let Some(u) = self.udts.get(&h.0)
                {
                    for f in &u.fields {
                        self.mark(f, marks);
                    }
                }
            }
            Variant::Array(a) if has_refs(a.element_type()) => {
                for e in a.iter() {
                    self.mark(e, marks);
                }
            }
            _ => {}
        }
    }

    /// Frees user-defined-type values no longer reachable from a frame,
    /// module variable, object or collection.
    pub(crate) fn collect_udts(&mut self) {
        let mut marks = BTreeSet::new();
        for fr in &self.frames {
            for s in &fr.slots {
                if let Slot::Val(v) = s {
                    self.mark(v, &mut marks);
                }
            }
            for r in &fr.regs {
                if let RegVal::V(v) = r {
                    self.mark(v, &mut marks);
                }
            }
            for v in fr.owned.iter().chain(&fr.held) {
                self.mark(v, &mut marks);
            }
        }
        for v in &self.globals {
            self.mark(v, &mut marks);
        }
        for o in self.objects.values() {
            match &o.kind {
                ObjKind::Class { fields, .. } => {
                    for v in fields {
                        self.mark(v, &mut marks);
                    }
                }
                ObjKind::Collection(items) => {
                    for i in items {
                        self.mark(&i.value, &mut marks);
                    }
                }
                _ => {}
            }
        }
        let dead: Vec<u64> = self
            .udts
            .keys()
            .filter(|k| !marks.contains(k))
            .copied()
            .collect();
        for k in dead {
            if let Some(u) = self.udts.remove(&k) {
                self.free_memory(32 + 32 * u.fields.len() as u64);
                for f in u.fields {
                    let _ = self.release_quiet(f);
                }
            }
        }
        self.gc_threshold = (self.udts.len() * 2).max(4096);
    }
}

pub(crate) fn no_frame() -> Fault {
    Fault::Internal("no active frame".into())
}
