//! Per-interface dispatch tables for `vba-automation` objects.
//!
//! An [`InterfaceTable`] is built from the members a type library declares
//! for one interface (or for the default interface of a coclass). It maps a
//! member name (case-insensitively) to its `DISPID`, and describes the member
//! for the call machinery of `vba-automation`: the invoke kinds it supports,
//! the parameters with their declared types, `ByRef`/`Optional`/`ParamArray`
//! flags and default values, and the result type. An object that implements
//! [`vba_automation::Dispatch`] for such an interface delegates to its table:
//!
//! * `member_id`, `member_info`, `get_ids_of_names` (named arguments resolve
//!   to parameter indices) and `accepts_dispid` answer from the table;
//! * [`InterfaceTable::bind_args`] matches an `Invoke`'s arguments to the
//!   declared parameters (omitted optional ones take the declared default)
//!   and converts every argument to its declared type with VBA's
//!   conversions, so the implementation sees typed values: a `String` where
//!   a `Long` is declared is a type mismatch (13) before the member runs; an
//!   array of the wrong rank is a subscript error (9).
//!
//! The table is what makes early binding sound on the host side: the ids a
//! compiled call carries are the ids this table answers to.

use std::collections::HashMap;

use vba_automation::{Bound, DispError, DispId, DispParams, InvokeFlags, MemberInfo, ParamInfo};
use vba_sema::{LibMember, LibMemberKind, LibraryType, Type, TypeProvider};
use vba_value::{Variant, VbaError, convert};

use crate::sema::TypelibProvider;

/// A declared parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamDesc {
    /// Name, matched case-insensitively for named arguments.
    pub name: String,
    /// Declared type.
    pub ty: Type,
    /// `Optional`.
    pub optional: bool,
    /// Declared `ByRef`: the callee may update the caller's variable.
    pub by_ref: bool,
    /// `ParamArray`.
    pub param_array: bool,
    /// Declared default for an omitted optional argument.
    pub default: Option<Variant>,
}

/// A member of an interface.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberDesc {
    /// Name as the library spells it.
    pub name: String,
    /// The member id.
    pub id: DispId,
    /// Method, or the property accessors that exist.
    pub kind: LibMemberKind,
    /// The invoke kinds the member supports.
    pub supports: InvokeFlags,
    /// Parameters (without the value of a property put).
    pub params: Vec<ParamDesc>,
    /// Result type, or the value type of a property.
    pub ty: Type,
    /// Whether the member is a `Sub` (no result).
    pub is_sub: bool,
    /// Whether this is the default member (id 0).
    pub is_default: bool,
}

impl MemberDesc {
    /// The `vba-automation` description of the member.
    pub fn info(&self) -> MemberInfo {
        MemberInfo::new(
            self.name.clone(),
            self.id,
            self.supports,
            self.params
                .iter()
                .map(|p| {
                    ParamInfo::required(&p.name)
                        .with_optional(p.optional)
                        .with_by_ref(p.by_ref)
                        .with_param_array(p.param_array)
                        .with_default_value(p.default.clone())
                })
                .collect(),
        )
    }
}

/// The members of one interface, by name and by id.
#[derive(Debug, Clone)]
pub struct InterfaceTable {
    ty: LibraryType,
    members: Vec<MemberDesc>,
    by_name: HashMap<String, usize>,
    by_id: HashMap<i32, usize>,
}

impl InterfaceTable {
    /// Builds a table from the members of `ty`. Members without an id are
    /// not dispatchable and are left out.
    pub fn from_members(ty: LibraryType, members: Vec<LibMember>) -> InterfaceTable {
        let mut t = InterfaceTable {
            ty,
            members: Vec::new(),
            by_name: HashMap::new(),
            by_id: HashMap::new(),
        };
        for m in members {
            let Some(id) = m.dispid else { continue };
            let supports = match m.kind {
                LibMemberKind::Method | LibMemberKind::PropertyGet => {
                    InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET
                }
                LibMemberKind::Property => {
                    InvokeFlags::METHOD
                        | InvokeFlags::PROPERTY_GET
                        | InvokeFlags::PROPERTY_PUT
                        | InvokeFlags::PROPERTY_PUT_REF
                }
                LibMemberKind::PropertyPut => {
                    InvokeFlags::PROPERTY_PUT | InvokeFlags::PROPERTY_PUT_REF
                }
            };
            let desc = MemberDesc {
                name: m.name,
                id: DispId(id),
                kind: m.kind,
                supports,
                params: m
                    .params
                    .into_iter()
                    .map(|p| ParamDesc {
                        name: p.name,
                        ty: p.ty,
                        optional: p.optional,
                        by_ref: p.by_ref,
                        param_array: p.param_array,
                        default: p.default,
                    })
                    .collect(),
                ty: m.ty,
                is_sub: m.is_sub,
                is_default: m.is_default,
            };
            let i = t.members.len();
            t.by_name.entry(desc.name.to_ascii_lowercase()).or_insert(i);
            t.by_id.entry(id).or_insert(i);
            t.members.push(desc);
        }
        t
    }

    /// The type the table describes.
    pub fn type_name(&self) -> &LibraryType {
        &self.ty
    }

    /// All members in the library's order.
    pub fn members(&self) -> &[MemberDesc] {
        &self.members
    }

    /// A member by name (case-insensitive).
    pub fn by_name(&self, name: &str) -> Option<&MemberDesc> {
        self.by_name
            .get(&name.to_ascii_lowercase())
            .map(|i| &self.members[*i])
    }

    /// A member by id.
    pub fn by_id(&self, id: DispId) -> Option<&MemberDesc> {
        self.by_id.get(&id.0).map(|i| &self.members[*i])
    }

    /// `GetIDsOfNames` for the member alone.
    pub fn member_id(&self, name: &str) -> Option<DispId> {
        self.by_name(name).map(|m| m.id)
    }

    /// The `vba-automation` description of a member.
    pub fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        self.by_id(id).map(MemberDesc::info)
    }

    /// Whether `id` is the id of the member called `name` (see
    /// [`vba_automation::Dispatch::accepts_dispid`]).
    pub fn accepts_dispid(&self, id: DispId, name: &str) -> bool {
        self.by_id(id)
            .is_some_and(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// `GetIDsOfNames`: the member id, then the 0-based parameter index of
    /// every named argument.
    pub fn get_ids_of_names(&self, names: &[&str]) -> Result<Vec<DispId>, DispError> {
        let (first, others) = names.split_first().ok_or(DispError::UNKNOWN_NAME)?;
        let m = self.by_name(first).ok_or(DispError::UNKNOWN_NAME)?;
        let mut ids = vec![m.id];
        for n in others {
            let i = m
                .params
                .iter()
                .position(|p| p.name.eq_ignore_ascii_case(n))
                .ok_or(DispError::UNKNOWN_NAME)?;
            ids.push(DispId(i as i32));
        }
        Ok(ids)
    }

    /// Matches the arguments of an `Invoke` to the declared parameters of
    /// member `id` (omitted optional arguments take the declared default,
    /// named arguments are placed) and converts each present argument, and
    /// the value of a property put, to its declared type.
    ///
    /// Errors: `DISP_E_MEMBERNOTFOUND` for an unknown id or an invoke kind
    /// the member does not support; the matching errors of
    /// [`vba_automation::bind`] for the argument list; type mismatch (13) for
    /// a value that does not convert, subscript out of range (9) for an
    /// array of the wrong rank.
    pub fn bind_args(
        &self,
        id: DispId,
        flags: InvokeFlags,
        params: &DispParams,
    ) -> Result<Bound, DispError> {
        let m = self.by_id(id).ok_or(DispError::MEMBER_NOT_FOUND)?;
        let wanted = if flags.is_write() {
            flags & (InvokeFlags::PROPERTY_PUT | InvokeFlags::PROPERTY_PUT_REF)
        } else {
            InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET
        };
        if !m.supports.intersects(wanted) {
            return Err(DispError::MEMBER_NOT_FOUND);
        }
        let infos = m.info().params;
        let mut b = vba_automation::bind(&infos, params)?;
        let fixed: Vec<&ParamDesc> = m.params.iter().filter(|p| !p.param_array).collect();
        for (slot, p) in b.slots.iter_mut().zip(fixed) {
            if !slot.is_missing() {
                slot.value = coerce(&p.ty, &slot.value)?;
            }
        }
        if let Some(v) = &b.value
            && flags.is_write()
        {
            b.value = Some(coerce(&m.ty, v)?);
        }
        Ok(b)
    }

    /// Converts the value a member returns to its declared type.
    pub fn coerce_result(&self, id: DispId, v: Variant) -> Result<Variant, DispError> {
        match self.by_id(id) {
            Some(m) if !m.is_sub => coerce(&m.ty, &v),
            _ => Ok(v),
        }
    }
}

/// Converts `v` to the declared type `ty` as an assignment would.
pub fn coerce(ty: &Type, v: &Variant) -> Result<Variant, DispError> {
    match ty {
        Type::Variant | Type::Any | Type::Error => Ok(v.clone()),
        Type::Array { elem, rank } => {
            let Variant::Array(a) = v else {
                return Err(DispError::TYPE_MISMATCH);
            };
            if rank.is_some_and(|r| usize::from(r) != a.dimensions()) {
                return Err(VbaError::SUBSCRIPT_OUT_OF_RANGE.into());
            }
            match elem.var_type() {
                Some(vt) if vt != a.element_type() && vt != vba_value::VarType::Variant => {
                    Err(DispError::TYPE_MISMATCH)
                }
                _ => Ok(v.clone()),
            }
        }
        t => match t.var_type() {
            Some(vt) => convert::convert_to(v, vt).map_err(DispError::from),
            None => Ok(v.clone()),
        },
    }
}

impl TypelibProvider {
    /// The dispatch table of a library type: an interface, a dispinterface,
    /// or a coclass (its default interface). `None` if the type is not
    /// known or its members are not completely known.
    pub fn interface_table(&self, ty: &LibraryType) -> Option<InterfaceTable> {
        if !self.describes_members(ty) {
            return None;
        }
        let members = self.all_members(ty)?;
        Some(InterfaceTable::from_members(ty.clone(), members))
    }
}
