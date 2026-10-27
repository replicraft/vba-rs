//! The injected view of external libraries (host object models, COM type
//! libraries).
//!
//! The analyzer never guesses about library types: a name the provider does
//! not know is reported as `SEM004`, and a member of a library type that the
//! provider cannot describe is bound late with an informational `SEM005`.

use std::collections::HashMap;

use vba_value::Variant;

use crate::types::{LibraryType, Type};

/// A parameter of a library member.
#[derive(Debug, Clone, PartialEq)]
pub struct LibParam {
    /// Name (used for named arguments).
    pub name: String,
    /// Declared type.
    pub ty: Type,
    /// `Optional`.
    pub optional: bool,
    /// Passed `ByRef`.
    pub by_ref: bool,
    /// `ParamArray`.
    pub param_array: bool,
    /// The default value the library declares for an omitted `Optional`
    /// argument (`[optional, defaultvalue(...)]`). Early-bound calls pass it
    /// explicitly; without one the argument is passed as missing.
    pub default: Option<Variant>,
}

impl LibParam {
    /// A required `ByVal`-style parameter.
    pub fn required(name: &str, ty: Type) -> LibParam {
        LibParam {
            name: name.to_owned(),
            ty,
            optional: false,
            by_ref: false,
            param_array: false,
            default: None,
        }
    }

    /// An optional parameter with a declared default value.
    #[must_use]
    pub fn with_default(mut self, v: Variant) -> LibParam {
        self.optional = true;
        self.default = Some(v);
        self
    }

    /// An optional parameter.
    pub fn optional(name: &str, ty: Type) -> LibParam {
        LibParam {
            optional: true,
            ..LibParam::required(name, ty)
        }
    }
}

/// What kind of member a library type has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibMemberKind {
    /// A method (`Sub` or `Function`).
    Method,
    /// A read-only property.
    PropertyGet,
    /// A read/write property.
    Property,
    /// A write-only property.
    PropertyPut,
}

/// A member of a library type.
#[derive(Debug, Clone, PartialEq)]
pub struct LibMember {
    /// Name.
    pub name: String,
    /// Kind.
    pub kind: LibMemberKind,
    /// Parameters (index arguments for properties).
    pub params: Vec<LibParam>,
    /// Result type (property type, function result; `Variant` for `Sub`s).
    pub ty: Type,
    /// Whether the member is the type's default member (DISPID 0).
    pub is_default: bool,
    /// Whether a `Sub` (no result).
    pub is_sub: bool,
    /// The member id (`DISPID`) when the library records one. With an id,
    /// calls on a statically typed receiver are early-bound: the run time
    /// invokes the member by id instead of looking its name up.
    pub dispid: Option<i32>,
}

impl LibMember {
    /// A read/write property.
    pub fn property(name: &str, ty: Type) -> LibMember {
        LibMember {
            name: name.to_owned(),
            kind: LibMemberKind::Property,
            params: Vec::new(),
            ty,
            is_default: false,
            is_sub: false,
            dispid: None,
        }
    }

    /// A function.
    pub fn function(name: &str, params: Vec<LibParam>, ty: Type) -> LibMember {
        LibMember {
            name: name.to_owned(),
            kind: LibMemberKind::Method,
            params,
            ty,
            is_default: false,
            is_sub: false,
            dispid: None,
        }
    }

    /// A subroutine.
    pub fn sub(name: &str, params: Vec<LibParam>) -> LibMember {
        LibMember {
            is_sub: true,
            ..LibMember::function(name, params, Type::Variant)
        }
    }

    /// Marks the member as the default member.
    #[must_use]
    pub fn default_member(mut self) -> LibMember {
        self.is_default = true;
        self
    }

    /// Sets the member id used for early binding.
    #[must_use]
    pub fn with_dispid(mut self, id: i32) -> LibMember {
        self.dispid = Some(id);
        self
    }

    /// Makes the member a read-only property.
    #[must_use]
    pub fn read_only(mut self) -> LibMember {
        if self.kind == LibMemberKind::Property {
            self.kind = LibMemberKind::PropertyGet;
        }
        self
    }
}

/// An event of a library type.
#[derive(Debug, Clone, PartialEq)]
pub struct LibEvent {
    /// Name.
    pub name: String,
    /// Parameters.
    pub params: Vec<LibParam>,
}

/// A field of a library record.
#[derive(Debug, Clone, PartialEq)]
pub struct LibField {
    /// Name.
    pub name: String,
    /// Declared type.
    pub ty: Type,
    /// Byte offset in the record, when the library gives one.
    pub offset: Option<u32>,
}

/// A record (UDT) of a library: its fields in declaration order and layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LibRecord {
    /// Fields in declaration order.
    pub fields: Vec<LibField>,
    /// Size of an instance in bytes (`0` when unknown).
    pub size: u32,
    /// Alignment in bytes (`0` when unknown).
    pub alignment: u32,
}

/// What a library says about a coclass.
#[derive(Debug, Clone, PartialEq)]
pub struct LibClass {
    /// The default (non-source) interface, whose members the class exposes.
    pub default_interface: Option<LibraryType>,
    /// Whether `New` may create instances (`cancreate`).
    pub creatable: bool,
}

/// A global name provided by a library.
#[derive(Debug, Clone, PartialEq)]
pub enum LibGlobal {
    /// A constant (`vbCrLf`, `xlUp`).
    Constant(Variant, Type),
    /// A global object (`Application`, `Err`).
    Object(Type),
    /// A global function or sub (`MsgBox`).
    Function(LibMember),
}

/// What kind of top-level name a library offers (see
/// [`TypeProvider::catalog`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibEntryKind {
    /// A class or interface that can follow `As`.
    Class,
    /// An enumeration that can follow `As`.
    Enum,
    /// A constant or an enumeration member.
    Constant,
    /// A global function or sub.
    Function,
    /// A predeclared global object.
    Global,
}

/// One top-level name of a library, for tools that list what can be typed
/// (completion, symbol search).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibEntry {
    /// The library the name belongs to.
    pub library: String,
    /// The name, as the library spells it.
    pub name: String,
    /// What the name is.
    pub kind: LibEntryKind,
}

/// Knowledge about external libraries, injected by the embedder.
///
/// Every method has a "does not know" default so that a provider only
/// describes what it can describe; the analyzer turns missing knowledge
/// into explicit diagnostics, never into guesses.
pub trait TypeProvider {
    /// Resolves a type name, with an optional library qualifier
    /// (`Excel.Range` is `(Some("Excel"), "Range")`).
    fn resolve_type(&self, qualifier: Option<&str>, name: &str) -> Option<LibraryType>;

    /// Resolves the type named in an `As` clause. This differs from
    /// [`resolve_type`](Self::resolve_type) when the name is an enum
    /// ([`Type::LibEnum`]), a record ([`Type::LibRecord`]) or an alias, which
    /// resolves to the type it stands for (an alias of `Long` is `Long`).
    /// The default treats every resolved type as an object type.
    fn resolve_declared(&self, qualifier: Option<&str>, name: &str) -> Option<Type> {
        self.resolve_type(qualifier, name).map(Type::Library)
    }

    /// Whether `name` is the name of a library this provider serves, so that
    /// `Library.Name` can qualify constants and enum names.
    fn has_library(&self, _name: &str) -> bool {
        false
    }

    /// The named members of a library enum, with their values.
    fn enum_members(&self, _ty: &LibraryType) -> Option<Vec<(String, i64)>> {
        None
    }

    /// The fields and layout of a library record.
    fn record(&self, _ty: &LibraryType) -> Option<LibRecord> {
        None
    }

    /// Facts about a coclass; `None` for types that are not coclasses or are
    /// not described.
    fn class_info(&self, _ty: &LibraryType) -> Option<LibClass> {
        None
    }

    /// Looks a member up. `None` means "not described", unless
    /// [`describes_members`](Self::describes_members) is `true`, in which
    /// case it means "does not exist".
    fn member(&self, _ty: &LibraryType, _name: &str) -> Option<LibMember> {
        None
    }

    /// The default member of a type, if described.
    fn default_member(&self, _ty: &LibraryType) -> Option<LibMember> {
        None
    }

    /// Whether the provider lists all members of `ty`.
    fn describes_members(&self, _ty: &LibraryType) -> bool {
        false
    }

    /// All members of an interface type, for `Implements` completeness
    /// checks. `None` if the interface cannot be enumerated.
    fn interface_members(&self, _ty: &LibraryType) -> Option<Vec<LibMember>> {
        None
    }

    /// The events a type raises, for `WithEvents`. `None` if unknown.
    fn events(&self, _ty: &LibraryType) -> Option<Vec<LibEvent>> {
        None
    }

    /// Whether `ty` may be named in an `Implements` statement. `None`: the
    /// provider does not say, and the statement is accepted (a type library
    /// interface is the usual case). `Some(false)`: `ty` is a described
    /// class that is not meant to be implemented.
    fn is_interface(&self, _ty: &LibraryType) -> Option<bool> {
        None
    }

    /// The element type of `For Each` over a value of `ty`, when the
    /// provider describes it (a collection of controls enumerates
    /// `Control`s).
    fn enumerable_item(&self, _ty: &LibraryType) -> Option<Type> {
        None
    }

    /// Whether `ty` is described completely and has no subtypes or
    /// alternative interfaces beyond what the provider lists: a value of
    /// another such type can never be used where `ty` is required, so the
    /// analysis reports the assignment instead of leaving it to a run-time
    /// cast. The default is `false` (type libraries have inheritance and
    /// `QueryInterface`).
    fn is_exact_class(&self, _ty: &LibraryType) -> bool {
        false
    }

    /// A global name of a referenced library.
    fn global(&self, _name: &str) -> Option<LibGlobal> {
        None
    }

    /// Every top-level name the provider offers: types, enumerations,
    /// constants, global functions and predeclared objects. Analysis never
    /// needs it; editors use it to list what can be typed. The default
    /// lists nothing.
    fn catalog(&self) -> Vec<LibEntry> {
        Vec::new()
    }

    /// Documentation text, for hover and signature help. `(Some(ty), None)`
    /// documents a type, `(Some(ty), Some(m))` one of its members and
    /// `(None, Some(n))` a global name (constant, function, object). The
    /// default has none.
    fn documentation(&self, _ty: Option<&LibraryType>, _member: Option<&str>) -> Option<String> {
        None
    }
}

/// A provider that knows nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLibraries;

impl TypeProvider for NoLibraries {
    fn resolve_type(&self, _: Option<&str>, _: &str) -> Option<LibraryType> {
        None
    }
}

/// A simple table-driven provider, useful for tests and small hosts.
#[derive(Debug, Clone, Default)]
pub struct StaticLibrary {
    library: String,
    types: HashMap<String, StaticType>,
    globals: HashMap<String, LibGlobal>,
}

#[derive(Debug, Clone, Default)]
struct StaticType {
    members: HashMap<String, LibMember>,
    events: Vec<LibEvent>,
    complete: bool,
    interface: bool,
    shape: Shape,
    creatable: Option<bool>,
}

/// What a [`StaticLibrary`] type is.
#[derive(Debug, Clone, Default)]
enum Shape {
    #[default]
    Object,
    Enum(Vec<(String, i64)>),
    Record(LibRecord),
    Alias(Type),
}

impl StaticLibrary {
    /// Creates a library with the given name.
    pub fn new(library: &str) -> StaticLibrary {
        StaticLibrary {
            library: library.to_owned(),
            ..StaticLibrary::default()
        }
    }

    /// Declares a type whose members are all listed.
    #[must_use]
    pub fn with_type(mut self, name: &str, members: Vec<LibMember>) -> StaticLibrary {
        self.types.insert(
            name.to_ascii_lowercase(),
            StaticType {
                members: members
                    .into_iter()
                    .map(|m| (m.name.to_ascii_lowercase(), m))
                    .collect(),
                complete: true,
                ..StaticType::default()
            },
        );
        self
    }

    /// Declares a type whose members are not described.
    #[must_use]
    pub fn with_opaque_type(mut self, name: &str) -> StaticLibrary {
        self.types
            .insert(name.to_ascii_lowercase(), StaticType::default());
        self
    }

    /// Marks an already declared type as an interface (enumerable members).
    #[must_use]
    pub fn as_interface(mut self, name: &str) -> StaticLibrary {
        if let Some(t) = self.types.get_mut(&name.to_ascii_lowercase()) {
            t.interface = true;
        }
        self
    }

    /// Declares an enum with named members.
    #[must_use]
    pub fn with_enum(mut self, name: &str, members: &[(&str, i64)]) -> StaticLibrary {
        self.types.insert(
            name.to_ascii_lowercase(),
            StaticType {
                shape: Shape::Enum(members.iter().map(|(n, v)| ((*n).to_owned(), *v)).collect()),
                complete: true,
                ..StaticType::default()
            },
        );
        for (n, v) in members {
            self.globals.insert(
                n.to_ascii_lowercase(),
                LibGlobal::Constant(
                    Variant::Long(i32::try_from(*v).unwrap_or_default()),
                    Type::LibEnum(LibraryType {
                        library: self.library.clone(),
                        name: name.to_owned(),
                    }),
                ),
            );
        }
        self
    }

    /// Declares a record with the given fields.
    #[must_use]
    pub fn with_record(mut self, name: &str, fields: Vec<LibField>) -> StaticLibrary {
        self.types.insert(
            name.to_ascii_lowercase(),
            StaticType {
                members: fields
                    .iter()
                    .map(|f| {
                        (
                            f.name.to_ascii_lowercase(),
                            LibMember::property(&f.name, f.ty.clone()),
                        )
                    })
                    .collect(),
                shape: Shape::Record(LibRecord {
                    fields,
                    size: 0,
                    alignment: 0,
                }),
                complete: true,
                ..StaticType::default()
            },
        );
        self
    }

    /// Declares an alias of another type.
    #[must_use]
    pub fn with_alias(mut self, name: &str, target: Type) -> StaticLibrary {
        self.types.insert(
            name.to_ascii_lowercase(),
            StaticType {
                shape: Shape::Alias(target),
                complete: true,
                ..StaticType::default()
            },
        );
        self
    }

    /// Records whether `New` may create instances of a declared type.
    #[must_use]
    pub fn with_creatable(mut self, name: &str, creatable: bool) -> StaticLibrary {
        if let Some(t) = self.types.get_mut(&name.to_ascii_lowercase()) {
            t.creatable = Some(creatable);
        }
        self
    }

    /// Adds events to a declared type.
    #[must_use]
    pub fn with_events(mut self, name: &str, events: Vec<LibEvent>) -> StaticLibrary {
        if let Some(t) = self.types.get_mut(&name.to_ascii_lowercase()) {
            t.events = events;
        }
        self
    }

    /// Adds a global.
    #[must_use]
    pub fn with_global(mut self, name: &str, g: LibGlobal) -> StaticLibrary {
        self.globals.insert(name.to_ascii_lowercase(), g);
        self
    }

    fn ty(&self, t: &LibraryType) -> Option<&StaticType> {
        if !t.library.eq_ignore_ascii_case(&self.library) {
            return None;
        }
        self.types.get(&t.name.to_ascii_lowercase())
    }
}

impl TypeProvider for StaticLibrary {
    fn resolve_type(&self, qualifier: Option<&str>, name: &str) -> Option<LibraryType> {
        if let Some(q) = qualifier
            && !q.eq_ignore_ascii_case(&self.library)
        {
            return None;
        }
        self.types
            .get(&name.to_ascii_lowercase())
            .map(|_| LibraryType {
                library: self.library.clone(),
                name: name.to_owned(),
            })
    }

    fn has_library(&self, name: &str) -> bool {
        self.library.eq_ignore_ascii_case(name)
    }

    fn resolve_declared(&self, qualifier: Option<&str>, name: &str) -> Option<Type> {
        let lt = self.resolve_type(qualifier, name)?;
        Some(match &self.types.get(&name.to_ascii_lowercase())?.shape {
            Shape::Object => Type::Library(lt),
            Shape::Enum(_) => Type::LibEnum(lt),
            Shape::Record(_) => Type::LibRecord(lt),
            Shape::Alias(t) => t.clone(),
        })
    }

    fn enum_members(&self, ty: &LibraryType) -> Option<Vec<(String, i64)>> {
        match &self.ty(ty)?.shape {
            Shape::Enum(m) => Some(m.clone()),
            _ => None,
        }
    }

    fn record(&self, ty: &LibraryType) -> Option<LibRecord> {
        match &self.ty(ty)?.shape {
            Shape::Record(r) => Some(r.clone()),
            _ => None,
        }
    }

    fn class_info(&self, ty: &LibraryType) -> Option<LibClass> {
        let t = self.ty(ty)?;
        Some(LibClass {
            default_interface: None,
            creatable: t.creatable?,
        })
    }

    fn member(&self, ty: &LibraryType, name: &str) -> Option<LibMember> {
        self.ty(ty)?
            .members
            .get(&name.to_ascii_lowercase())
            .cloned()
    }

    fn default_member(&self, ty: &LibraryType) -> Option<LibMember> {
        self.ty(ty)?
            .members
            .values()
            .find(|m| m.is_default)
            .cloned()
    }

    fn describes_members(&self, ty: &LibraryType) -> bool {
        self.ty(ty).is_some_and(|t| t.complete)
    }

    fn interface_members(&self, ty: &LibraryType) -> Option<Vec<LibMember>> {
        let t = self.ty(ty)?;
        if !t.interface {
            return None;
        }
        let mut v: Vec<LibMember> = t.members.values().cloned().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        Some(v)
    }

    fn events(&self, ty: &LibraryType) -> Option<Vec<LibEvent>> {
        let t = self.ty(ty)?;
        Some(t.events.clone())
    }

    fn global(&self, name: &str) -> Option<LibGlobal> {
        self.globals.get(&name.to_ascii_lowercase()).cloned()
    }
}

/// Two providers consulted in order.
pub struct Chain<'a>(pub &'a dyn TypeProvider, pub &'a dyn TypeProvider);

impl TypeProvider for Chain<'_> {
    fn resolve_type(&self, q: Option<&str>, n: &str) -> Option<LibraryType> {
        self.0
            .resolve_type(q, n)
            .or_else(|| self.1.resolve_type(q, n))
    }
    fn has_library(&self, n: &str) -> bool {
        self.0.has_library(n) || self.1.has_library(n)
    }
    fn resolve_declared(&self, q: Option<&str>, n: &str) -> Option<Type> {
        self.0
            .resolve_declared(q, n)
            .or_else(|| self.1.resolve_declared(q, n))
    }
    fn enum_members(&self, t: &LibraryType) -> Option<Vec<(String, i64)>> {
        self.0.enum_members(t).or_else(|| self.1.enum_members(t))
    }
    fn record(&self, t: &LibraryType) -> Option<LibRecord> {
        self.0.record(t).or_else(|| self.1.record(t))
    }
    fn class_info(&self, t: &LibraryType) -> Option<LibClass> {
        self.0.class_info(t).or_else(|| self.1.class_info(t))
    }
    fn member(&self, t: &LibraryType, n: &str) -> Option<LibMember> {
        self.0.member(t, n).or_else(|| self.1.member(t, n))
    }
    fn default_member(&self, t: &LibraryType) -> Option<LibMember> {
        self.0
            .default_member(t)
            .or_else(|| self.1.default_member(t))
    }
    fn describes_members(&self, t: &LibraryType) -> bool {
        self.0.describes_members(t) || self.1.describes_members(t)
    }
    fn interface_members(&self, t: &LibraryType) -> Option<Vec<LibMember>> {
        self.0
            .interface_members(t)
            .or_else(|| self.1.interface_members(t))
    }
    fn events(&self, t: &LibraryType) -> Option<Vec<LibEvent>> {
        self.0.events(t).or_else(|| self.1.events(t))
    }
    fn is_interface(&self, t: &LibraryType) -> Option<bool> {
        self.0.is_interface(t).or_else(|| self.1.is_interface(t))
    }
    fn enumerable_item(&self, t: &LibraryType) -> Option<Type> {
        self.0
            .enumerable_item(t)
            .or_else(|| self.1.enumerable_item(t))
    }
    fn is_exact_class(&self, t: &LibraryType) -> bool {
        self.0.is_exact_class(t) || self.1.is_exact_class(t)
    }
    fn global(&self, n: &str) -> Option<LibGlobal> {
        self.0.global(n).or_else(|| self.1.global(n))
    }
    fn catalog(&self) -> Vec<LibEntry> {
        let mut v = self.0.catalog();
        v.extend(self.1.catalog());
        v
    }
    fn documentation(&self, t: Option<&LibraryType>, m: Option<&str>) -> Option<String> {
        self.0
            .documentation(t, m)
            .or_else(|| self.1.documentation(t, m))
    }
}

/// Any number of providers consulted in order: the first that knows the
/// answer wins. The elements may be references, boxes or `Arc`s of
/// providers.
///
/// ```
/// use vba_sema::{Layers, NoLibraries, StaticLibrary, TypeProvider};
///
/// let a = StaticLibrary::new("A");
/// let b = StaticLibrary::new("B");
/// let layers = Layers(vec![&a as &dyn TypeProvider, &b, &NoLibraries]);
/// assert!(layers.has_library("b"));
/// ```
pub struct Layers<T>(pub Vec<T>);

impl<T> TypeProvider for Layers<T>
where
    T: std::ops::Deref,
    T::Target: TypeProvider,
{
    fn resolve_type(&self, q: Option<&str>, n: &str) -> Option<LibraryType> {
        self.0.iter().find_map(|p| p.resolve_type(q, n))
    }
    fn has_library(&self, n: &str) -> bool {
        self.0.iter().any(|p| p.has_library(n))
    }
    fn resolve_declared(&self, q: Option<&str>, n: &str) -> Option<Type> {
        self.0.iter().find_map(|p| p.resolve_declared(q, n))
    }
    fn enum_members(&self, t: &LibraryType) -> Option<Vec<(String, i64)>> {
        self.0.iter().find_map(|p| p.enum_members(t))
    }
    fn record(&self, t: &LibraryType) -> Option<LibRecord> {
        self.0.iter().find_map(|p| p.record(t))
    }
    fn class_info(&self, t: &LibraryType) -> Option<LibClass> {
        self.0.iter().find_map(|p| p.class_info(t))
    }
    fn member(&self, t: &LibraryType, n: &str) -> Option<LibMember> {
        self.0.iter().find_map(|p| p.member(t, n))
    }
    fn default_member(&self, t: &LibraryType) -> Option<LibMember> {
        self.0.iter().find_map(|p| p.default_member(t))
    }
    fn describes_members(&self, t: &LibraryType) -> bool {
        self.0.iter().any(|p| p.describes_members(t))
    }
    fn interface_members(&self, t: &LibraryType) -> Option<Vec<LibMember>> {
        self.0.iter().find_map(|p| p.interface_members(t))
    }
    fn events(&self, t: &LibraryType) -> Option<Vec<LibEvent>> {
        self.0.iter().find_map(|p| p.events(t))
    }
    fn is_interface(&self, t: &LibraryType) -> Option<bool> {
        self.0.iter().find_map(|p| p.is_interface(t))
    }
    fn enumerable_item(&self, t: &LibraryType) -> Option<Type> {
        self.0.iter().find_map(|p| p.enumerable_item(t))
    }
    fn is_exact_class(&self, t: &LibraryType) -> bool {
        self.0.iter().any(|p| p.is_exact_class(t))
    }
    fn global(&self, n: &str) -> Option<LibGlobal> {
        self.0.iter().find_map(|p| p.global(n))
    }
    fn catalog(&self) -> Vec<LibEntry> {
        self.0.iter().flat_map(|p| p.catalog()).collect()
    }
    fn documentation(&self, t: Option<&LibraryType>, m: Option<&str>) -> Option<String> {
        self.0.iter().find_map(|p| p.documentation(t, m))
    }
}
