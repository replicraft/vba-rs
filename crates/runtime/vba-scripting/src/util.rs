//! Shared plumbing for the table-driven automation objects of this crate.

use vba_automation::{Bound, DispError, DispId, InvokeFlags, MemberInfo, ParamInfo};
use vba_value::{Variant, VbaString, convert};

/// How a member can be invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A method (also readable as a property get, as automation functions are).
    Method,
    /// A read-only property.
    Get,
    /// A property that can be read and assigned (`Let`; `Set` as well when the
    /// flag is set).
    GetPut { put_ref: bool },
}

/// The declaration of one member.
pub(crate) struct Def {
    pub name: &'static str,
    pub id: i32,
    /// Parameter names; the first `required` are mandatory.
    pub params: &'static [&'static str],
    pub required: usize,
    pub kind: Kind,
}

/// A method with `params` of which the first `required` are mandatory.
pub(crate) const fn method(
    name: &'static str,
    id: i32,
    params: &'static [&'static str],
    required: usize,
) -> Def {
    Def {
        name,
        id,
        params,
        required,
        kind: Kind::Method,
    }
}

/// A read-only property, with `params` for indexed ones.
pub(crate) const fn getter(name: &'static str, id: i32, params: &'static [&'static str]) -> Def {
    Def {
        name,
        id,
        params,
        required: params.len(),
        kind: Kind::Get,
    }
}

/// A read/write property.
pub(crate) const fn property(
    name: &'static str,
    id: i32,
    params: &'static [&'static str],
    put_ref: bool,
) -> Def {
    Def {
        name,
        id,
        params,
        required: params.len(),
        kind: Kind::GetPut { put_ref },
    }
}

pub(crate) fn find_name<'a>(defs: &'a [Def], name: &str) -> Option<&'a Def> {
    defs.iter()
        .find(|d| vba_automation::names_equal(d.name, name))
}

pub(crate) fn find_id(defs: &[Def], id: DispId) -> Option<&Def> {
    defs.iter().find(|d| d.id == id.0)
}

pub(crate) fn params_of(d: &Def) -> Vec<ParamInfo> {
    d.params
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if i < d.required {
                ParamInfo::required(n).by_val()
            } else {
                ParamInfo::optional(n).by_val()
            }
        })
        .collect()
}

pub(crate) fn info_of(d: &Def) -> MemberInfo {
    let supports = match d.kind {
        Kind::Method => InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET,
        Kind::Get => InvokeFlags::PROPERTY_GET | InvokeFlags::METHOD,
        Kind::GetPut { put_ref: false } => {
            InvokeFlags::PROPERTY_GET | InvokeFlags::METHOD | InvokeFlags::PROPERTY_PUT
        }
        Kind::GetPut { put_ref: true } => {
            InvokeFlags::PROPERTY_GET
                | InvokeFlags::METHOD
                | InvokeFlags::PROPERTY_PUT
                | InvokeFlags::PROPERTY_PUT_REF
        }
    };
    MemberInfo {
        name: d.name.to_owned(),
        id: DispId(d.id),
        supports,
        params: params_of(d),
    }
}

/// Checks that the member supports the requested access; otherwise the
/// member is "not found" for that access (error 438).
pub(crate) fn check_access(d: &Def, flags: InvokeFlags) -> Result<(), DispError> {
    let ok = match d.kind {
        Kind::Method | Kind::Get => !flags.is_write(),
        Kind::GetPut { put_ref } => !flags.contains(InvokeFlags::PROPERTY_PUT_REF) || put_ref,
    };
    if ok {
        Ok(())
    } else {
        Err(DispError::MEMBER_NOT_FOUND)
    }
}

/// An error with a VBA number and the standard description, raised by the
/// object `source`.
pub(crate) fn raise(number: i32, description: &str, source: &str) -> DispError {
    DispError::raise(number, description, source)
}

pub(crate) fn text_arg(b: &Bound, i: usize) -> Result<String, DispError> {
    Ok(string_of(&b.slots[i].value)?.to_string_lossy())
}

pub(crate) fn string_of(v: &Variant) -> Result<VbaString, DispError> {
    convert::to_vba_string(v).map_err(DispError::from)
}

pub(crate) fn long_arg(b: &Bound, i: usize) -> Result<i32, DispError> {
    convert::to_long(&b.slots[i].value).map_err(DispError::from)
}

pub(crate) fn bool_arg(b: &Bound, i: usize) -> Result<bool, DispError> {
    convert::to_boolean(&b.slots[i].value).map_err(DispError::from)
}

pub(crate) fn present(b: &Bound, i: usize) -> bool {
    b.slots.get(i).is_some_and(|a| !a.is_missing())
}

pub(crate) fn opt_bool(b: &Bound, i: usize, default: bool) -> Result<bool, DispError> {
    if present(b, i) {
        bool_arg(b, i)
    } else {
        Ok(default)
    }
}

pub(crate) fn opt_long(b: &Bound, i: usize, default: i32) -> Result<i32, DispError> {
    if present(b, i) {
        long_arg(b, i)
    } else {
        Ok(default)
    }
}

/// Generates the `Dispatch` boilerplate for a type with `DEFS`, `CLASS` and
/// an inherent `call(&self, table, def, flags, bound)`.
macro_rules! dispatch_impl {
    ($ty:ty) => {
        impl vba_automation::Dispatch for $ty {
            fn class_name(&self) -> String {
                Self::CLASS.to_owned()
            }

            fn member_id(&self, name: &str) -> Option<vba_automation::DispId> {
                $crate::util::find_name(Self::DEFS, name).map(|d| vba_automation::DispId(d.id))
            }

            fn member_info(
                &self,
                id: vba_automation::DispId,
            ) -> Option<vba_automation::MemberInfo> {
                $crate::util::find_id(Self::DEFS, id).map($crate::util::info_of)
            }

            fn invoke(
                &self,
                table: &vba_automation::ObjectTable,
                id: vba_automation::DispId,
                flags: vba_automation::InvokeFlags,
                params: &mut vba_automation::DispParams,
            ) -> Result<vba_value::Variant, vba_automation::DispError> {
                if id == vba_automation::DispId::NEWENUM {
                    return self.new_enum(table);
                }
                let def = $crate::util::find_id(Self::DEFS, id)
                    .ok_or(vba_automation::DispError::MEMBER_NOT_FOUND)?;
                $crate::util::check_access(def, flags)?;
                let bound = vba_automation::bind(&$crate::util::params_of(def), params)?;
                self.call(table, def.id, flags, &bound)
            }

            fn terminate(&self, table: &vba_automation::ObjectTable) {
                self.on_terminate(table);
            }

            fn held_references(&self) -> Vec<vba_automation::ObjectHandle> {
                self.held()
            }
        }
    };
}
pub(crate) use dispatch_impl;
