//! Signatures, documentation and top-level names of the libraries the
//! embedding application's type provider describes.

use vba_sema::{LibEntry, LibEntryKind, LibGlobal, LibMember, LibMemberKind, LibraryType, Type};
use vba_value::Variant;

use crate::project::Project;
use crate::types::{
    CompletionItem, CompletionKind, Hover, ParameterInfo, SignatureHelp, SignatureInfo,
};

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

    /// The documentation text the provider has for a type
    /// (`(Some(ty), None)`), one of its members, or a global name
    /// (`(None, Some(name))`).
    pub(crate) fn lib_doc(&self, ty: Option<&LibraryType>, member: Option<&str>) -> Option<String> {
        self.provider.as_deref()?.documentation(ty, member)
    }

    /// The library a top-level name belongs to, from the provider's catalog.
    fn lib_entry(&self, name: &str) -> Option<LibEntry> {
        let p = self.provider.as_deref()?;
        p.catalog().into_iter().find(|e| {
            e.name.eq_ignore_ascii_case(name)
                && !matches!(e.kind, LibEntryKind::Class | LibEntryKind::Enum)
        })
    }

    /// The label and documentation of a top-level library name.
    fn lib_entry_text(&self, e: &LibEntry) -> Option<(String, Option<String>)> {
        let p = self.provider.as_deref()?;
        let qualified = format!("{}.{}", e.library, e.name);
        let ty = LibraryType {
            library: e.library.clone(),
            name: e.name.clone(),
        };
        Some(match e.kind {
            LibEntryKind::Class => (
                format!("Class {qualified}"),
                p.documentation(Some(&ty), None),
            ),
            LibEntryKind::Enum => (
                format!("Enum {qualified}"),
                p.documentation(Some(&ty), None),
            ),
            LibEntryKind::Constant | LibEntryKind::Function | LibEntryKind::Global => {
                let doc = p.documentation(None, Some(&e.name));
                let label = match p.global(&e.name)? {
                    LibGlobal::Constant(v, t) => format!(
                        "Const {} = {} As {}",
                        e.name,
                        constant_text(&v),
                        self.type_name(&t)
                    ),
                    LibGlobal::Object(t) => format!("{} As {}", e.name, self.type_name(&t)),
                    LibGlobal::Function(m) => {
                        let owner = LibraryType {
                            library: e.library.clone(),
                            name: e.library.clone(),
                        };
                        self.lib_member_sig(&owner, &m).0
                    }
                };
                (label, doc)
            }
        })
    }

    /// Hover for a top-level library name: a constant, a global function or
    /// object, or (in a type position) a class or enumeration.
    pub(crate) fn lib_global_hover(
        &self,
        name: &str,
        in_type: bool,
        fence: impl Fn(String, Vec<String>) -> Option<Hover>,
    ) -> Option<Hover> {
        let p = self.provider.as_deref()?;
        let entry = if in_type {
            let ty = p.resolve_declared(None, name)?;
            let (Type::Library(lt) | Type::LibEnum(lt)) = ty else {
                return None;
            };
            p.catalog().into_iter().find(|e| {
                e.name.eq_ignore_ascii_case(&lt.name)
                    && matches!(e.kind, LibEntryKind::Class | LibEntryKind::Enum)
            })?
        } else {
            self.lib_entry(name)?
        };
        let (label, doc) = self.lib_entry_text(&entry)?;
        let mut extra: Vec<String> = doc.into_iter().collect();
        extra.push(format!("Provided by the `{}` library.", entry.library));
        fence(label, extra)
    }

    /// Completion items for the provider's top-level names: classes and
    /// enumerations where a type is expected, everything else in
    /// expressions.
    pub(crate) fn lib_catalog_items(&self, types_only: bool) -> Vec<CompletionItem> {
        let Some(p) = self.provider.as_deref() else {
            return Vec::new();
        };
        p.catalog()
            .into_iter()
            .filter(|e| {
                let is_type = matches!(e.kind, LibEntryKind::Class | LibEntryKind::Enum);
                is_type == types_only
            })
            .filter_map(|e| self.catalog_item(&e))
            .collect()
    }

    /// The names a library offers after `Library.`.
    pub(crate) fn lib_qualified_items(&self, library: &str) -> Option<Vec<CompletionItem>> {
        let p = self.provider.as_deref()?;
        if !p.has_library(library) {
            return None;
        }
        Some(
            p.catalog()
                .into_iter()
                .filter(|e| e.library.eq_ignore_ascii_case(library))
                .filter_map(|e| self.catalog_item(&e))
                .collect(),
        )
    }

    fn catalog_item(&self, e: &LibEntry) -> Option<CompletionItem> {
        let (label, doc) = self.lib_entry_text(e)?;
        Some(CompletionItem {
            label: e.name.clone(),
            kind: match e.kind {
                LibEntryKind::Class | LibEntryKind::Enum => CompletionKind::Type,
                LibEntryKind::Constant => CompletionKind::Constant,
                LibEntryKind::Function => CompletionKind::Function,
                LibEntryKind::Global => CompletionKind::Variable,
            },
            detail: Some(label),
            documentation: doc,
        })
    }

    /// Signature help for a call of a global library function (`Nz(`).
    /// `name` may be qualified with the library.
    pub(crate) fn lib_global_signature(
        &self,
        name: &str,
        arg: u32,
        named: Option<&str>,
    ) -> Option<SignatureHelp> {
        let p = self.provider.as_deref()?;
        let bare = name.rsplit('.').next().unwrap_or(name);
        let LibGlobal::Function(m) = p.global(bare)? else {
            return None;
        };
        let entry = self.lib_entry(bare)?;
        let owner = LibraryType {
            library: entry.library.clone(),
            name: entry.library.clone(),
        };
        let (label, spans) = self.lib_member_sig(&owner, &m);
        let n = m.params.len();
        let idx = match named {
            Some(nm) => m
                .params
                .iter()
                .position(|q| q.name.eq_ignore_ascii_case(nm))
                .map(|i| i as u32),
            None => Some(arg),
        };
        let active = idx.and_then(|i| {
            if (i as usize) < n {
                Some(i)
            } else if m.params.last().is_some_and(|q| q.param_array) {
                Some(n as u32 - 1)
            } else {
                None
            }
        });
        let doc = p.documentation(None, Some(&m.name));
        Some(SignatureHelp {
            signatures: vec![SignatureInfo {
                label,
                documentation: Some(match doc {
                    Some(d) => format!("{d}\n\nFunction of the `{}` library.", entry.library),
                    None => format!("Function of the `{}` library.", entry.library),
                }),
                parameters: spans
                    .into_iter()
                    .map(|(start, end)| ParameterInfo { start, end })
                    .collect(),
            }],
            active_parameter: active,
        })
    }
}

fn constant_text(v: &Variant) -> String {
    match v {
        Variant::String(s) => format!("\"{}\"", s.to_string_lossy()),
        Variant::Boolean(b) => if *b { "True" } else { "False" }.to_string(),
        Variant::Integer(n) => n.to_string(),
        Variant::Long(n) => n.to_string(),
        Variant::LongLong(n) => n.to_string(),
        Variant::Double(n) => n.to_string(),
        Variant::Single(n) => n.to_string(),
        Variant::Null => "Null".to_string(),
        Variant::Empty => "Empty".to_string(),
        _ => "...".to_string(),
    }
}
