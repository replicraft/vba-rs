//! Human-readable rendering of type descriptions.

use crate::model::{Guid, TypeDesc, TypeLib, TypeRef, vt};

impl TypeLib {
    /// A short human-readable rendering of a type description
    /// (`BSTR`, `Foo*`, `SAFEARRAY(I4)`, `UI1[8]`, ...), for dumps and
    /// diagnostics.
    pub fn describe(&self, t: &TypeDesc) -> String {
        match t {
            TypeDesc::Prim(v) => vt_name(*v),
            TypeDesc::Ptr(inner) => format!("{}*", self.describe(inner)),
            TypeDesc::SafeArray(inner) => format!("SAFEARRAY({})", self.describe(inner)),
            TypeDesc::CArray { elem, bounds } => {
                let dims: Vec<String> = bounds.iter().map(|(n, _)| n.to_string()).collect();
                format!("{}[{}]", self.describe(elem), dims.join(","))
            }
            TypeDesc::User(TypeRef::Local(i)) => self
                .types
                .get(*i)
                .map_or_else(|| format!("<bad type {i}>"), |t| t.name.clone()),
            TypeDesc::User(TypeRef::Imported(i)) => match self.imports.get(*i) {
                Some(imp) => match imp.type_guid {
                    Some(g) if g == Guid::IDISPATCH => "IDispatch".to_owned(),
                    Some(g) if g == Guid::IUNKNOWN => "IUnknown".to_owned(),
                    Some(g) => format!("{}!{{{g}}}", imp.file),
                    None => format!("{}!?", imp.file),
                },
                None => format!("<bad import {i}>"),
            },
        }
    }
}

/// The name of a `VT_*` value (`"I4"`, `"BSTR"`), or `VT_<n>` when unknown.
pub fn vt_name(v: u16) -> String {
    let n = match v {
        vt::EMPTY => "EMPTY",
        vt::NULL => "NULL",
        vt::I2 => "I2",
        vt::I4 => "I4",
        vt::R4 => "R4",
        vt::R8 => "R8",
        vt::CY => "CY",
        vt::DATE => "DATE",
        vt::BSTR => "BSTR",
        vt::DISPATCH => "DISPATCH",
        vt::ERROR => "ERROR",
        vt::BOOL => "BOOL",
        vt::VARIANT => "VARIANT",
        vt::UNKNOWN => "UNKNOWN",
        vt::DECIMAL => "DECIMAL",
        vt::I1 => "I1",
        vt::UI1 => "UI1",
        vt::UI2 => "UI2",
        vt::UI4 => "UI4",
        vt::I8 => "I8",
        vt::UI8 => "UI8",
        vt::INT => "INT",
        vt::UINT => "UINT",
        vt::VOID => "VOID",
        vt::HRESULT => "HRESULT",
        vt::LPSTR => "LPSTR",
        vt::LPWSTR => "LPWSTR",
        other => return format!("VT_{other}"),
    };
    n.to_owned()
}
