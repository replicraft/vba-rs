//! Runtime binding: `Dispatch` objects whose metadata and argument
//! handling come from a checked description.
//!
//! A [`HostObject`] answers `member_id`, `member_info` and
//! `get_ids_of_names` from the description, binds every call with
//! `vba_automation::bind` against the described parameters, converts each
//! argument to its declared type, runs the implementation
//! ([`HostImpl`]) with the typed values, writes `ByRef` results back and
//! converts the result to the declared type. Late-bound calls (by name)
//! and early-bound calls (by `DISPID`) both end in the same code, so they
//! behave identically.

use std::collections::HashMap;

use vba_automation::{
    Arg, Bound, ConnectionPoint, DispError, DispId, DispParams, Dispatch, EventSource,
    FOREIGN_HANDLE_BIT, InvokeFlags, MemberInfo, ObjectTable, VariantEnum, names_equal,
};
use vba_sema::Type;
use vba_value::{ObjectHandle, ObjectRef, VarType, Variant, convert};

use crate::check::{CheckedLibrary, MemberSig, key};

/// A problem binding an implementation to a description.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BindError {
    /// The library has no such class.
    #[error("the library has no class `{0}`")]
    UnknownClass(String),
    /// The library has no such global function.
    #[error("the library has no global function `{0}`")]
    UnknownFunction(String),
    /// The library has no such predeclared global object.
    #[error("the library has no global object `{0}`")]
    UnknownGlobal(String),
    /// An implementation names a member the class does not have.
    #[error("class `{class}` has no member `{member}`")]
    UnknownMember {
        /// The class.
        class: String,
        /// The member.
        member: String,
    },
    /// An implementation handles a kind of call the member does not
    /// support (a `Let` of a read-only property).
    #[error("`{class}.{member}` does not support {kind:?}")]
    KindNotSupported {
        /// The class.
        class: String,
        /// The member.
        member: String,
        /// The kind of call.
        kind: CallKind,
    },
    /// A described member has no implementation.
    #[error("`{class}.{member}` is described but not implemented ({kind:?})")]
    Unimplemented {
        /// The class (empty for a global function).
        class: String,
        /// The member.
        member: String,
        /// The kind of call.
        kind: CallKind,
    },
    /// A creatable class has no factory, or a predeclared object has no
    /// factory.
    #[error("`{0}` has no factory")]
    NoFactory(String),
}

impl From<BindError> for DispError {
    fn from(e: BindError) -> DispError {
        DispError::raise(5, e.to_string(), "vba-host-model")
    }
}

/// What kind of call reached an implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallKind {
    /// A `Sub` or `Function` call.
    Call,
    /// A `Property Get`.
    Get,
    /// A `Property Let` (the value is in [`Args::value`]).
    Let,
    /// A `Property Set`.
    Set,
    /// `For Each` asks for the enumerator (`_NewEnum`).
    NewEnum,
}

/// The arguments of one call, bound to the described parameters and
/// converted to their declared types.
pub struct Args<'a> {
    sig: &'a MemberSig,
    bound: Bound,
}

impl Args<'_> {
    /// Number of described parameters (without the `ParamArray`).
    pub fn len(&self) -> usize {
        self.bound.slots.len()
    }

    /// `true` if the member has no fixed parameters.
    pub fn is_empty(&self) -> bool {
        self.bound.slots.is_empty()
    }

    /// `true` if the argument was omitted and the parameter has no
    /// default (`IsMissing`).
    pub fn is_missing(&self, i: usize) -> bool {
        self.bound.slots.get(i).is_none_or(Arg::is_missing)
    }

    /// The value of argument `i`; `None` if it was omitted without a
    /// default.
    pub fn get(&self, i: usize) -> Option<&Variant> {
        self.bound
            .slots
            .get(i)
            .filter(|a| !a.is_missing())
            .map(|a| &a.value)
    }

    /// The value of the argument for the parameter called `name`.
    pub fn by_name(&self, name: &str) -> Option<&Variant> {
        let i = self
            .sig
            .params
            .iter()
            .filter(|p| !p.param_array)
            .position(|p| names_equal(&p.name, name))?;
        self.get(i)
    }

    /// The `ParamArray` elements.
    pub fn rest(&self) -> impl Iterator<Item = &Variant> {
        self.bound.rest.iter().map(|a| &a.value)
    }

    /// The value a `Property Let`/`Set` assigns.
    pub fn value(&self) -> Result<&Variant, DispError> {
        self.bound
            .value
            .as_ref()
            .ok_or(DispError::PARAM_NOT_OPTIONAL)
    }

    /// Assigns an output parameter. The caller's variable changes only if
    /// the parameter is declared `ByRef` and the caller passed a variable.
    pub fn set(&mut self, i: usize, v: impl Into<Variant>) {
        if let Some(a) = self.bound.slots.get_mut(i) {
            a.value = v.into();
        }
    }

    fn need(&self, i: usize) -> Result<&Variant, DispError> {
        self.get(i).ok_or(DispError::PARAM_NOT_OPTIONAL)
    }

    /// Argument `i` as a `String`; an omitted argument is an error.
    pub fn string(&self, i: usize) -> Result<String, DispError> {
        Ok(convert::to_vba_string(self.need(i)?)?.to_string_lossy())
    }

    /// Argument `i` as a `Long`; an omitted argument is an error.
    pub fn long(&self, i: usize) -> Result<i32, DispError> {
        Ok(convert::to_long(self.need(i)?)?)
    }

    /// Argument `i` as a `Double`; an omitted argument is an error.
    pub fn double(&self, i: usize) -> Result<f64, DispError> {
        Ok(convert::to_double(self.need(i)?)?)
    }

    /// Argument `i` as a `Boolean`; an omitted argument is an error.
    pub fn boolean(&self, i: usize) -> Result<bool, DispError> {
        Ok(convert::to_boolean(self.need(i)?)?)
    }

    /// Argument `i` as an object handle (`None` for `Nothing`); an omitted
    /// argument is an error.
    pub fn object(&self, i: usize) -> Result<Option<ObjectHandle>, DispError> {
        match self.need(i)? {
            Variant::Object(ObjectRef::Handle(h)) => Ok(Some(*h)),
            Variant::Object(ObjectRef::Nothing) => Ok(None),
            _ => Err(DispError::TYPE_MISMATCH),
        }
    }
}

/// One call, as the implementation sees it.
pub struct Invocation<'a> {
    /// The object table, to create or reference objects.
    pub table: &'a ObjectTable,
    /// What kind of call this is.
    pub kind: CallKind,
    /// The signature of the member (from the description).
    pub sig: &'a MemberSig,
    /// The class name (empty for a global function).
    pub class: &'a str,
    /// The arguments.
    pub args: Args<'a>,
    object: Option<&'a HostObject>,
}

impl Invocation<'_> {
    /// The member name as described.
    pub fn name(&self) -> &str {
        &self.sig.name
    }

    /// Raises an event of the object's class to the connected sinks (see
    /// [`HostObject::raise_event`]). A global function has no events.
    pub fn raise_event(&self, event: &str, args: &mut [Arg]) -> Result<(), DispError> {
        match self.object {
            Some(o) => o.raise_event(self.table, event, args),
            None => Err(DispError::MEMBER_NOT_FOUND),
        }
    }
}

/// The behavior behind a described class or the global functions.
///
/// Implement it directly (it receives every call and matches on
/// [`Invocation::name`] and [`Invocation::kind`]), pass a closure, or use
/// [`Handlers`] for a table of per-member closures that is checked against
/// the description.
pub trait HostImpl {
    /// Runs one call. An object result must carry one reference for the
    /// caller (`table.insert(..)` returns a handle that does).
    fn invoke(&self, cx: &mut Invocation<'_>) -> Result<Variant, DispError>;

    /// Called when the last reference is released.
    fn terminate(&self, _table: &ObjectTable) {}

    /// References the object owns, released after `terminate`.
    fn held_references(&self) -> Vec<ObjectHandle> {
        Vec::new()
    }

    /// Whether the implementation calls back into the running session (see
    /// `Dispatch::reentrant`).
    fn reentrant(&self) -> bool {
        false
    }

    /// The `(member, kind)` pairs the implementation handles, when it can
    /// say. [`HostObject::new`] then rejects an implementation that lacks a
    /// described member or handles an undescribed one.
    fn handles(&self) -> Option<Vec<(String, CallKind)>> {
        None
    }
}

impl<F> HostImpl for F
where
    F: Fn(&mut Invocation<'_>) -> Result<Variant, DispError>,
{
    fn invoke(&self, cx: &mut Invocation<'_>) -> Result<Variant, DispError> {
        self(cx)
    }
}

type Handler<S> = Box<dyn Fn(&S, &mut Invocation<'_>) -> Result<Variant, DispError>>;
type TerminateFn<S> = Box<dyn Fn(&S, &ObjectTable)>;
type HeldFn<S> = Box<dyn Fn(&S) -> Vec<ObjectHandle>>;

/// A [`HostImpl`] made of one closure per member and kind of call, around a
/// state value. [`HostObject::new`] checks the table against the
/// description: every described member needs a handler, and a handler may
/// not name an undescribed member or an unsupported kind.
///
/// The state is shared by reference (`&S`); use `Cell`/`RefCell` inside it
/// for mutation.
pub struct Handlers<S> {
    state: S,
    table: HashMap<(String, CallKind), Handler<S>>,
    order: Vec<(String, CallKind)>,
    on_terminate: Option<TerminateFn<S>>,
    held: Option<HeldFn<S>>,
    reentrant: bool,
}

impl<S: 'static> Handlers<S> {
    /// An empty table around `state`.
    pub fn new(state: S) -> Handlers<S> {
        Handlers {
            state,
            table: HashMap::new(),
            order: Vec::new(),
            on_terminate: None,
            held: None,
            reentrant: false,
        }
    }

    fn add(
        mut self,
        name: &str,
        kind: CallKind,
        f: impl Fn(&S, &mut Invocation<'_>) -> Result<Variant, DispError> + 'static,
    ) -> Self {
        let k = (key(name), kind);
        if self.table.insert(k.clone(), Box::new(f)).is_none() {
            self.order.push((name.to_owned(), kind));
        }
        self
    }

    /// The `Sub` or `Function` `name`.
    #[must_use]
    pub fn method(
        self,
        name: &str,
        f: impl Fn(&S, &mut Invocation<'_>) -> Result<Variant, DispError> + 'static,
    ) -> Self {
        self.add(name, CallKind::Call, f)
    }

    /// The `Property Get` of `name`.
    #[must_use]
    pub fn get(
        self,
        name: &str,
        f: impl Fn(&S, &mut Invocation<'_>) -> Result<Variant, DispError> + 'static,
    ) -> Self {
        self.add(name, CallKind::Get, f)
    }

    /// The `Property Let` of `name`; the value is `cx.args.value()`.
    #[must_use]
    pub fn let_(
        self,
        name: &str,
        f: impl Fn(&S, &mut Invocation<'_>) -> Result<(), DispError> + 'static,
    ) -> Self {
        self.add(name, CallKind::Let, move |s, cx| {
            f(s, cx).map(|()| Variant::Empty)
        })
    }

    /// The `Property Set` of `name`.
    #[must_use]
    pub fn set(
        self,
        name: &str,
        f: impl Fn(&S, &mut Invocation<'_>) -> Result<(), DispError> + 'static,
    ) -> Self {
        self.add(name, CallKind::Set, move |s, cx| {
            f(s, cx).map(|()| Variant::Empty)
        })
    }

    /// `For Each`: the items of the collection. Object items carry a
    /// reference that the enumerator takes over.
    #[must_use]
    pub fn enumerate(self, f: impl Fn(&S, &ObjectTable) -> Vec<Variant> + 'static) -> Self {
        self.add("_NewEnum", CallKind::NewEnum, move |s, cx| {
            let en = VecEnum::new(f(s, cx.table));
            Ok(Variant::Object(ObjectRef::Handle(cx.table.insert_obj(en))))
        })
    }

    /// Runs when the last reference is released.
    #[must_use]
    pub fn on_terminate(mut self, f: impl Fn(&S, &ObjectTable) + 'static) -> Self {
        self.on_terminate = Some(Box::new(f));
        self
    }

    /// The references the object owns (released after termination).
    #[must_use]
    pub fn holds(mut self, f: impl Fn(&S) -> Vec<ObjectHandle> + 'static) -> Self {
        self.held = Some(Box::new(f));
        self
    }

    /// Marks the object re-entrant.
    #[must_use]
    pub fn reentrant(mut self) -> Self {
        self.reentrant = true;
        self
    }

    /// The state.
    pub fn state(&self) -> &S {
        &self.state
    }
}

impl<S: 'static> HostImpl for Handlers<S> {
    fn invoke(&self, cx: &mut Invocation<'_>) -> Result<Variant, DispError> {
        let k = (key(cx.name()), cx.kind);
        match self.table.get(&k) {
            Some(h) => h(&self.state, cx),
            None => Err(DispError::MEMBER_NOT_FOUND),
        }
    }

    fn terminate(&self, table: &ObjectTable) {
        if let Some(f) = &self.on_terminate {
            f(&self.state, table);
        }
    }

    fn held_references(&self) -> Vec<ObjectHandle> {
        self.held.as_ref().map_or_else(Vec::new, |f| f(&self.state))
    }

    fn reentrant(&self) -> bool {
        self.reentrant
    }

    fn handles(&self) -> Option<Vec<(String, CallKind)>> {
        Some(self.order.clone())
    }
}

/// The items of a collection as an enumerator object.
struct VecEnum {
    items: Vec<Variant>,
    pos: std::cell::Cell<usize>,
}

impl VecEnum {
    fn new(items: Vec<Variant>) -> VecEnum {
        VecEnum {
            items,
            pos: std::cell::Cell::new(0),
        }
    }
}

impl VariantEnum for VecEnum {
    fn next(&self, table: &ObjectTable) -> Result<Option<Variant>, DispError> {
        let i = self.pos.get();
        let Some(v) = self.items.get(i) else {
            return Ok(None);
        };
        self.pos.set(i + 1);
        table.add_ref_value(v);
        Ok(Some(v.clone()))
    }

    fn reset(&self) -> Result<(), DispError> {
        self.pos.set(0);
        Ok(())
    }
}

impl Dispatch for VecEnum {
    fn class_name(&self) -> String {
        "Enumerator".into()
    }
    fn member_id(&self, _: &str) -> Option<DispId> {
        None
    }
    fn member_info(&self, _: DispId) -> Option<MemberInfo> {
        None
    }
    fn invoke(
        &self,
        _: &ObjectTable,
        _: DispId,
        _: InvokeFlags,
        _: &mut DispParams,
    ) -> Result<Variant, DispError> {
        Err(DispError::MEMBER_NOT_FOUND)
    }
    fn held_references(&self) -> Vec<ObjectHandle> {
        self.items
            .iter()
            .filter_map(|v| match v {
                Variant::Object(ObjectRef::Handle(h)) => Some(*h),
                _ => None,
            })
            .collect()
    }
    fn enumerator(&self) -> Option<&dyn VariantEnum> {
        Some(self)
    }
}

/// An automation object of a described class.
pub struct HostObject {
    lib: CheckedLibrary,
    class: usize,
    imp: Box<dyn HostImpl>,
    events: Option<ConnectionPoint>,
    new_enum: Option<MemberSig>,
}

fn required_kinds(sig: &MemberSig) -> Vec<CallKind> {
    let mut v = Vec::new();
    if sig
        .supports
        .intersects(InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET)
    {
        v.push(match sig.kind {
            vba_sema::LibMemberKind::Method => CallKind::Call,
            _ => CallKind::Get,
        });
    }
    v
}

fn put_kinds(sig: &MemberSig) -> Vec<CallKind> {
    let mut v = Vec::new();
    if sig.supports.contains(InvokeFlags::PROPERTY_PUT) {
        v.push(CallKind::Let);
    }
    if sig.supports.contains(InvokeFlags::PROPERTY_PUT_REF) {
        v.push(CallKind::Set);
    }
    v
}

impl HostObject {
    /// An object of `class` behaving as `imp`.
    ///
    /// When `imp` can list what it handles (see [`Handlers`]) the list is
    /// checked against the description: a described member without a
    /// handler is [`BindError::Unimplemented`], a handler for an unknown
    /// member is [`BindError::UnknownMember`], and one for a kind of call
    /// the member does not support is [`BindError::KindNotSupported`].
    pub fn new(
        lib: &CheckedLibrary,
        class: &str,
        imp: impl HostImpl + 'static,
    ) -> Result<HostObject, BindError> {
        let ci = lib
            .class_index(class)
            .ok_or_else(|| BindError::UnknownClass(class.to_owned()))?;
        let c = &lib.library().classes[ci];
        if let Some(handled) = imp.handles() {
            for (name, kind) in &handled {
                if *kind == CallKind::NewEnum {
                    if c.enumerable.is_none() {
                        return Err(BindError::UnknownMember {
                            class: c.name.clone(),
                            member: name.clone(),
                        });
                    }
                    continue;
                }
                let sig =
                    lib.class_member(&c.name, name)
                        .ok_or_else(|| BindError::UnknownMember {
                            class: c.name.clone(),
                            member: name.clone(),
                        })?;
                let ok = required_kinds(sig).contains(kind) || put_kinds(sig).contains(kind);
                if !ok {
                    return Err(BindError::KindNotSupported {
                        class: c.name.clone(),
                        member: sig.name.clone(),
                        kind: *kind,
                    });
                }
            }
            let has = |n: &str, k: CallKind| {
                handled
                    .iter()
                    .any(|(hn, hk)| *hk == k && names_equal(hn, n))
            };
            for sig in &lib.inner.class_members[ci] {
                for k in required_kinds(sig) {
                    if !has(&sig.name, k) {
                        return Err(BindError::Unimplemented {
                            class: c.name.clone(),
                            member: sig.name.clone(),
                            kind: k,
                        });
                    }
                }
                let puts = put_kinds(sig);
                if !puts.is_empty() && !puts.iter().any(|k| has(&sig.name, *k)) {
                    return Err(BindError::Unimplemented {
                        class: c.name.clone(),
                        member: sig.name.clone(),
                        kind: puts[0],
                    });
                }
            }
            if c.enumerable.is_some() && !has("_NewEnum", CallKind::NewEnum) {
                return Err(BindError::Unimplemented {
                    class: c.name.clone(),
                    member: "_NewEnum".into(),
                    kind: CallKind::NewEnum,
                });
            }
        }
        let new_enum = c.enumerable.as_ref().map(|_| MemberSig {
            name: "_NewEnum".into(),
            id: DispId::NEWENUM,
            supports: InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
            kind: vba_sema::LibMemberKind::PropertyGet,
            params: Vec::new(),
            ty: Type::Object,
            class: None,
            is_sub: false,
            is_default: false,
            hidden: true,
            infos: Vec::new(),
        });
        Ok(HostObject {
            lib: lib.clone(),
            class: ci,
            imp: Box::new(imp),
            events: (!c.events.is_empty())
                .then(|| ConnectionPoint::new(&format!("{}Events", c.name))),
            new_enum,
        })
    }

    /// The description this object was built from.
    pub fn library(&self) -> &CheckedLibrary {
        &self.lib
    }

    /// Raises `event` to the connected `WithEvents` sinks, in order.
    /// `args` must have one entry per described parameter; entries for
    /// `ByRef` parameters should be passed by reference so that handlers
    /// can change them (`Cancel`).
    pub fn raise_event(
        &self,
        table: &ObjectTable,
        event: &str,
        args: &mut [Arg],
    ) -> Result<(), DispError> {
        let sig = self.lib.inner.class_events[self.class]
            .iter()
            .find(|e| names_equal(&e.name, event))
            .ok_or(DispError::MEMBER_NOT_FOUND)?;
        if args.len() != sig.params.len() {
            return Err(DispError::BAD_PARAM_COUNT);
        }
        match &self.events {
            Some(cp) => cp.fire(table, &sig.name, args),
            None => Ok(()),
        }
    }

    fn sig_by_id(&self, id: DispId) -> Option<&MemberSig> {
        if id == DispId::NEWENUM {
            return self.new_enum.as_ref();
        }
        let j = *self.lib.inner.class_id_ix[self.class].get(&id.0)?;
        Some(&self.lib.inner.class_members[self.class][j])
    }
}

impl Dispatch for HostObject {
    fn class_name(&self) -> String {
        self.lib.library().classes[self.class].name.clone()
    }

    fn member_id(&self, name: &str) -> Option<DispId> {
        if self.new_enum.is_some() && names_equal(name, "_NewEnum") {
            return Some(DispId::NEWENUM);
        }
        let j = *self.lib.inner.class_member_ix[self.class].get(&key(name))?;
        Some(self.lib.inner.class_members[self.class][j].id)
    }

    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        self.sig_by_id(id).map(MemberSig::to_member_info)
    }

    fn accepts_dispid(&self, id: DispId, name: &str) -> bool {
        self.sig_by_id(id)
            .is_some_and(|s| names_equal(&s.name, name))
    }

    fn invoke(
        &self,
        table: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let sig = self.sig_by_id(id).ok_or(DispError::MEMBER_NOT_FOUND)?;
        let class = &self.lib.library().classes[self.class].name;
        run_member(sig, class, &*self.imp, Some(self), table, flags, params)
    }

    fn terminate(&self, table: &ObjectTable) {
        self.imp.terminate(table);
    }

    fn held_references(&self) -> Vec<ObjectHandle> {
        self.imp.held_references()
    }

    fn event_source(&self) -> Option<&dyn EventSource> {
        self.events.as_ref().map(|c| c as &dyn EventSource)
    }

    fn reentrant(&self) -> bool {
        self.imp.reentrant()
    }
}

/// Converts `v` to the declared type as an assignment would. A class type
/// takes an object of that class (or `Nothing`).
pub(crate) fn coerce(
    table: &ObjectTable,
    ty: &Type,
    class: Option<&str>,
    v: &Variant,
) -> Result<Variant, DispError> {
    match ty {
        Type::Variant | Type::Any | Type::Error => Ok(v.clone()),
        Type::Array { elem, rank } => {
            let Variant::Array(a) = v else {
                return Err(DispError::TYPE_MISMATCH);
            };
            if rank.is_some_and(|r| usize::from(r) != a.dimensions()) {
                return Err(vba_value::VbaError::SUBSCRIPT_OUT_OF_RANGE.into());
            }
            match elem.var_type() {
                Some(vt) if vt != a.element_type() && vt != VarType::Variant => {
                    Err(DispError::TYPE_MISMATCH)
                }
                _ => Ok(v.clone()),
            }
        }
        Type::Library(_) | Type::Object => match v {
            Variant::Object(ObjectRef::Nothing) => Ok(v.clone()),
            Variant::Object(ObjectRef::Handle(h)) => {
                if let Some(want) = class
                    && h.0 & FOREIGN_HANDLE_BIT == 0
                    && !names_equal(&table.get(*h)?.class_name(), want)
                {
                    return Err(DispError::TYPE_MISMATCH);
                }
                Ok(v.clone())
            }
            _ => Err(DispError::TYPE_MISMATCH),
        },
        t => match t.var_type() {
            Some(vt) => Ok(convert::convert_to(v, vt)?),
            None => Ok(v.clone()),
        },
    }
}

/// The one place a call is bound, converted, run and written back. Objects
/// and global functions both come through here.
pub(crate) fn run_member(
    sig: &MemberSig,
    class: &str,
    imp: &dyn HostImpl,
    object: Option<&HostObject>,
    table: &ObjectTable,
    flags: InvokeFlags,
    params: &mut DispParams,
) -> Result<Variant, DispError> {
    let write = flags.is_write();
    let wanted = if write {
        flags & (InvokeFlags::PROPERTY_PUT | InvokeFlags::PROPERTY_PUT_REF)
    } else {
        InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET
    };
    if !sig.supports.intersects(wanted) {
        return Err(DispError::MEMBER_NOT_FOUND);
    }
    let mut bound = vba_automation::bind(&sig.infos, params)?;
    let original: Vec<Variant> = bound.slots.iter().map(|a| a.value.clone()).collect();
    let fixed = sig.params.iter().filter(|p| !p.param_array);
    for (slot, p) in bound.slots.iter_mut().zip(fixed) {
        if !slot.is_missing() {
            slot.value = coerce(table, &p.ty, p.class.as_deref(), &slot.value)?;
        }
    }
    let coerced: Vec<Variant> = bound.slots.iter().map(|a| a.value.clone()).collect();
    let kind = if write {
        match &bound.value {
            Some(v) => {
                bound.value = Some(coerce(table, &sig.ty, sig.class.as_deref(), v)?);
            }
            None => return Err(DispError::PARAM_NOT_OPTIONAL),
        }
        if flags.contains(InvokeFlags::PROPERTY_PUT_REF) {
            CallKind::Set
        } else {
            CallKind::Let
        }
    } else if sig.id == DispId::NEWENUM {
        CallKind::NewEnum
    } else if sig.kind == vba_sema::LibMemberKind::Method {
        CallKind::Call
    } else {
        CallKind::Get
    };
    let mut cx = Invocation {
        table,
        kind,
        sig,
        class,
        args: Args { sig, bound },
        object,
    };
    let result = imp.invoke(&mut cx)?;
    let mut bound = cx.args.bound;
    // Only parameters the implementation assigned flow back; an untouched
    // one keeps the caller's own value, not its converted copy.
    for (i, slot) in bound.slots.iter_mut().enumerate() {
        if slot.value == coerced[i] {
            slot.value = original[i].clone();
        }
    }
    bound.write_back(params);
    if sig.is_sub || write || sig.id == DispId::NEWENUM {
        return Ok(result);
    }
    coerce(table, &sig.ty, sig.class.as_deref(), &result)
}
