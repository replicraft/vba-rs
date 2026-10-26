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
        }
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
}

/// An event of a library type.
#[derive(Debug, Clone, PartialEq)]
pub struct LibEvent {
    /// Name.
    pub name: String,
    /// Parameters.
    pub params: Vec<LibParam>,
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

/// Knowledge about external libraries, injected by the embedder.
///
/// Every method has a "does not know" default so that a provider only
/// describes what it can describe; the analyzer turns missing knowledge
/// into explicit diagnostics, never into guesses.
pub trait TypeProvider {
    /// Resolves a type name, with an optional library qualifier
    /// (`Excel.Range` is `(Some("Excel"), "Range")`).
    fn resolve_type(&self, qualifier: Option<&str>, name: &str) -> Option<LibraryType>;

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

    /// A global name of a referenced library.
    fn global(&self, _name: &str) -> Option<LibGlobal> {
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
    fn global(&self, n: &str) -> Option<LibGlobal> {
        self.0.global(n).or_else(|| self.1.global(n))
    }
}
