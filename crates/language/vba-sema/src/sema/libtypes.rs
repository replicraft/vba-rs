//! Library enums and records met during analysis.
//!
//! A library enum is stored as `Long`; a library record gets a synthetic
//! [`UdtId`] numbered after the project's own user-defined types, so the
//! later stages treat it exactly like a `Type ... End Type` of the project.
//! Both are registered the first time a type is seen (in an `As` clause or in
//! the signature of a library member) and moved into the
//! [`Program`](crate::Program) when the analysis ends.

use vba_ast::Name;
use vba_source::Span;

use super::Sema;
use crate::codes;
use crate::program::{FieldInfo, LibEnumInfo, LibRecordInfo};
use crate::types::{LibraryType, ModuleId, Type, UdtId};

/// The registry; see the module documentation.
#[derive(Debug, Default)]
pub(crate) struct LibTypes {
    pub enums: Vec<LibEnumInfo>,
    pub records: Vec<LibRecordInfo>,
}

const MAX_NESTING: usize = 16;

impl Sema<'_> {
    /// Registers the library enums and records that `t` mentions. Types the
    /// provider does not describe are left alone (their members stay
    /// unverified).
    pub(crate) fn note_type(&self, t: &Type) {
        self.note_type_depth(t, 0);
    }

    fn note_type_depth(&self, t: &Type, depth: usize) {
        if depth > MAX_NESTING {
            return;
        }
        match t {
            Type::LibEnum(lt) => {
                let mut reg = self.lib_types.borrow_mut();
                if reg.enums.iter().any(|e| e.ty == *lt) {
                    return;
                }
                let members = self.provider.enum_members(lt).unwrap_or_default();
                reg.enums.push(LibEnumInfo {
                    ty: lt.clone(),
                    members,
                });
            }
            Type::LibRecord(lt) => {
                if self.lib_types.borrow().records.iter().any(|r| r.ty == *lt) {
                    return;
                }
                let Some(rec) = self.provider.record(lt) else {
                    return;
                };
                let id =
                    UdtId((self.program.udts.len() + self.lib_types.borrow().records.len()) as u32);
                let fields = rec
                    .fields
                    .iter()
                    .map(|f| FieldInfo {
                        name: Name::new(&f.name),
                        ty: f.ty.clone(),
                        fixed_bounds: None,
                        span: Span::empty(0),
                    })
                    .collect();
                self.lib_types.borrow_mut().records.push(LibRecordInfo {
                    id,
                    ty: lt.clone(),
                    fields,
                    offsets: rec.fields.iter().map(|f| f.offset).collect(),
                    size: rec.size,
                    alignment: rec.alignment,
                });
                for f in &rec.fields {
                    self.note_type_depth(&f.ty, depth + 1);
                }
            }
            Type::Array { elem, .. } => self.note_type_depth(elem, depth + 1),
            _ => {}
        }
    }

    /// Reports `New` applied to a library type that cannot be instantiated
    /// (an interface, or a coclass without `cancreate`).
    pub(crate) fn check_creatable(&self, module: ModuleId, span: Span, t: &Type) {
        if let Type::Library(lt) = t
            && let Some(ci) = self.provider.class_info(lt)
            && !ci.creatable
        {
            self.error(
                module,
                codes::NOT_CREATABLE,
                span,
                format!("`New` cannot create `{lt}`: it is not an instantiable class"),
            );
        }
    }

    /// The synthetic id of a registered library record.
    pub(crate) fn lib_record_id(&self, lt: &LibraryType) -> Option<UdtId> {
        self.lib_types
            .borrow()
            .records
            .iter()
            .find(|r| r.ty == *lt)
            .map(|r| r.id)
    }

    /// Index and type of the field `name` of a registered record.
    pub(crate) fn lib_record_field(&self, id: UdtId, name: &Name) -> Option<(u32, Type)> {
        let reg = self.lib_types.borrow();
        let r = reg.records.iter().find(|r| r.id == id)?;
        let i = r.fields.iter().position(|f| f.name == *name)?;
        Some((i as u32, r.fields[i].ty.clone()))
    }

    /// Moves the registry into the program.
    pub(crate) fn finish_library_types(&mut self) {
        let reg = std::mem::take(&mut *self.lib_types.borrow_mut());
        self.program.lib_enums = reg.enums;
        self.program.lib_records = reg.records;
    }
}
