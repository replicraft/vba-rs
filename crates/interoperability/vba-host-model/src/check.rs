//! Validation and the checked form of a description.
//!
//! [`HostLibrary::check`] verifies a description and derives, once, the
//! signature of every member ([`MemberSig`]): its `DISPID`, the invoke
//! kinds it supports and its typed parameters. The `TypeProvider`, the
//! `Dispatch` objects and the tools all read these signatures, so they
//! cannot disagree.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use vba_automation::{DispId, InvokeFlags, MemberInfo, ParamInfo};
use vba_sema::{LibMember, LibMemberKind, LibParam, LibraryType, Type};
use vba_value::Variant;

use crate::model::{
    Access, ConstValue, HostClass, HostLibrary, HostMember, HostParam, MemberKind, TypeRef,
};

/// One problem found in a description. Validation reports all of them.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ModelError {
    /// A name is empty.
    #[error("{at}: the name is empty")]
    EmptyName {
        /// Where.
        at: String,
    },
    /// A name is not a VBA identifier.
    #[error("{at}: `{name}` is not a valid VBA identifier")]
    InvalidName {
        /// Where.
        at: String,
        /// The name.
        name: String,
    },
    /// A class or enumeration is named like an intrinsic type.
    #[error("{at}: `{name}` is an intrinsic type name")]
    ReservedName {
        /// Where.
        at: String,
        /// The name.
        name: String,
    },
    /// Two names in one scope are equal when case is ignored.
    #[error("{scope}: `{second}` duplicates `{first}` (names are case-insensitive)")]
    Duplicate {
        /// The scope (the library, a class, a member).
        scope: String,
        /// The first spelling.
        first: String,
        /// The second spelling.
        second: String,
    },
    /// A type name is neither intrinsic nor a class or enumeration of the
    /// library.
    #[error("{at}: unknown type `{name}`")]
    DanglingType {
        /// Where.
        at: String,
        /// The name.
        name: String,
    },
    /// A `ParamArray` parameter is not the last one.
    #[error("{at}: ParamArray `{param}` must be the last parameter")]
    ParamArrayNotLast {
        /// Where.
        at: String,
        /// The parameter.
        param: String,
    },
    /// A `ParamArray` parameter is not an optional `ByRef` array of
    /// `Variant` without a default.
    #[error("{at}: ParamArray `{param}` must be an optional ByRef Variant() without a default")]
    ParamArrayShape {
        /// Where.
        at: String,
        /// The parameter.
        param: String,
    },
    /// A required parameter follows an optional one.
    #[error("{at}: required parameter `{param}` follows an optional parameter")]
    RequiredAfterOptional {
        /// Where.
        at: String,
        /// The parameter.
        param: String,
    },
    /// A default value on a parameter that is not optional.
    #[error("{at}: parameter `{param}` has a default value but is not optional")]
    DefaultNotOptional {
        /// Where.
        at: String,
        /// The parameter.
        param: String,
    },
    /// A default value that does not fit the parameter type.
    #[error("{at}: the default of `{param}` does not fit type {ty}")]
    DefaultTypeMismatch {
        /// Where.
        at: String,
        /// The parameter.
        param: String,
        /// Its type.
        ty: String,
    },
    /// A number that JSON cannot carry (NaN, infinity).
    #[error("{at}: the number is not finite")]
    NonFiniteNumber {
        /// Where.
        at: String,
    },
    /// A `ByRef` parameter on a global function, whose arguments are
    /// passed by value.
    #[error("{at}: parameter `{param}` cannot be ByRef on a global function")]
    ByRefFunctionParam {
        /// Where.
        at: String,
        /// The parameter.
        param: String,
    },
    /// A global function that is a property.
    #[error("{at}: a global function must be a method")]
    FunctionNotMethod {
        /// Where.
        at: String,
    },
    /// A property without a type.
    #[error("{at}: a property needs a type")]
    PropertyWithoutType {
        /// Where.
        at: String,
    },
    /// More than one default member in a class.
    #[error("{class}: `{second}` and `{first}` are both the default member")]
    MultipleDefaultMembers {
        /// The class.
        class: String,
        /// The first.
        first: String,
        /// The second.
        second: String,
    },
    /// A default member that is a `Sub`.
    #[error("{at}: the default member must have a value type")]
    DefaultMemberIsSub {
        /// Where.
        at: String,
    },
    /// A `DISPID` used twice in one class.
    #[error("{class}: DISPID {id} is used by `{first}` and `{second}`")]
    DuplicateDispId {
        /// The class.
        class: String,
        /// The id.
        id: i32,
        /// The first member.
        first: String,
        /// The second member.
        second: String,
    },
    /// A `DISPID` that is reserved or contradicts the default-member flag.
    #[error("{at}: DISPID {id} is reserved or contradicts the default-member flag")]
    ReservedDispId {
        /// Where.
        at: String,
        /// The id.
        id: i32,
    },
}

/// All the problems of one description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelErrors(pub Vec<ModelError>);

impl fmt::Display for ModelErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{e}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ModelErrors {}

/// The case-insensitive key of a name.
pub(crate) fn key(name: &str) -> String {
    name.chars().flat_map(char::to_lowercase).collect()
}

fn is_identifier(name: &str) -> bool {
    let mut it = name.chars();
    let ok_first = it.next().is_some_and(char::is_alphabetic);
    ok_first && it.all(|c| c.is_alphanumeric() || c == '_') && name.chars().count() <= 255
}

/// One parameter, ready for binding and for tools.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamSig {
    /// Name.
    pub name: String,
    /// Declared type.
    pub ty: Type,
    /// The class the type names, for the check of object arguments.
    pub class: Option<String>,
    /// `Optional`.
    pub optional: bool,
    /// `ByRef`.
    pub by_ref: bool,
    /// `ParamArray`.
    pub param_array: bool,
    /// Default of an omitted optional argument.
    pub default: Option<Variant>,
}

/// The signature of one member, derived once from the description.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberSig {
    /// Name as described.
    pub name: String,
    /// The member id (`DISPID`).
    pub id: DispId,
    /// The invoke kinds the member answers.
    pub supports: InvokeFlags,
    /// Method or the property accessors that exist.
    pub kind: LibMemberKind,
    /// Parameters (without the value of a property put).
    pub params: Vec<ParamSig>,
    /// Result type, or value type of a property (`Variant` for a `Sub`).
    pub ty: Type,
    /// The class `ty` names, for the check of object results and values.
    pub class: Option<String>,
    /// `true` for a `Sub`.
    pub is_sub: bool,
    /// `true` for the default member.
    pub is_default: bool,
    /// Hidden from completion.
    pub hidden: bool,
    /// The parameters as `vba-automation` binds them.
    pub(crate) infos: Vec<ParamInfo>,
}

impl MemberSig {
    /// The `vba-automation` description of the member.
    pub fn to_member_info(&self) -> MemberInfo {
        MemberInfo::new(
            self.name.clone(),
            self.id,
            self.supports,
            self.infos.clone(),
        )
    }

    /// The `vba-sema` description of the member.
    pub fn to_lib_member(&self) -> LibMember {
        LibMember {
            name: self.name.clone(),
            kind: self.kind,
            params: self
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
            ty: self.ty.clone(),
            is_default: self.is_default,
            is_sub: self.is_sub,
            dispid: Some(self.id.0),
        }
    }
}

pub(crate) fn param_infos(params: &[ParamSig]) -> Vec<ParamInfo> {
    params
        .iter()
        .map(|p| {
            ParamInfo::required(&p.name)
                .with_optional(p.optional)
                .with_by_ref(p.by_ref)
                .with_param_array(p.param_array)
                .with_default_value(p.default.clone())
        })
        .collect()
}

/// An event signature.
#[derive(Debug, Clone, PartialEq)]
pub struct EventSig {
    /// Name.
    pub name: String,
    /// Parameters.
    pub params: Vec<ParamSig>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ValueRef {
    Const(usize),
    EnumMember(usize, usize),
    Function(usize),
    Global(usize),
}

#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) lib: HostLibrary,
    pub(crate) classes: HashMap<String, usize>,
    pub(crate) enums: HashMap<String, usize>,
    pub(crate) values: HashMap<String, ValueRef>,
    pub(crate) class_members: Vec<Vec<MemberSig>>,
    pub(crate) class_member_ix: Vec<HashMap<String, usize>>,
    pub(crate) class_id_ix: Vec<HashMap<i32, usize>>,
    pub(crate) class_events: Vec<Vec<EventSig>>,
    pub(crate) functions: Vec<MemberSig>,
}

/// A validated description with its derived signatures. Cheap to clone and
/// `Send + Sync`, so one value serves the analyzer, the runtime and a
/// language server.
#[derive(Debug, Clone)]
pub struct CheckedLibrary {
    pub(crate) inner: Arc<Inner>,
}

impl HostLibrary {
    /// Validates the description: names (non-empty, identifiers,
    /// case-insensitively unique per scope), type references, parameter
    /// lists, default values, default members and `DISPID`s.
    ///
    /// Returns every problem found, not only the first.
    pub fn check(self) -> Result<CheckedLibrary, ModelErrors> {
        let mut errors = Vec::new();
        validate(&self, &mut errors);
        if !errors.is_empty() {
            return Err(ModelErrors(errors));
        }
        Ok(CheckedLibrary {
            inner: Arc::new(derive(self)),
        })
    }

    /// The validation problems without consuming the description.
    pub fn problems(&self) -> Vec<ModelError> {
        let mut errors = Vec::new();
        validate(self, &mut errors);
        errors
    }
}

struct Names<'a> {
    scope: String,
    seen: HashMap<String, &'a str>,
}

impl<'a> Names<'a> {
    fn new(scope: impl Into<String>) -> Self {
        Names {
            scope: scope.into(),
            seen: HashMap::new(),
        }
    }

    fn add(&mut self, name: &'a str, errors: &mut Vec<ModelError>) {
        if let Some(first) = self.seen.get(&key(name)) {
            errors.push(ModelError::Duplicate {
                scope: self.scope.clone(),
                first: (*first).to_owned(),
                second: name.to_owned(),
            });
        } else {
            self.seen.insert(key(name), name);
        }
    }
}

fn check_name(at: &str, name: &str, errors: &mut Vec<ModelError>) -> bool {
    if name.is_empty() {
        errors.push(ModelError::EmptyName { at: at.to_owned() });
        false
    } else if !is_identifier(name) {
        errors.push(ModelError::InvalidName {
            at: at.to_owned(),
            name: name.to_owned(),
        });
        false
    } else {
        true
    }
}

fn type_exists(lib: &HostLibrary, t: &TypeRef) -> Result<(), String> {
    match t {
        TypeRef::Named(n) => {
            let k = key(n);
            let found = lib.classes.iter().any(|c| key(&c.name) == k)
                || lib.enums.iter().any(|e| key(&e.name) == k);
            if found { Ok(()) } else { Err(n.clone()) }
        }
        TypeRef::Array(e) => type_exists(lib, e),
        _ => Ok(()),
    }
}

fn check_type(lib: &HostLibrary, at: &str, t: &TypeRef, errors: &mut Vec<ModelError>) {
    if let Err(name) = type_exists(lib, t) {
        errors.push(ModelError::DanglingType {
            at: at.to_owned(),
            name,
        });
    }
}

fn default_fits(lib: &HostLibrary, ty: &TypeRef, v: &ConstValue) -> bool {
    use ConstValue as C;
    match (ty, v) {
        (TypeRef::Variant, _) => true,
        (TypeRef::Boolean, C::Boolean(_)) => true,
        (
            TypeRef::Byte | TypeRef::Integer | TypeRef::Long | TypeRef::LongLong,
            C::Integer(_) | C::Long(_) | C::Boolean(_),
        ) => true,
        (
            TypeRef::Single | TypeRef::Double | TypeRef::Currency,
            C::Integer(_) | C::Long(_) | C::Double(_),
        ) => true,
        (TypeRef::String | TypeRef::Date, C::String(_)) => true,
        (TypeRef::Named(n), C::Integer(_) | C::Long(_)) => {
            lib.enums.iter().any(|e| key(&e.name) == key(n))
        }
        _ => false,
    }
}

fn check_params(
    lib: &HostLibrary,
    at: &str,
    params: &[HostParam],
    function: bool,
    errors: &mut Vec<ModelError>,
) {
    let mut names = Names::new(at);
    let mut seen_optional = false;
    for (i, p) in params.iter().enumerate() {
        let pat = format!("{at}({})", p.name);
        check_name(&pat, &p.name, errors);
        names.add(&p.name, errors);
        check_type(lib, &pat, &p.ty, errors);
        if p.param_array {
            if i + 1 != params.len() {
                errors.push(ModelError::ParamArrayNotLast {
                    at: at.to_owned(),
                    param: p.name.clone(),
                });
            }
            let shape_ok = p.optional
                && p.by_ref
                && p.default.is_none()
                && p.ty == TypeRef::array_of(TypeRef::Variant);
            if !shape_ok {
                errors.push(ModelError::ParamArrayShape {
                    at: at.to_owned(),
                    param: p.name.clone(),
                });
            }
        } else {
            if p.optional {
                seen_optional = true;
            } else if seen_optional {
                errors.push(ModelError::RequiredAfterOptional {
                    at: at.to_owned(),
                    param: p.name.clone(),
                });
            }
            if let Some(d) = &p.default {
                if !p.optional {
                    errors.push(ModelError::DefaultNotOptional {
                        at: at.to_owned(),
                        param: p.name.clone(),
                    });
                } else if !default_fits(lib, &p.ty, d) {
                    errors.push(ModelError::DefaultTypeMismatch {
                        at: at.to_owned(),
                        param: p.name.clone(),
                        ty: p.ty.to_string(),
                    });
                }
                if matches!(d, ConstValue::Double(x) if !x.is_finite()) {
                    errors.push(ModelError::NonFiniteNumber { at: pat.clone() });
                }
            }
        }
        if function && p.by_ref {
            errors.push(ModelError::ByRefFunctionParam {
                at: at.to_owned(),
                param: p.name.clone(),
            });
        }
    }
}

fn validate(lib: &HostLibrary, errors: &mut Vec<ModelError>) {
    check_name("library", &lib.name, errors);

    let mut types = Names::new(format!("library {}: types", lib.name));
    for c in &lib.classes {
        let at = format!("class {}", c.name);
        if check_name(&at, &c.name, errors) && TypeRef::is_intrinsic_name(&c.name) {
            errors.push(ModelError::ReservedName {
                at,
                name: c.name.clone(),
            });
        }
        types.add(&c.name, errors);
    }
    for e in &lib.enums {
        let at = format!("enum {}", e.name);
        if check_name(&at, &e.name, errors) && TypeRef::is_intrinsic_name(&e.name) {
            errors.push(ModelError::ReservedName {
                at,
                name: e.name.clone(),
            });
        }
        types.add(&e.name, errors);
    }

    let mut values = Names::new(format!("library {}: global names", lib.name));
    for c in &lib.consts {
        let at = format!("constant {}", c.name);
        check_name(&at, &c.name, errors);
        values.add(&c.name, errors);
        if matches!(c.value, ConstValue::Double(x) if !x.is_finite()) {
            errors.push(ModelError::NonFiniteNumber { at });
        }
    }
    for e in &lib.enums {
        let mut own = Names::new(format!("enum {}", e.name));
        for (n, _, _) in &e.members {
            check_name(&format!("enum {}", e.name), n, errors);
            own.add(n, errors);
            values.add(n, errors);
        }
    }
    for g in &lib.globals {
        let at = format!("global {}", g.name);
        check_name(&at, &g.name, errors);
        values.add(&g.name, errors);
        match &g.ty {
            TypeRef::Named(n) if lib.classes.iter().any(|c| key(&c.name) == key(n)) => {}
            TypeRef::Object => {}
            other => errors.push(ModelError::DanglingType {
                at,
                name: other.to_string(),
            }),
        }
    }
    for f in &lib.functions {
        let at = format!("function {}", f.name);
        check_name(&at, &f.name, errors);
        values.add(&f.name, errors);
        if f.kind != MemberKind::Method {
            errors.push(ModelError::FunctionNotMethod { at: at.clone() });
        }
        if let Some(t) = &f.ty {
            check_type(lib, &at, t, errors);
        }
        check_params(lib, &at, &f.params, true, errors);
    }

    for c in &lib.classes {
        validate_class(lib, c, errors);
    }
}

fn validate_class(lib: &HostLibrary, c: &HostClass, errors: &mut Vec<ModelError>) {
    let mut members = Names::new(format!("class {}", c.name));
    let mut default: Option<&str> = None;
    let mut ids: HashMap<i32, &str> = HashMap::new();
    if let Some(t) = &c.enumerable {
        check_type(lib, &format!("class {}: enumerable", c.name), t, errors);
    }
    for m in &c.members {
        let at = format!("{}.{}", c.name, m.name);
        check_name(&at, &m.name, errors);
        members.add(&m.name, errors);
        match (&m.kind, &m.ty) {
            (MemberKind::Property, None) => {
                errors.push(ModelError::PropertyWithoutType { at: at.clone() })
            }
            (_, Some(t)) => check_type(lib, &at, t, errors),
            _ => {}
        }
        check_params(lib, &at, &m.params, false, errors);
        if m.default_member {
            if m.ty.is_none() {
                errors.push(ModelError::DefaultMemberIsSub { at: at.clone() });
            }
            if let Some(first) = default {
                errors.push(ModelError::MultipleDefaultMembers {
                    class: c.name.clone(),
                    first: first.to_owned(),
                    second: m.name.clone(),
                });
            } else {
                default = Some(&m.name);
            }
        }
        if let Some(id) = m.dispid {
            let reserved = id == DispId::NEWENUM.0 || (id == 0) != m.default_member;
            if reserved {
                errors.push(ModelError::ReservedDispId { at: at.clone(), id });
            } else if let Some(first) = ids.insert(id, &m.name) {
                errors.push(ModelError::DuplicateDispId {
                    class: c.name.clone(),
                    id,
                    first: first.to_owned(),
                    second: m.name.clone(),
                });
            }
        }
    }
    let mut events = Names::new(format!("class {}: events", c.name));
    for e in &c.events {
        let at = format!("{} event {}", c.name, e.name);
        check_name(&at, &e.name, errors);
        events.add(&e.name, errors);
        check_params(lib, &at, &e.params, false, errors);
    }
}

fn derive(lib: HostLibrary) -> Inner {
    let mut inner = Inner {
        classes: HashMap::new(),
        enums: HashMap::new(),
        values: HashMap::new(),
        class_members: Vec::new(),
        class_member_ix: Vec::new(),
        class_id_ix: Vec::new(),
        class_events: Vec::new(),
        functions: Vec::new(),
        lib: HostLibrary::new(""),
    };
    for (i, c) in lib.classes.iter().enumerate() {
        inner.classes.insert(key(&c.name), i);
    }
    for (i, e) in lib.enums.iter().enumerate() {
        inner.enums.insert(key(&e.name), i);
    }
    for (i, c) in lib.consts.iter().enumerate() {
        inner.values.insert(key(&c.name), ValueRef::Const(i));
    }
    for (i, e) in lib.enums.iter().enumerate() {
        for (j, (n, _, _)) in e.members.iter().enumerate() {
            inner.values.insert(key(n), ValueRef::EnumMember(i, j));
        }
    }
    for (i, f) in lib.functions.iter().enumerate() {
        inner.values.insert(key(&f.name), ValueRef::Function(i));
    }
    for (i, g) in lib.globals.iter().enumerate() {
        inner.values.insert(key(&g.name), ValueRef::Global(i));
    }

    let ctx = Ctx { lib: &lib };
    for c in &lib.classes {
        let mut used: Vec<i32> = c.members.iter().filter_map(|m| m.dispid).collect();
        used.push(0);
        used.push(DispId::NEWENUM.0);
        let mut next = 1;
        let mut sigs = Vec::new();
        for m in &c.members {
            let id = match m.dispid {
                Some(id) => id,
                None if m.default_member => 0,
                None => {
                    while used.contains(&next) {
                        next += 1;
                    }
                    used.push(next);
                    next
                }
            };
            sigs.push(ctx.member_sig(m, DispId(id), false));
        }
        inner.class_member_ix.push(
            sigs.iter()
                .enumerate()
                .map(|(i, s)| (key(&s.name), i))
                .collect(),
        );
        inner
            .class_id_ix
            .push(sigs.iter().enumerate().map(|(i, s)| (s.id.0, i)).collect());
        inner.class_members.push(sigs);
        inner.class_events.push(
            c.events
                .iter()
                .map(|e| EventSig {
                    name: e.name.clone(),
                    params: e.params.iter().map(|p| ctx.param_sig(p, false)).collect(),
                })
                .collect(),
        );
    }
    inner.functions = lib
        .functions
        .iter()
        .enumerate()
        .map(|(i, f)| ctx.member_sig(f, DispId(i as i32 + 1), true))
        .collect();
    inner.lib = lib;
    inner
}

pub(crate) fn sema_type(lib: &HostLibrary, t: &TypeRef) -> Type {
    match t {
        TypeRef::Boolean => Type::Boolean,
        TypeRef::Byte => Type::Byte,
        TypeRef::Integer => Type::Integer,
        TypeRef::Long => Type::Long,
        TypeRef::LongLong => Type::LongLong,
        TypeRef::Single => Type::Single,
        TypeRef::Double => Type::Double,
        TypeRef::Currency => Type::Currency,
        TypeRef::Date => Type::Date,
        TypeRef::String => Type::String,
        TypeRef::Variant => Type::Variant,
        TypeRef::Object => Type::Object,
        TypeRef::Named(n) => {
            if let Some(c) = lib.classes.iter().find(|c| key(&c.name) == key(n)) {
                Type::Library(LibraryType {
                    library: lib.name.clone(),
                    name: c.name.clone(),
                })
            } else if let Some(e) = lib.enums.iter().find(|e| key(&e.name) == key(n)) {
                Type::LibEnum(LibraryType {
                    library: lib.name.clone(),
                    name: e.name.clone(),
                })
            } else {
                Type::Error
            }
        }
        TypeRef::Array(e) => Type::array_of(sema_type(lib, e), None),
    }
}

struct Ctx<'a> {
    lib: &'a HostLibrary,
}

impl Ctx<'_> {
    fn class_named(&self, t: &TypeRef) -> Option<String> {
        match t {
            TypeRef::Named(n) => self
                .lib
                .classes
                .iter()
                .find(|c| key(&c.name) == key(n))
                .map(|c| c.name.clone()),
            _ => None,
        }
    }

    fn sema_type(&self, t: &TypeRef) -> Type {
        sema_type(self.lib, t)
    }

    fn param_sig(&self, p: &HostParam, function: bool) -> ParamSig {
        ParamSig {
            name: p.name.clone(),
            ty: self.sema_type(&p.ty),
            class: self.class_named(&p.ty),
            optional: p.optional,
            by_ref: p.by_ref && !function,
            param_array: p.param_array,
            default: p.default.as_ref().map(ConstValue::to_variant),
        }
    }

    fn member_sig(&self, m: &HostMember, id: DispId, function: bool) -> MemberSig {
        let ty = m.ty.as_ref().map_or(Type::Variant, |t| self.sema_type(t));
        let class = m.ty.as_ref().and_then(|t| self.class_named(t));
        let is_object = class.is_some() || matches!(m.ty, Some(TypeRef::Object));
        let is_variant = matches!(m.ty, Some(TypeRef::Variant));
        let read = InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET;
        let put = InvokeFlags::PROPERTY_PUT;
        let put_ref = InvokeFlags::PROPERTY_PUT_REF;
        let writes = match (is_object, is_variant) {
            (true, _) => put_ref,
            (_, true) => put | put_ref,
            _ => put,
        };
        let (supports, kind) = match (m.kind, m.access) {
            (MemberKind::Method, _) => (read, LibMemberKind::Method),
            (MemberKind::Property, Access::ReadOnly) => (read, LibMemberKind::PropertyGet),
            (MemberKind::Property, Access::ReadWrite) => (read | writes, LibMemberKind::Property),
            (MemberKind::Property, Access::WriteOnly) => (writes, LibMemberKind::PropertyPut),
        };
        let params: Vec<ParamSig> = m
            .params
            .iter()
            .map(|p| self.param_sig(p, function))
            .collect();
        MemberSig {
            name: m.name.clone(),
            id,
            supports,
            kind,
            infos: param_infos(&params),
            params,
            ty,
            class,
            is_sub: m.is_sub(),
            is_default: m.default_member,
            hidden: m.hidden,
        }
    }
}

impl CheckedLibrary {
    /// The description this was made from.
    pub fn library(&self) -> &HostLibrary {
        &self.inner.lib
    }

    /// The library name.
    pub fn name(&self) -> &str {
        &self.inner.lib.name
    }

    /// A class by name (case-insensitive).
    pub fn class(&self, name: &str) -> Option<&HostClass> {
        self.class_index(name).map(|i| &self.inner.lib.classes[i])
    }

    pub(crate) fn class_index(&self, name: &str) -> Option<usize> {
        self.inner.classes.get(&key(name)).copied()
    }

    pub(crate) fn enum_index(&self, name: &str) -> Option<usize> {
        self.inner.enums.get(&key(name)).copied()
    }

    /// The signatures of a class's members, in declaration order.
    pub fn class_members(&self, class: &str) -> Option<&[MemberSig]> {
        self.class_index(class)
            .map(|i| self.inner.class_members[i].as_slice())
    }

    /// The signature of one member of a class.
    pub fn class_member(&self, class: &str, name: &str) -> Option<&MemberSig> {
        let i = self.class_index(class)?;
        let j = *self.inner.class_member_ix[i].get(&key(name))?;
        Some(&self.inner.class_members[i][j])
    }

    /// The signature of the member of a class with this `DISPID`.
    pub fn class_member_by_id(&self, class: &str, id: DispId) -> Option<&MemberSig> {
        let i = self.class_index(class)?;
        let j = *self.inner.class_id_ix[i].get(&id.0)?;
        Some(&self.inner.class_members[i][j])
    }

    /// The signatures of a class's events.
    pub fn class_events(&self, class: &str) -> Option<&[EventSig]> {
        self.class_index(class)
            .map(|i| self.inner.class_events[i].as_slice())
    }

    /// The signatures of the global functions.
    pub fn functions(&self) -> &[MemberSig] {
        &self.inner.functions
    }

    /// The signature of a global function (case-insensitive).
    pub fn function(&self, name: &str) -> Option<&MemberSig> {
        match self.inner.values.get(&key(name))? {
            ValueRef::Function(i) => Some(&self.inner.functions[*i]),
            _ => None,
        }
    }
}
