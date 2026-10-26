//! Adapter from loaded type libraries to `vba_sema::TypeProvider`.
//!
//! The mapping follows how VBA presents COM members:
//!
//! * `[lcid]` parameters and the `[retval]` parameter are hidden; the
//!   `[retval]` type (dereferenced) becomes the result type.
//! * `HRESULT`/`void` members without a `[retval]` are `Sub`s.
//! * A get/put/putref group of one name is one property; a `[propget]` with
//!   index parameters keeps them as index parameters.
//! * `FUNCFLAG_FRESTRICTED` members are invisible; hidden ones stay callable.
//! * A pointer to an interface is an object (`ByVal`); any other pointer is
//!   a `ByRef` parameter of the pointee type.
//! * Unsigned types map to the next wider signed VBA type.
//! * A coclass exposes the members of its default interface, and its
//!   `[source]` interface supplies the events.
//!
//! * Enums are `Type::LibEnum` (stored as `Long`), records are
//!   `Type::LibRecord`, aliases resolve transitively to the type they stand
//!   for; coclasses report whether `New` may create them. Members carry
//!   their `DISPID`s and the defaults the library declares for optional
//!   parameters, which is what early binding uses.
//!
//! Unions are not modelled: naming one is an unresolved type.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vba_sema::{
    LibClass, LibEvent, LibField, LibGlobal, LibMember, LibMemberKind, LibParam, LibRecord,
    LibraryType, Type, TypeProvider,
};
use vba_value::{Currency, Variant, VbaString};

use crate::model::{
    Const, ConstValue, FuncFlags, Function, Guid, InvokeKind, ParamFlags, TypeDesc, TypeInfo,
    TypeKind, TypeLib, TypeRef, VarFlags, VarKind, vt,
};

/// Serves one or more loaded libraries to `vba-sema`.
#[derive(Debug, Clone, Default)]
pub struct TypelibProvider {
    libs: Vec<TypeLib>,
    /// Members of a type, by `(library, type)` index; cleared when a library
    /// is added.
    cache: RefCell<HashMap<(usize, usize), Rc<Members>>>,
}

/// The VBA members of a type and whether the list is complete.
type Members = (Vec<LibMember>, bool);

const MAX_DEPTH: usize = 16;

impl TypelibProvider {
    /// An empty provider.
    pub fn new() -> TypelibProvider {
        TypelibProvider::default()
    }

    /// Adds a library (builder style).
    #[must_use]
    pub fn with_library(mut self, lib: TypeLib) -> TypelibProvider {
        self.add_library(lib);
        self
    }

    /// Adds a library.
    pub fn add_library(&mut self, lib: TypeLib) {
        self.libs.push(lib);
        self.cache.borrow_mut().clear();
    }

    /// The loaded libraries.
    pub fn libraries(&self) -> &[TypeLib] {
        &self.libs
    }

    fn find(&self, lt: &LibraryType) -> Option<(usize, usize)> {
        self.libs.iter().enumerate().find_map(|(li, l)| {
            if !l.name.eq_ignore_ascii_case(&lt.library) {
                return None;
            }
            l.type_by_name(&lt.name).map(|(ti, _)| (li, ti))
        })
    }

    fn lib_type(&self, li: usize, ti: usize) -> LibraryType {
        LibraryType {
            library: self.libs[li].name.clone(),
            name: self.libs[li].types[ti].name.clone(),
        }
    }

    /// Finds a type by GUID in any loaded library.
    fn find_guid(&self, g: &Guid) -> Option<(usize, usize)> {
        self.libs
            .iter()
            .enumerate()
            .find_map(|(li, l)| l.type_by_guid(g).map(|(ti, _)| (li, ti)))
    }

    /// Resolves a reference made inside library `li`.
    fn target(&self, li: usize, r: TypeRef) -> Option<Target> {
        match r {
            TypeRef::Local(i) => self.libs.get(li)?.types.get(i).map(|_| Target::At(li, i)),
            TypeRef::Imported(i) => {
                let g = self.libs.get(li)?.imports.get(i)?.type_guid?;
                if g == Guid::IDISPATCH || g == Guid::IUNKNOWN {
                    return Some(Target::Root);
                }
                self.find_guid(&g).map(|(l, t)| Target::At(l, t))
            }
        }
    }

    /// The functions visible on a type: its own plus inherited ones.
    /// The flag tells whether the list is complete (every base resolved).
    fn functions(&self, li: usize, ti: usize) -> (Vec<(usize, &Function)>, bool) {
        let mut out = Vec::new();
        let mut complete = true;
        let mut cur = Some((li, ti));
        let mut depth = 0;
        while let Some((l, t)) = cur {
            depth += 1;
            if depth > MAX_DEPTH {
                complete = false;
                break;
            }
            let info = &self.libs[l].types[t];
            let source = match info.kind {
                TypeKind::CoClass => {
                    // The default non-source interface carries the members.
                    let pick = info
                        .implements
                        .iter()
                        .find(|i| {
                            i.flags.contains(crate::model::ImplFlags::DEFAULT)
                                && !i.flags.contains(crate::model::ImplFlags::SOURCE)
                        })
                        .or_else(|| {
                            info.implements
                                .iter()
                                .find(|i| !i.flags.contains(crate::model::ImplFlags::SOURCE))
                        });
                    match pick.and_then(|i| self.target(l, i.target)) {
                        Some(Target::At(l2, t2)) => {
                            cur = Some((l2, t2));
                            continue;
                        }
                        _ => {
                            complete = false;
                            break;
                        }
                    }
                }
                _ => info,
            };
            out.extend(source.functions.iter().map(|f| (l, f)));
            cur = None;
            if let Some(base) = source.implements.first() {
                match self.target(l, base.target) {
                    Some(Target::At(l2, t2)) => cur = Some((l2, t2)),
                    Some(Target::Root) => {}
                    None => complete = false,
                }
            }
        }
        (out, complete)
    }

    /// The VBA members of a type and whether the list is complete (cached).
    fn members(&self, li: usize, ti: usize) -> Rc<Members> {
        if let Some(m) = self.cache.borrow().get(&(li, ti)) {
            return m.clone();
        }
        let m = Rc::new(self.compute_members(li, ti));
        self.cache.borrow_mut().insert((li, ti), m.clone());
        m
    }

    fn compute_members(&self, li: usize, ti: usize) -> (Vec<LibMember>, bool) {
        let info = &self.libs[li].types[ti];
        let (funcs, complete) = self.functions(li, ti);
        let mut order: Vec<String> = Vec::new();
        let mut groups: Vec<Vec<(usize, &Function)>> = Vec::new();
        for (l, f) in funcs {
            if f.flags.contains(FuncFlags::RESTRICTED) {
                continue;
            }
            let key = f.name.to_ascii_lowercase();
            if let Some(i) = order.iter().position(|k| *k == key) {
                groups[i].push((l, f));
            } else {
                order.push(key);
                groups.push(vec![(l, f)]);
            }
        }
        let mut out: Vec<LibMember> = groups
            .iter()
            .filter_map(|g| self.member_from_group(g))
            .collect();
        for v in &info.variables {
            let (kind, writable) = match v.kind {
                VarKind::Const | VarKind::Static => continue,
                VarKind::Dispatch | VarKind::PerInstance => (
                    LibMemberKind::Property,
                    !v.flags.contains(VarFlags::READ_ONLY),
                ),
            };
            let name = v.name.clone();
            if out.iter().any(|m| m.name.eq_ignore_ascii_case(&name)) {
                continue;
            }
            out.push(LibMember {
                name,
                kind: if writable {
                    kind
                } else {
                    LibMemberKind::PropertyGet
                },
                params: Vec::new(),
                ty: self.map(li, &v.ty),
                is_default: v.memid == 0,
                is_sub: false,
                dispid: Some(v.memid),
            });
        }
        (out, complete)
    }

    fn member_from_group(&self, g: &[(usize, &Function)]) -> Option<LibMember> {
        let pick = |k: InvokeKind| g.iter().find(|(_, f)| f.invoke == k).copied();
        let (get, put, putref, func) = (
            pick(InvokeKind::PropertyGet),
            pick(InvokeKind::PropertyPut),
            pick(InvokeKind::PropertyPutRef),
            pick(InvokeKind::Func),
        );
        let first = g.first()?;
        let name = first.1.name.clone();
        let is_default = g.iter().any(|(_, f)| f.memid == 0);
        if let (Some((l, f)), None, None, None) = (func, get, put, putref) {
            let view = self.view(l, f);
            let is_sub = view.result.is_none();
            return Some(LibMember {
                name,
                kind: LibMemberKind::Method,
                params: view.params,
                ty: view.result.unwrap_or(Type::Variant),
                is_default,
                is_sub,
                dispid: Some(f.memid),
            });
        }
        if let Some((l, f)) = get {
            let view = self.view(l, f);
            let writable = put.is_some() || putref.is_some();
            return Some(LibMember {
                name,
                kind: if writable {
                    LibMemberKind::Property
                } else {
                    LibMemberKind::PropertyGet
                },
                params: view.params,
                ty: view.result.unwrap_or(Type::Variant),
                is_default,
                is_sub: false,
                dispid: Some(f.memid),
            });
        }
        let (l, f) = put.or(putref)?;
        let mut view = self.view(l, f);
        let value = view.params.pop();
        Some(LibMember {
            name,
            kind: LibMemberKind::PropertyPut,
            params: view.params,
            ty: value.map_or(Type::Variant, |p| p.ty),
            is_default,
            is_sub: false,
            dispid: Some(f.memid),
        })
    }

    /// The VBA view of a function: visible parameters and result.
    fn view(&self, li: usize, f: &Function) -> View {
        let retval = f
            .params
            .iter()
            .find(|p| p.flags.contains(ParamFlags::RETVAL));
        let n = f.params.len();
        let mut params = Vec::new();
        for (i, p) in f.params.iter().enumerate() {
            if p.flags.intersects(ParamFlags::RETVAL | ParamFlags::LCID) {
                continue;
            }
            let param_array = f.optional_count < 0 && i + 1 == n;
            let ty = self.map(li, &p.ty);
            let default = p
                .default
                .as_ref()
                .filter(|_| p.flags.contains(ParamFlags::HAS_DEFAULT))
                .and_then(|c| self.constant(c))
                .and_then(|(v, _)| default_for(v, &ty));
            params.push(LibParam {
                name: p.name.clone().unwrap_or_else(|| format!("arg{}", i + 1)),
                ty,
                optional: p.flags.contains(ParamFlags::OPTIONAL) || param_array,
                by_ref: self.is_by_ref(li, &p.ty),
                param_array,
                default,
            });
        }
        let result = match retval {
            Some(r) => Some(self.map(li, &r.ty)),
            None => match &f.ret {
                TypeDesc::Prim(vt::VOID | vt::HRESULT) => None,
                other => Some(self.map(li, other)),
            },
        };
        View { params, result }
    }

    fn is_by_ref(&self, li: usize, d: &TypeDesc) -> bool {
        match d {
            TypeDesc::Ptr(inner) => match &**inner {
                TypeDesc::User(r) => !self.is_object_type(li, *r),
                _ => true,
            },
            _ => false,
        }
    }

    fn is_object_type(&self, li: usize, r: TypeRef) -> bool {
        match self.target(li, r) {
            Some(Target::At(l, t)) => matches!(
                self.libs[l].types[t].kind,
                TypeKind::Interface | TypeKind::Dispatch | TypeKind::CoClass
            ),
            _ => true,
        }
    }

    /// Maps a type description to a VBA type.
    fn map(&self, li: usize, d: &TypeDesc) -> Type {
        self.map_depth(li, d, 0)
    }

    fn map_depth(&self, li: usize, d: &TypeDesc, depth: usize) -> Type {
        if depth > MAX_DEPTH {
            return Type::Variant;
        }
        match d {
            TypeDesc::Prim(v) => prim(*v),
            TypeDesc::Ptr(inner) => match &**inner {
                TypeDesc::Prim(vt::VOID) => Type::Any,
                other => self.map_depth(li, other, depth + 1),
            },
            TypeDesc::SafeArray(e) => Type::Array {
                elem: Box::new(self.map_depth(li, e, depth + 1)),
                rank: None,
            },
            TypeDesc::CArray { elem, bounds } => Type::Array {
                elem: Box::new(self.map_depth(li, elem, depth + 1)),
                rank: u8::try_from(bounds.len()).ok(),
            },
            TypeDesc::User(r) => match self.target(li, *r) {
                Some(Target::At(l, t)) => {
                    let info = &self.libs[l].types[t];
                    match info.kind {
                        TypeKind::Enum => Type::LibEnum(self.lib_type(l, t)),
                        TypeKind::Record => Type::LibRecord(self.lib_type(l, t)),
                        TypeKind::Alias => match &info.alias {
                            Some(a) => self.map_depth(l, a, depth + 1),
                            None => Type::Variant,
                        },
                        TypeKind::Module | TypeKind::Union => Type::Variant,
                        _ => Type::Library(self.lib_type(l, t)),
                    }
                }
                Some(Target::Root) | None => Type::Object,
            },
        }
    }

    fn constant(&self, c: &Const) -> Option<(Variant, Type)> {
        Some(match &c.value {
            ConstValue::Int(i) => match i32::try_from(*i) {
                Ok(v) if c.vt == vt::I2 => i16::try_from(v)
                    .map_or((Variant::Long(v), Type::Long), |s| {
                        (Variant::Integer(s), Type::Integer)
                    }),
                Ok(v) => (Variant::Long(v), Type::Long),
                Err(_) => (Variant::LongLong(*i), Type::LongLong),
            },
            ConstValue::UInt(u) => match i32::try_from(*u) {
                Ok(v) => (Variant::Long(v), Type::Long),
                Err(_) => (Variant::LongLong(i64::try_from(*u).ok()?), Type::LongLong),
            },
            ConstValue::Real(r) => (Variant::Double(*r), Type::Double),
            ConstValue::Currency(c) => {
                (Variant::Currency(Currency::from_scaled(*c)), Type::Currency)
            }
            ConstValue::Bool(b) => (Variant::Boolean(*b), Type::Boolean),
            ConstValue::Str(s) => (Variant::String(VbaString::from(s.as_str())), Type::String),
            ConstValue::Nothing | ConstValue::Unsupported => return None,
        })
    }
}

enum Target {
    /// `IUnknown` / `IDispatch` (not modelled; they add no VBA members).
    Root,
    At(usize, usize),
}

struct View {
    params: Vec<LibParam>,
    /// `None` for a `Sub`.
    result: Option<Type>,
}

fn prim(v: u16) -> Type {
    match v {
        vt::I2 => Type::Integer,
        vt::I4 | vt::INT | vt::UINT | vt::UI4 | vt::UI2 | vt::ERROR | vt::HRESULT => Type::Long,
        vt::I1 | vt::UI1 => Type::Byte,
        vt::I8 | vt::UI8 => Type::LongLong,
        vt::R4 => Type::Single,
        vt::R8 => Type::Double,
        vt::CY => Type::Currency,
        vt::DATE => Type::Date,
        vt::BSTR | vt::LPSTR | vt::LPWSTR => Type::String,
        vt::BOOL => Type::Boolean,
        vt::DECIMAL => Type::Decimal,
        vt::DISPATCH | vt::UNKNOWN => Type::Object,
        _ => Type::Variant,
    }
}

/// The user-defined type an alias (possibly through pointers) designates.
fn alias_user(d: &TypeDesc) -> Option<TypeRef> {
    match d {
        TypeDesc::User(r) => Some(*r),
        TypeDesc::Ptr(inner) => alias_user(inner),
        _ => None,
    }
}

/// Converts a stored default to the representation of the parameter type.
fn default_for(v: Variant, ty: &Type) -> Option<Variant> {
    match ty.var_type() {
        Some(vt) if !matches!(ty, Type::Variant | Type::Object | Type::Library(_)) => {
            vba_value::convert::convert_to(&v, vt).ok()
        }
        _ => Some(v),
    }
}

fn type_ok_for_decl(t: &TypeInfo) -> bool {
    matches!(
        t.kind,
        TypeKind::Interface | TypeKind::Dispatch | TypeKind::CoClass | TypeKind::Record
    )
}

impl TypeProvider for TypelibProvider {
    fn resolve_type(&self, qualifier: Option<&str>, name: &str) -> Option<LibraryType> {
        for (li, lib) in self.libs.iter().enumerate() {
            if qualifier.is_some_and(|q| !lib.name.eq_ignore_ascii_case(q)) {
                continue;
            }
            let Some((ti, info)) = lib.type_by_name(name) else {
                continue;
            };
            if type_ok_for_decl(info) {
                return Some(self.lib_type(li, ti));
            }
            if info.kind == TypeKind::Alias
                && let Some(r) = info.alias.as_ref().and_then(alias_user)
                && let Some(Target::At(l, t)) = self.target(li, r)
                && type_ok_for_decl(&self.libs[l].types[t])
            {
                return Some(self.lib_type(l, t));
            }
        }
        None
    }

    fn has_library(&self, name: &str) -> bool {
        self.libs.iter().any(|l| l.name.eq_ignore_ascii_case(name))
    }

    fn resolve_declared(&self, qualifier: Option<&str>, name: &str) -> Option<Type> {
        for (li, lib) in self.libs.iter().enumerate() {
            if qualifier.is_some_and(|q| !lib.name.eq_ignore_ascii_case(q)) {
                continue;
            }
            let Some((ti, info)) = lib.type_by_name(name) else {
                continue;
            };
            let t = match info.kind {
                TypeKind::Enum => Type::LibEnum(self.lib_type(li, ti)),
                TypeKind::Record => Type::LibRecord(self.lib_type(li, ti)),
                TypeKind::Alias => self.map(li, &TypeDesc::User(TypeRef::Local(ti))),
                TypeKind::Interface | TypeKind::Dispatch | TypeKind::CoClass => {
                    Type::Library(self.lib_type(li, ti))
                }
                TypeKind::Module | TypeKind::Union => continue,
            };
            return Some(t);
        }
        None
    }

    fn enum_members(&self, ty: &LibraryType) -> Option<Vec<(String, i64)>> {
        let (li, ti) = self.find(ty)?;
        let info = &self.libs[li].types[ti];
        if info.kind != TypeKind::Enum {
            return None;
        }
        Some(
            info.variables
                .iter()
                .filter(|v| v.kind == VarKind::Const)
                .filter_map(|v| {
                    let value = match &v.value.as_ref()?.value {
                        ConstValue::Int(i) => *i,
                        ConstValue::UInt(u) => i64::try_from(*u).ok()?,
                        ConstValue::Bool(b) => i64::from(*b),
                        _ => return None,
                    };
                    Some((v.name.clone(), value))
                })
                .collect(),
        )
    }

    fn record(&self, ty: &LibraryType) -> Option<LibRecord> {
        let (li, ti) = self.find(ty)?;
        let info = &self.libs[li].types[ti];
        if info.kind != TypeKind::Record {
            return None;
        }
        Some(LibRecord {
            fields: info
                .variables
                .iter()
                .filter(|v| v.kind == VarKind::PerInstance)
                .map(|v| LibField {
                    name: v.name.clone(),
                    ty: self.map(li, &v.ty),
                    offset: v.offset,
                })
                .collect(),
            size: info.instance_size,
            alignment: u32::from(info.alignment),
        })
    }

    fn class_info(&self, ty: &LibraryType) -> Option<LibClass> {
        let (li, ti) = self.find(ty)?;
        let info = &self.libs[li].types[ti];
        match info.kind {
            TypeKind::CoClass => {
                let default_interface = info
                    .implements
                    .iter()
                    .find(|i| {
                        i.flags.contains(crate::model::ImplFlags::DEFAULT)
                            && !i.flags.contains(crate::model::ImplFlags::SOURCE)
                    })
                    .and_then(|i| match self.target(li, i.target) {
                        Some(Target::At(l, t)) => Some(self.lib_type(l, t)),
                        _ => None,
                    });
                Some(LibClass {
                    default_interface,
                    creatable: info.flags.contains(crate::model::TypeFlags::CAN_CREATE),
                })
            }
            TypeKind::Interface | TypeKind::Dispatch => Some(LibClass {
                default_interface: None,
                creatable: false,
            }),
            _ => None,
        }
    }

    fn member(&self, ty: &LibraryType, name: &str) -> Option<LibMember> {
        let (li, ti) = self.find(ty)?;
        self.members(li, ti)
            .0
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
            .cloned()
    }

    fn default_member(&self, ty: &LibraryType) -> Option<LibMember> {
        let (li, ti) = self.find(ty)?;
        self.members(li, ti)
            .0
            .iter()
            .find(|m| m.is_default)
            .cloned()
    }

    fn describes_members(&self, ty: &LibraryType) -> bool {
        self.find(ty).is_some_and(|(li, ti)| {
            let info = &self.libs[li].types[ti];
            type_ok_for_decl(info) && self.members(li, ti).1
        })
    }

    fn interface_members(&self, ty: &LibraryType) -> Option<Vec<LibMember>> {
        let (li, ti) = self.find(ty)?;
        let info = &self.libs[li].types[ti];
        if !matches!(info.kind, TypeKind::Interface | TypeKind::Dispatch) {
            return None;
        }
        let m = self.members(li, ti);
        m.1.then(|| m.0.clone())
    }

    fn events(&self, ty: &LibraryType) -> Option<Vec<LibEvent>> {
        let (li, ti) = self.find(ty)?;
        let info = &self.libs[li].types[ti];
        if info.kind != TypeKind::CoClass {
            return None;
        }
        let mut out = Vec::new();
        for i in info
            .implements
            .iter()
            .filter(|i| i.flags.contains(crate::model::ImplFlags::SOURCE))
        {
            let Some(Target::At(l, t)) = self.target(li, i.target) else {
                return None;
            };
            for f in &self.libs[l].types[t].functions {
                let v = self.view(l, f);
                out.push(LibEvent {
                    name: f.name.clone(),
                    params: v.params,
                });
            }
        }
        Some(out)
    }

    fn global(&self, name: &str) -> Option<LibGlobal> {
        for (li, lib) in self.libs.iter().enumerate() {
            for (ti, t) in lib.types.iter().enumerate() {
                match t.kind {
                    TypeKind::Enum | TypeKind::Module => {
                        for v in t.variables.iter().filter(|v| v.kind == VarKind::Const) {
                            if v.name.eq_ignore_ascii_case(name)
                                && let Some((val, ty)) =
                                    v.value.as_ref().and_then(|c| self.constant(c))
                            {
                                let ty = if t.kind == TypeKind::Enum
                                    && matches!(ty, Type::Long | Type::Integer)
                                {
                                    Type::LibEnum(self.lib_type(li, ti))
                                } else {
                                    ty
                                };
                                return Some(LibGlobal::Constant(val, ty));
                            }
                        }
                        if t.kind == TypeKind::Module {
                            let (funcs, _) = self.functions(li, ti);
                            let group: Vec<_> = funcs
                                .into_iter()
                                .filter(|(_, f)| {
                                    f.name.eq_ignore_ascii_case(name)
                                        && !f.flags.contains(FuncFlags::RESTRICTED)
                                })
                                .collect();
                            if let Some(m) = self.member_from_group(&group) {
                                return Some(LibGlobal::Function(m));
                            }
                        }
                    }
                    TypeKind::CoClass
                        if t.flags.contains(crate::model::TypeFlags::APPOBJECT)
                            && t.name.eq_ignore_ascii_case(name) =>
                    {
                        return Some(LibGlobal::Object(Type::Library(self.lib_type(li, ti))));
                    }
                    _ => {}
                }
            }
        }
        None
    }
}
