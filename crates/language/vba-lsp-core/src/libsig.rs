//! Signatures of library members described by the embedding application's
//! type provider.

use vba_sema::{LibMember, LibMemberKind, LibraryType};

use crate::project::Project;

impl Project {
    /// The declaration-style signature of a library member, with the UTF-16
    /// offsets of its parameters in the label.
    pub fn lib_member_sig(&self, owner: &LibraryType, m: &LibMember) -> (String, Vec<(u32, u32)>) {
        let kw = match (m.kind, m.is_sub) {
            (LibMemberKind::PropertyGet | LibMemberKind::Property, _) => "Property Get",
            (LibMemberKind::PropertyPut, _) => "Property Let",
            (LibMemberKind::Method, true) => "Sub",
            (LibMemberKind::Method, false) => "Function",
        };
        let mut label = format!("{kw} {}.{}", owner.name, m.name);
        let mut spans = Vec::new();
        if !m.params.is_empty() || m.kind == LibMemberKind::Method {
            label.push('(');
            for (i, p) in m.params.iter().enumerate() {
                if i > 0 {
                    label.push_str(", ");
                }
                let start = label.encode_utf16().count() as u32;
                if p.optional {
                    label.push_str("Optional ");
                }
                if p.param_array {
                    label.push_str("ParamArray ");
                }
                label.push_str(if p.by_ref { "ByRef " } else { "ByVal " });
                label.push_str(&p.name);
                label.push_str(" As ");
                label.push_str(&self.type_name(&p.ty));
                spans.push((start, label.encode_utf16().count() as u32));
            }
            label.push(')');
        }
        if !m.is_sub {
            label.push_str(" As ");
            label.push_str(&self.type_name(&m.ty));
        }
        (label, spans)
    }
}
