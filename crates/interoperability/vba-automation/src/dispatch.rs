//! The `IDispatch` abstraction: ids, invoke flags, parameters and the
//! [`Dispatch`] trait (MS-OAUT, IDispatch).

use crate::enumerate::VariantEnum;
use crate::error::DispError;
use crate::events::EventSource;
use crate::hresult;
use crate::table::ObjectTable;
use bitflags::bitflags;
use std::rc::Rc;
use vba_core::VbaError;
use vba_value::{ObjectHandle, Variant};
use windows_sys::Win32::System::{Com as com, Ole as ole};

/// A member identifier (`DISPID`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DispId(pub i32);

impl DispId {
    /// `DISPID_VALUE`: the default member.
    pub const VALUE: DispId = DispId(ole::DISPID_VALUE as i32);
    /// `DISPID_UNKNOWN`: name not resolved.
    pub const UNKNOWN: DispId = DispId(ole::DISPID_UNKNOWN);
    /// `DISPID_PROPERTYPUT`: the named argument carrying the assigned value.
    pub const PROPERTYPUT: DispId = DispId(ole::DISPID_PROPERTYPUT);
    /// `DISPID_NEWENUM`: the `_NewEnum` member used by `For Each`.
    pub const NEWENUM: DispId = DispId(ole::DISPID_NEWENUM);
    /// `DISPID_EVALUATE`: the `[ ]` evaluate member.
    pub const EVALUATE: DispId = DispId(ole::DISPID_EVALUATE);
    /// `DISPID_CONSTRUCTOR`.
    pub const CONSTRUCTOR: DispId = DispId(ole::DISPID_CONSTRUCTOR);
    /// `DISPID_DESTRUCTOR`.
    pub const DESTRUCTOR: DispId = DispId(ole::DISPID_DESTRUCTOR);
    /// `DISPID_COLLECT`.
    pub const COLLECT: DispId = DispId(ole::DISPID_COLLECT);
}

bitflags! {
    /// `Invoke` flags (`DISPATCH_*`), combinable with `|`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct InvokeFlags: u16 {
        /// No flags.
        const NONE = 0;
        /// `DISPATCH_METHOD`.
        const METHOD = com::DISPATCH_METHOD;
        /// `DISPATCH_PROPERTYGET`.
        const PROPERTY_GET = com::DISPATCH_PROPERTYGET;
        /// `DISPATCH_PROPERTYPUT` (Let).
        const PROPERTY_PUT = com::DISPATCH_PROPERTYPUT;
        /// `DISPATCH_PROPERTYPUTREF` (Set).
        const PROPERTY_PUT_REF = com::DISPATCH_PROPERTYPUTREF;
    }
}

impl InvokeFlags {
    const READ_BITS: InvokeFlags = InvokeFlags::METHOD.union(InvokeFlags::PROPERTY_GET);
    const WRITE_BITS: InvokeFlags = InvokeFlags::PROPERTY_PUT.union(InvokeFlags::PROPERTY_PUT_REF);

    /// `true` for a read (Method and/or PropertyGet, no put bit).
    pub const fn is_read(self) -> bool {
        self.intersects(Self::READ_BITS) && !self.intersects(Self::WRITE_BITS)
    }

    /// `true` for PropertyPut or PropertyPutRef.
    pub const fn is_write(self) -> bool {
        self.intersects(Self::WRITE_BITS)
    }
}

/// One argument: a value and whether the caller passed a variable ByRef.
///
/// Missing optional arguments are `Variant::Error(DISP_E_PARAMNOTFOUND)`, the
/// OLE convention; see [`Arg::missing`].
#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    /// The value (updated by the callee for ByRef parameters).
    pub value: Variant,
    /// `true` if the caller passed a variable and expects write-back.
    pub by_ref: bool,
}

impl Arg {
    /// A by-value argument.
    pub fn val(value: impl Into<Variant>) -> Self {
        Arg {
            value: value.into(),
            by_ref: false,
        }
    }

    /// A by-reference argument (the caller reads `value` back after the call).
    pub fn by_ref(value: impl Into<Variant>) -> Self {
        Arg {
            value: value.into(),
            by_ref: true,
        }
    }

    /// The omitted-argument marker.
    pub fn missing() -> Self {
        Arg::val(Variant::Error(VbaError::new(hresult::DISP_E_PARAMNOTFOUND)))
    }

    /// `true` if this is the omitted-argument marker (`IsMissing`).
    pub fn is_missing(&self) -> bool {
        matches!(&self.value, Variant::Error(e) if e.number() == hresult::DISP_E_PARAMNOTFOUND)
    }
}

/// `DISPPARAMS`: positional arguments in source (left-to-right) order, plus
/// named arguments identified by `DispId`.
///
/// The OLE wire format reverses positional arguments; this model keeps them in
/// source order. For PropertyPut/PropertyPutRef the assigned value is the
/// named argument `DISPID_PROPERTYPUT`. Named argument ids are parameter
/// indices (from `GetIDsOfNames`, 0-based).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DispParams {
    /// Positional arguments.
    pub args: Vec<Arg>,
    /// Named arguments.
    pub named: Vec<(DispId, Arg)>,
}

impl DispParams {
    /// No arguments.
    pub fn new() -> Self {
        Self::default()
    }

    /// The value assigned by a property put, if present.
    pub fn put_value(&self) -> Option<&Variant> {
        self.named
            .iter()
            .find(|(id, _)| *id == DispId::PROPERTYPUT)
            .map(|(_, a)| &a.value)
    }
}

/// Declared parameter of a member.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamInfo {
    /// Parameter name (matched case-insensitively for named arguments).
    pub name: String,
    /// `Optional`.
    pub optional: bool,
    /// Declared `ByRef` (the VBA default). `ByVal` parameters never write
    /// back.
    pub by_ref: bool,
    /// `ParamArray` (only valid as the last parameter).
    pub param_array: bool,
    /// Default for an omitted optional parameter; `None` leaves the missing
    /// marker.
    pub default: Option<Variant>,
}

impl ParamInfo {
    /// A required ByRef parameter.
    pub fn required(name: &str) -> Self {
        ParamInfo {
            name: name.to_string(),
            optional: false,
            by_ref: true,
            param_array: false,
            default: None,
        }
    }

    /// An optional ByRef parameter.
    pub fn optional(name: &str) -> Self {
        ParamInfo {
            optional: true,
            ..Self::required(name)
        }
    }

    /// A `ParamArray` parameter.
    pub fn param_array(name: &str) -> Self {
        ParamInfo {
            optional: true,
            param_array: true,
            ..Self::required(name)
        }
    }

    /// Marks the parameter ByVal.
    pub fn by_val(mut self) -> Self {
        self.by_ref = false;
        self
    }

    /// Sets the default value.
    pub fn with_default(mut self, v: Variant) -> Self {
        self.default = Some(v);
        self
    }
}

/// Type information about one member (the subset of `ITypeInfo` VBA's call
/// resolution needs).
#[derive(Debug, Clone, PartialEq)]
pub struct MemberInfo {
    /// Member name.
    pub name: String,
    /// The id.
    pub id: DispId,
    /// Which invoke kinds the member supports (any of the flag bits).
    pub supports: InvokeFlags,
    /// Declared parameters (excluding the value of a property put).
    pub params: Vec<ParamInfo>,
}

impl MemberInfo {
    /// Number of declared non-ParamArray parameters.
    pub fn fixed_param_count(&self) -> usize {
        self.params.iter().filter(|p| !p.param_array).count()
    }

    /// `true` if the last parameter is a ParamArray.
    pub fn has_param_array(&self) -> bool {
        self.params.last().is_some_and(|p| p.param_array)
    }
}

/// Case-insensitive identifier comparison, as VBA names are matched.
pub fn names_equal(a: &str, b: &str) -> bool {
    a.chars()
        .flat_map(char::to_lowercase)
        .eq(b.chars().flat_map(char::to_lowercase))
}

/// Arguments matched against declared parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct Bound {
    /// One entry per declared non-ParamArray parameter; omitted optional
    /// parameters hold their default or the missing marker.
    pub slots: Vec<Arg>,
    /// ParamArray elements.
    pub rest: Vec<Arg>,
    /// The assigned value of a property put.
    pub value: Option<Variant>,
    origin_slots: Vec<Option<Origin>>,
    origin_rest: Vec<usize>,
    slot_by_ref: Vec<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Origin {
    Positional(usize),
    Named(usize),
}

impl Bound {
    /// Copies ByRef results back into the caller's `DispParams`.
    ///
    /// Only arguments the caller passed ByRef and the callee declared ByRef
    /// (ParamArray elements are always ByRef) are written back.
    pub fn write_back(&self, dp: &mut DispParams) {
        for (i, o) in self.origin_slots.iter().enumerate() {
            if !self.slot_by_ref[i] {
                continue;
            }
            let target = match o {
                Some(Origin::Positional(p)) => dp.args.get_mut(*p),
                Some(Origin::Named(n)) => dp.named.get_mut(*n).map(|(_, a)| a),
                None => None,
            };
            if let Some(a) = target
                && a.by_ref
            {
                a.value = self.slots[i].value.clone();
            }
        }
        for (k, p) in self.origin_rest.iter().enumerate() {
            if let Some(a) = dp.args.get_mut(*p)
                && a.by_ref
            {
                a.value = self.rest[k].value.clone();
            }
        }
    }
}

/// Matches `dp` against `params` following the Automation rules:
/// positional arguments fill parameters left to right, extra positionals go to
/// a trailing ParamArray, named arguments (ids are parameter indices) fill the
/// rest, and omitted optional parameters take their default.
///
/// Errors: too many arguments is `DISP_E_BADPARAMCOUNT`; a named argument out
/// of range or naming an already-filled parameter is `DISP_E_PARAMNOTFOUND`;
/// an omitted required parameter is `DISP_E_PARAMNOTOPTIONAL`.
pub fn bind(params: &[ParamInfo], dp: &DispParams) -> Result<Bound, DispError> {
    let fixed: Vec<&ParamInfo> = params.iter().filter(|p| !p.param_array).collect();
    let has_pa = params.last().is_some_and(|p| p.param_array);
    let mut slots: Vec<Option<(Arg, Origin)>> = vec![None; fixed.len()];
    let mut rest = Vec::new();
    let mut origin_rest = Vec::new();
    for (i, a) in dp.args.iter().enumerate() {
        if i < fixed.len() {
            slots[i] = Some((a.clone(), Origin::Positional(i)));
        } else if has_pa {
            rest.push(a.clone());
            origin_rest.push(i);
        } else {
            return Err(DispError::BAD_PARAM_COUNT);
        }
    }
    let mut value = None;
    for (n, (id, a)) in dp.named.iter().enumerate() {
        if *id == DispId::PROPERTYPUT {
            value = Some(a.value.clone());
            continue;
        }
        let idx = usize::try_from(id.0).map_err(|_| DispError::PARAM_NOT_FOUND)?;
        match slots.get_mut(idx) {
            Some(s) if s.is_none() => *s = Some((a.clone(), Origin::Named(n))),
            _ => return Err(DispError::PARAM_NOT_FOUND),
        }
    }
    let mut out = Vec::with_capacity(fixed.len());
    let mut origin_slots = Vec::with_capacity(fixed.len());
    let mut slot_by_ref = Vec::with_capacity(fixed.len());
    for (p, s) in fixed.iter().zip(slots) {
        slot_by_ref.push(p.by_ref);
        match s {
            Some((a, o)) if !a.is_missing() => {
                out.push(a);
                origin_slots.push(Some(o));
            }
            _ => {
                if !p.optional {
                    return Err(DispError::PARAM_NOT_OPTIONAL);
                }
                out.push(match &p.default {
                    Some(d) => Arg::val(d.clone()),
                    None => Arg::missing(),
                });
                origin_slots.push(None);
            }
        }
    }
    Ok(Bound {
        slots: out,
        rest,
        value,
        origin_slots,
        origin_rest,
        slot_by_ref,
    })
}

/// An automation object (`IDispatch`).
///
/// Methods take `&self`: objects use interior mutability, because invocations
/// re-enter the [`ObjectTable`] (an object's code may touch other objects, or
/// itself). Every method receives the table to create or reference objects.
pub trait Dispatch {
    /// The class name, used as the default `Err.Source` and by `TypeName`.
    fn class_name(&self) -> String;

    /// Resolves one member name (case-insensitive) to its id.
    fn member_id(&self, name: &str) -> Option<DispId>;

    /// Type information for a member, when available. Without it the call
    /// helper cannot split surplus arguments off to a default member, cannot
    /// treat ByVal parameters specially and assumes every argument is ByRef.
    fn member_info(&self, id: DispId) -> Option<MemberInfo>;

    /// Whether `id` is the member id of the member called `name` on this
    /// object, so that a caller that learned the id from a type library
    /// (early binding) may invoke by id and skip the name lookup. The
    /// default compares the name in [`member_info`](Dispatch::member_info);
    /// objects that implement it cheaply should override this. An object
    /// whose ids are private answers `false` and is called by name.
    fn accepts_dispid(&self, id: DispId, name: &str) -> bool {
        self.member_info(id)
            .is_some_and(|i| names_equal(&i.name, name))
    }

    /// `GetIDsOfNames`: `names[0]` is the member, the others are named
    /// parameters, resolved to their 0-based index through
    /// [`Dispatch::member_info`]. Any unknown name is `DISP_E_UNKNOWNNAME`.
    fn get_ids_of_names(&self, names: &[&str]) -> Result<Vec<DispId>, DispError> {
        let (first, others) = names.split_first().ok_or(DispError::UNKNOWN_NAME)?;
        let id = self.member_id(first).ok_or(DispError::UNKNOWN_NAME)?;
        let mut ids = vec![id];
        if !others.is_empty() {
            let info = self.member_info(id).ok_or(DispError::UNKNOWN_NAME)?;
            for n in others {
                let idx = info
                    .params
                    .iter()
                    .position(|p| names_equal(&p.name, n))
                    .ok_or(DispError::UNKNOWN_NAME)?;
                ids.push(DispId(idx as i32));
            }
        }
        Ok(ids)
    }

    /// `Invoke`. `params` is mutable so ByRef arguments can be updated; the
    /// call helper discards changes to by-value arguments. An object returned
    /// in the result carries one reference owned by the caller.
    fn invoke(
        &self,
        table: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError>;

    /// Called once when the last reference is released, before the object is
    /// removed (`Class_Terminate`). The object may still be invoked and may
    /// take new references to itself ("resurrection").
    fn terminate(&self, _table: &ObjectTable) {}

    /// References the object owns; released, in order, after `terminate`.
    fn held_references(&self) -> Vec<ObjectHandle> {
        Vec::new()
    }

    /// The enumeration interface, for objects that serve as the result of
    /// `DISPID_NEWENUM`.
    fn enumerator(&self) -> Option<&dyn VariantEnum> {
        None
    }

    /// The connection-point container, for objects that raise events.
    fn event_source(&self) -> Option<&dyn EventSource> {
        None
    }
}

/// Shared pointer to an object as stored in the table.
pub type DispatchRef = Rc<dyn Dispatch>;
