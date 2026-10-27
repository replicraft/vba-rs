//! `TypeProvider` over a checked description: early-bound typing for sema
//! and the data language servers list.

use vba_sema::{
    LibClass, LibEntry, LibEntryKind, LibEvent, LibGlobal, LibMember, LibParam, LibRecord,
    LibraryType, Type, TypeProvider,
};

use crate::check::{CheckedLibrary, ValueRef, key};

impl CheckedLibrary {
    fn qualifies(&self, qualifier: Option<&str>) -> bool {
        qualifier.is_none_or(|q| q.eq_ignore_ascii_case(self.name()))
    }

    fn owns(&self, ty: &LibraryType) -> Option<()> {
        ty.library.eq_ignore_ascii_case(self.name()).then_some(())
    }

    fn lib_type(&self, name: &str) -> LibraryType {
        LibraryType {
            library: self.name().to_owned(),
            name: name.to_owned(),
        }
    }

    fn class_of(&self, ty: &LibraryType) -> Option<usize> {
        self.owns(ty)?;
        self.class_index(&ty.name)
    }
}

impl TypeProvider for CheckedLibrary {
    fn resolve_type(&self, qualifier: Option<&str>, name: &str) -> Option<LibraryType> {
        if !self.qualifies(qualifier) {
            return None;
        }
        if let Some(i) = self.class_index(name) {
            return Some(self.lib_type(&self.library().classes[i].name));
        }
        let i = self.enum_index(name)?;
        Some(self.lib_type(&self.library().enums[i].name))
    }

    fn resolve_declared(&self, qualifier: Option<&str>, name: &str) -> Option<Type> {
        if !self.qualifies(qualifier) {
            return None;
        }
        if let Some(i) = self.class_index(name) {
            return Some(Type::Library(
                self.lib_type(&self.library().classes[i].name),
            ));
        }
        let i = self.enum_index(name)?;
        Some(Type::LibEnum(self.lib_type(&self.library().enums[i].name)))
    }

    fn has_library(&self, name: &str) -> bool {
        self.name().eq_ignore_ascii_case(name)
    }

    fn enum_members(&self, ty: &LibraryType) -> Option<Vec<(String, i64)>> {
        self.owns(ty)?;
        let e = &self.library().enums[self.enum_index(&ty.name)?];
        Some(
            e.members
                .iter()
                .map(|(n, v, _)| (n.clone(), i64::from(*v)))
                .collect(),
        )
    }

    fn record(&self, _ty: &LibraryType) -> Option<LibRecord> {
        None
    }

    fn class_info(&self, ty: &LibraryType) -> Option<LibClass> {
        let i = self.class_of(ty)?;
        Some(LibClass {
            default_interface: None,
            creatable: self.library().classes[i].creatable,
        })
    }

    fn member(&self, ty: &LibraryType, name: &str) -> Option<LibMember> {
        self.owns(ty)?;
        self.member_sig(&ty.name, name).map(|s| s.to_lib_member())
    }

    fn default_member(&self, ty: &LibraryType) -> Option<LibMember> {
        self.owns(ty)?;
        self.class_members(&ty.name)?
            .iter()
            .find(|m| m.is_default)
            .map(|s| s.to_lib_member())
    }

    fn describes_members(&self, ty: &LibraryType) -> bool {
        self.class_of(ty).is_some()
            || (self.owns(ty).is_some() && self.enum_index(&ty.name).is_some())
    }

    fn interface_members(&self, ty: &LibraryType) -> Option<Vec<LibMember>> {
        self.owns(ty)?;
        Some(
            self.class_members(&ty.name)?
                .iter()
                .filter(|m| !m.hidden)
                .map(|s| s.to_lib_member())
                .collect(),
        )
    }

    fn is_interface(&self, ty: &LibraryType) -> Option<bool> {
        let i = self.class_of(ty)?;
        Some(self.library().classes[i].interface)
    }

    fn enumerable_item(&self, ty: &LibraryType) -> Option<Type> {
        let i = self.class_of(ty)?;
        let item = self.library().classes[i].enumerable.as_ref()?;
        Some(crate::check::sema_type(self.library(), item))
    }

    fn is_exact_class(&self, ty: &LibraryType) -> bool {
        self.class_of(ty).is_some()
    }

    fn events(&self, ty: &LibraryType) -> Option<Vec<LibEvent>> {
        self.owns(ty)?;
        Some(
            self.class_events(&ty.name)?
                .iter()
                .map(|e| LibEvent {
                    name: e.name.clone(),
                    params: e
                        .params
                        .iter()
                        .map(|p| LibParam {
                            name: p.name.clone(),
                            ty: p.ty.clone(),
                            optional: p.optional,
                            by_ref: p.by_ref,
                            param_array: p.param_array,
                            default: p.default.clone(),
                        })
                        .collect(),
                })
                .collect(),
        )
    }

    fn global(&self, name: &str) -> Option<LibGlobal> {
        let lib = self.library();
        Some(match self.inner.values.get(&key(name))? {
            ValueRef::Const(i) => {
                let c = &lib.consts[*i];
                let ty = self.sema_type_of(&c.value.type_ref());
                LibGlobal::Constant(c.value.to_variant(), ty)
            }
            ValueRef::EnumMember(e, j) => {
                let en = &lib.enums[*e];
                LibGlobal::Constant(
                    vba_value::Variant::Long(en.members[*j].1),
                    Type::LibEnum(self.lib_type(&en.name)),
                )
            }
            ValueRef::Function(i) => LibGlobal::Function(self.inner.functions[*i].to_lib_member()),
            ValueRef::Global(i) => LibGlobal::Object(self.sema_type_of(&lib.globals[*i].ty)),
        })
    }

    fn catalog(&self) -> Vec<LibEntry> {
        let lib = self.library();
        let entry = |name: &str, kind| LibEntry {
            library: lib.name.clone(),
            name: name.to_owned(),
            kind,
        };
        let mut v = Vec::new();
        v.extend(
            lib.classes
                .iter()
                .map(|c| entry(&c.name, LibEntryKind::Class)),
        );
        v.extend(lib.enums.iter().map(|e| entry(&e.name, LibEntryKind::Enum)));
        v.extend(
            lib.consts
                .iter()
                .map(|c| entry(&c.name, LibEntryKind::Constant)),
        );
        for e in &lib.enums {
            v.extend(
                e.members
                    .iter()
                    .map(|(n, _, _)| entry(n, LibEntryKind::Constant)),
            );
        }
        v.extend(
            lib.functions
                .iter()
                .map(|f| entry(&f.name, LibEntryKind::Function)),
        );
        v.extend(
            lib.globals
                .iter()
                .map(|g| entry(&g.name, LibEntryKind::Global)),
        );
        v
    }

    fn documentation(&self, ty: Option<&LibraryType>, member: Option<&str>) -> Option<String> {
        let lib = self.library();
        let text = match (ty, member) {
            (Some(t), None) => {
                self.owns(t)?;
                match (self.class_index(&t.name), self.enum_index(&t.name)) {
                    (Some(i), _) => lib.classes[i].doc.clone(),
                    (_, Some(i)) => lib.enums[i].doc.clone(),
                    _ => return None,
                }
            }
            (Some(t), Some(m)) => {
                self.owns(t)?;
                let c = self.class(&t.name)?;
                c.members
                    .iter()
                    .find(|x| key(&x.name) == key(m))
                    .map(|x| x.doc.clone())?
            }
            (None, Some(n)) => match self.inner.values.get(&key(n))? {
                ValueRef::Const(i) => lib.consts[*i].doc.clone(),
                ValueRef::EnumMember(e, j) => lib.enums[*e].members[*j].2.clone(),
                ValueRef::Function(i) => lib.functions[*i].doc.clone(),
                ValueRef::Global(i) => lib.globals[*i].doc.clone(),
            },
            (None, None) => return None,
        };
        (!text.is_empty()).then_some(text)
    }
}

impl CheckedLibrary {
    fn member_sig(&self, class: &str, name: &str) -> Option<&crate::check::MemberSig> {
        self.class_member(class, name)
    }

    fn sema_type_of(&self, t: &crate::model::TypeRef) -> Type {
        crate::check::sema_type(self.library(), t)
    }
}
