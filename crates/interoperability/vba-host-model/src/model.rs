//! The description itself: plain data with builders. Nothing here is
//! checked; [`HostLibrary::check`](crate::HostLibrary::check) validates a
//! description and produces the form every adapter takes.

use std::fmt;

use vba_value::Variant;

/// A type named in a description.
///
/// `Named` refers to a class or an enumeration of the same library; whether
/// it exists is decided by validation, not by construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeRef {
    /// `Boolean`
    Boolean,
    /// `Byte`
    Byte,
    /// `Integer`
    Integer,
    /// `Long`
    Long,
    /// `LongLong`
    LongLong,
    /// `Single`
    Single,
    /// `Double`
    Double,
    /// `Currency`
    Currency,
    /// `Date`
    Date,
    /// `String`
    String,
    /// `Variant`
    Variant,
    /// The generic `Object`
    Object,
    /// A class or enumeration of the library, by name.
    Named(String),
    /// An array of any rank (`Long()` is `Array(Long)`).
    Array(Box<TypeRef>),
}

/// Deepest array nesting [`TypeRef::parse`] accepts.
const MAX_ARRAY_NESTING: usize = 32;

impl TypeRef {
    /// A class or enumeration of the library.
    pub fn named(name: impl Into<String>) -> TypeRef {
        TypeRef::Named(name.into())
    }

    /// An array of `elem`.
    pub fn array_of(elem: TypeRef) -> TypeRef {
        TypeRef::Array(Box::new(elem))
    }

    /// Reads the VBA spelling: an intrinsic type name (any case), a name of
    /// the library, or either followed by `()` for an array. The result of
    /// an unknown name is `Named`; validation reports it when it dangles.
    pub fn parse(text: &str) -> TypeRef {
        let mut t = text.trim();
        // Count the `()` suffixes iteratively: hostile text may hold
        // millions of them, and a recursive parse (or a deeply nested
        // result) would overflow the stack.
        let mut rank = 0usize;
        while let Some(inner) = t.strip_suffix("()") {
            rank += 1;
            t = inner.trim_end();
        }
        if rank > MAX_ARRAY_NESTING {
            // Not a type any library declares; validation reports it as an
            // unresolved name.
            return TypeRef::Named(text.trim().to_owned());
        }
        let mut ty = match t.to_ascii_lowercase().as_str() {
            "boolean" => TypeRef::Boolean,
            "byte" => TypeRef::Byte,
            "integer" => TypeRef::Integer,
            "long" => TypeRef::Long,
            "longlong" => TypeRef::LongLong,
            "single" => TypeRef::Single,
            "double" => TypeRef::Double,
            "currency" => TypeRef::Currency,
            "date" => TypeRef::Date,
            "string" => TypeRef::String,
            "variant" => TypeRef::Variant,
            "object" => TypeRef::Object,
            _ => TypeRef::Named(t.to_owned()),
        };
        for _ in 0..rank {
            ty = TypeRef::array_of(ty);
        }
        ty
    }

    /// The intrinsic type names, which a class or enumeration may not use.
    pub(crate) fn is_intrinsic_name(name: &str) -> bool {
        !matches!(TypeRef::parse(name), TypeRef::Named(_))
    }
}

impl From<&str> for TypeRef {
    fn from(s: &str) -> TypeRef {
        TypeRef::parse(s)
    }
}

impl From<String> for TypeRef {
    fn from(s: String) -> TypeRef {
        TypeRef::parse(&s)
    }
}

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeRef::Boolean => f.write_str("Boolean"),
            TypeRef::Byte => f.write_str("Byte"),
            TypeRef::Integer => f.write_str("Integer"),
            TypeRef::Long => f.write_str("Long"),
            TypeRef::LongLong => f.write_str("LongLong"),
            TypeRef::Single => f.write_str("Single"),
            TypeRef::Double => f.write_str("Double"),
            TypeRef::Currency => f.write_str("Currency"),
            TypeRef::Date => f.write_str("Date"),
            TypeRef::String => f.write_str("String"),
            TypeRef::Variant => f.write_str("Variant"),
            TypeRef::Object => f.write_str("Object"),
            TypeRef::Named(n) => f.write_str(n),
            TypeRef::Array(e) => write!(f, "{e}()"),
        }
    }
}

/// A constant value: the default of an optional parameter or the value of
/// a [`HostConst`].
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    /// `Empty`
    Empty,
    /// `Null`
    Null,
    /// A `Boolean`.
    Boolean(bool),
    /// An `Integer`.
    Integer(i16),
    /// A `Long`.
    Long(i32),
    /// A `Double`.
    Double(f64),
    /// A `String`.
    String(String),
}

impl ConstValue {
    /// The run-time value.
    pub fn to_variant(&self) -> Variant {
        match self {
            ConstValue::Empty => Variant::Empty,
            ConstValue::Null => Variant::Null,
            ConstValue::Boolean(b) => Variant::Boolean(*b),
            ConstValue::Integer(i) => Variant::Integer(*i),
            ConstValue::Long(l) => Variant::Long(*l),
            ConstValue::Double(d) => Variant::Double(*d),
            ConstValue::String(s) => Variant::from(s.as_str()),
        }
    }

    /// The static type of a constant of this value.
    pub fn type_ref(&self) -> TypeRef {
        match self {
            ConstValue::Empty | ConstValue::Null => TypeRef::Variant,
            ConstValue::Boolean(_) => TypeRef::Boolean,
            ConstValue::Integer(_) => TypeRef::Integer,
            ConstValue::Long(_) => TypeRef::Long,
            ConstValue::Double(_) => TypeRef::Double,
            ConstValue::String(_) => TypeRef::String,
        }
    }
}

impl From<bool> for ConstValue {
    fn from(v: bool) -> Self {
        ConstValue::Boolean(v)
    }
}
impl From<i16> for ConstValue {
    fn from(v: i16) -> Self {
        ConstValue::Integer(v)
    }
}
impl From<i32> for ConstValue {
    fn from(v: i32) -> Self {
        ConstValue::Long(v)
    }
}
impl From<f64> for ConstValue {
    fn from(v: f64) -> Self {
        ConstValue::Double(v)
    }
}
impl From<&str> for ConstValue {
    fn from(v: &str) -> Self {
        ConstValue::String(v.to_owned())
    }
}

/// A parameter of a member, function or event.
#[derive(Debug, Clone, PartialEq)]
pub struct HostParam {
    /// Name (used for named arguments).
    pub name: String,
    /// Declared type.
    pub ty: TypeRef,
    /// `Optional`.
    pub optional: bool,
    /// The value an omitted optional argument takes. Without one the
    /// argument is passed as missing (`IsMissing` is true).
    pub default: Option<ConstValue>,
    /// Declared `ByRef`: the member may assign to the caller's variable.
    /// The default is `ByVal`, unlike a VBA declaration.
    pub by_ref: bool,
    /// `ParamArray` (last parameter only).
    pub param_array: bool,
    /// Documentation text.
    pub doc: String,
}

impl HostParam {
    /// A required `ByVal` parameter.
    pub fn new(name: impl Into<String>, ty: impl Into<TypeRef>) -> HostParam {
        HostParam {
            name: name.into(),
            ty: ty.into(),
            optional: false,
            default: None,
            by_ref: false,
            param_array: false,
            doc: String::new(),
        }
    }

    /// Makes the parameter `Optional`.
    #[must_use]
    pub fn optional(mut self) -> HostParam {
        self.optional = true;
        self
    }

    /// Makes the parameter `Optional` with a default value.
    #[must_use]
    pub fn default_value(mut self, v: impl Into<ConstValue>) -> HostParam {
        self.optional = true;
        self.default = Some(v.into());
        self
    }

    /// Declares the parameter `ByRef`.
    #[must_use]
    pub fn by_ref(mut self) -> HostParam {
        self.by_ref = true;
        self
    }

    /// Makes the parameter a `ParamArray` (`Variant()`, optional, `ByRef`
    /// as VBA requires).
    #[must_use]
    pub fn param_array(mut self) -> HostParam {
        self.param_array = true;
        self.optional = true;
        self.by_ref = true;
        self.ty = TypeRef::array_of(TypeRef::Variant);
        self
    }

    /// Sets the documentation text.
    #[must_use]
    pub fn doc(mut self, doc: impl Into<String>) -> HostParam {
        self.doc = doc.into();
        self
    }
}

/// What a member is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberKind {
    /// A `Sub` (no result type) or a `Function`.
    Method,
    /// A property: `Property Get`, `Let` and `Set` under one name.
    Property,
}

/// Which accessors a property has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Access {
    /// `Property Get` only.
    ReadOnly,
    /// `Property Get` and the assignment of the property type: `Let` for a
    /// value type, `Set` for a class, both for a `Variant`.
    ReadWrite,
    /// The assignment only.
    WriteOnly,
}

/// A method or property of a class, or a global function.
#[derive(Debug, Clone, PartialEq)]
pub struct HostMember {
    /// Name.
    pub name: String,
    /// Method or property.
    pub kind: MemberKind,
    /// Result type of a function, value type of a property; `None` for a
    /// `Sub`.
    pub ty: Option<TypeRef>,
    /// Parameters (index parameters of a property).
    pub params: Vec<HostParam>,
    /// Accessors of a property; ignored for a method.
    pub access: Access,
    /// The default member of the class (`DISPID_VALUE`): `rs("a")`,
    /// `rs!a`.
    pub default_member: bool,
    /// Left out of completion lists (still callable).
    pub hidden: bool,
    /// An explicit `DISPID`; unset members get one automatically.
    pub dispid: Option<i32>,
    /// Documentation text.
    pub doc: String,
    /// Free-form attributes (`VB_Description`, a help id...), carried to
    /// the JSON form and to tools; they do not change behavior.
    pub attributes: Vec<(String, String)>,
}

impl HostMember {
    fn base(name: impl Into<String>, kind: MemberKind, ty: Option<TypeRef>) -> HostMember {
        HostMember {
            name: name.into(),
            kind,
            ty,
            params: Vec::new(),
            access: Access::ReadWrite,
            default_member: false,
            hidden: false,
            dispid: None,
            doc: String::new(),
            attributes: Vec::new(),
        }
    }

    /// A `Sub`.
    pub fn sub(name: impl Into<String>) -> HostMember {
        HostMember::base(name, MemberKind::Method, None)
    }

    /// A `Function` returning `ty`.
    pub fn function(name: impl Into<String>, ty: impl Into<TypeRef>) -> HostMember {
        HostMember::base(name, MemberKind::Method, Some(ty.into()))
    }

    /// A read/write property of type `ty`.
    pub fn property(name: impl Into<String>, ty: impl Into<TypeRef>) -> HostMember {
        HostMember::base(name, MemberKind::Property, Some(ty.into()))
    }

    /// Adds a parameter.
    #[must_use]
    pub fn param(mut self, p: HostParam) -> HostMember {
        self.params.push(p);
        self
    }

    /// Makes a property read-only.
    #[must_use]
    pub fn read_only(mut self) -> HostMember {
        self.access = Access::ReadOnly;
        self
    }

    /// Makes a property write-only.
    #[must_use]
    pub fn write_only(mut self) -> HostMember {
        self.access = Access::WriteOnly;
        self
    }

    /// Marks the default member.
    #[must_use]
    pub fn default_member(mut self) -> HostMember {
        self.default_member = true;
        self
    }

    /// Hides the member from completion lists.
    #[must_use]
    pub fn hidden(mut self) -> HostMember {
        self.hidden = true;
        self
    }

    /// Fixes the `DISPID`.
    #[must_use]
    pub fn dispid(mut self, id: i32) -> HostMember {
        self.dispid = Some(id);
        self
    }

    /// Sets the documentation text.
    #[must_use]
    pub fn doc(mut self, doc: impl Into<String>) -> HostMember {
        self.doc = doc.into();
        self
    }

    /// Adds an attribute.
    #[must_use]
    pub fn attribute(mut self, name: impl Into<String>, value: impl Into<String>) -> HostMember {
        self.attributes.push((name.into(), value.into()));
        self
    }

    /// `true` for a `Sub`.
    pub fn is_sub(&self) -> bool {
        self.kind == MemberKind::Method && self.ty.is_none()
    }
}

/// An event a class raises.
#[derive(Debug, Clone, PartialEq)]
pub struct HostEvent {
    /// Name.
    pub name: String,
    /// Parameters (`Cancel As Integer` is `ByRef`).
    pub params: Vec<HostParam>,
    /// Documentation text.
    pub doc: String,
}

impl HostEvent {
    /// An event without parameters.
    pub fn new(name: impl Into<String>) -> HostEvent {
        HostEvent {
            name: name.into(),
            params: Vec::new(),
            doc: String::new(),
        }
    }

    /// Adds a parameter.
    #[must_use]
    pub fn param(mut self, p: HostParam) -> HostEvent {
        self.params.push(p);
        self
    }

    /// Sets the documentation text.
    #[must_use]
    pub fn doc(mut self, doc: impl Into<String>) -> HostEvent {
        self.doc = doc.into();
        self
    }
}

/// A class of the object model.
#[derive(Debug, Clone, PartialEq)]
pub struct HostClass {
    /// Name.
    pub name: String,
    /// Documentation text.
    pub doc: String,
    /// Methods and properties in declaration order.
    pub members: Vec<HostMember>,
    /// Events.
    pub events: Vec<HostEvent>,
    /// Whether `New` and `CreateObject("Library.Class")` may create
    /// instances.
    pub creatable: bool,
    /// Whether the class is an interface: only such a class can be named in
    /// `Implements`. A class is not one by default.
    pub interface: bool,
    /// The element type of `For Each` over an instance (a `_NewEnum`
    /// member exists); `None` when the class is not enumerable.
    pub enumerable: Option<TypeRef>,
    /// Free-form attributes.
    pub attributes: Vec<(String, String)>,
}

impl HostClass {
    /// A class without members.
    pub fn new(name: impl Into<String>) -> HostClass {
        HostClass {
            name: name.into(),
            doc: String::new(),
            members: Vec::new(),
            events: Vec::new(),
            creatable: false,
            interface: false,
            enumerable: None,
            attributes: Vec::new(),
        }
    }

    /// Sets the documentation text.
    #[must_use]
    pub fn doc(mut self, doc: impl Into<String>) -> HostClass {
        self.doc = doc.into();
        self
    }

    /// Adds a method or property.
    #[must_use]
    pub fn member(mut self, m: HostMember) -> HostClass {
        self.members.push(m);
        self
    }

    /// Adds an event.
    #[must_use]
    pub fn event(mut self, e: HostEvent) -> HostClass {
        self.events.push(e);
        self
    }

    /// Lets `New` and `CreateObject` create instances.
    #[must_use]
    pub fn creatable(mut self) -> HostClass {
        self.creatable = true;
        self
    }

    /// Marks the class as an interface that `Implements` may name.
    #[must_use]
    pub fn interface(mut self) -> HostClass {
        self.interface = true;
        self
    }

    /// Makes `For Each` work on instances; `item` is the element type.
    #[must_use]
    pub fn enumerable(mut self, item: impl Into<TypeRef>) -> HostClass {
        self.enumerable = Some(item.into());
        self
    }

    /// Adds an attribute.
    #[must_use]
    pub fn attribute(mut self, name: impl Into<String>, value: impl Into<String>) -> HostClass {
        self.attributes.push((name.into(), value.into()));
        self
    }
}

/// An enumeration: named `Long` constants usable as a type.
#[derive(Debug, Clone, PartialEq)]
pub struct HostEnum {
    /// Name.
    pub name: String,
    /// Documentation text.
    pub doc: String,
    /// Members: name, value, documentation.
    pub members: Vec<(String, i32, String)>,
}

impl HostEnum {
    /// An enumeration without members.
    pub fn new(name: impl Into<String>) -> HostEnum {
        HostEnum {
            name: name.into(),
            doc: String::new(),
            members: Vec::new(),
        }
    }

    /// Sets the documentation text.
    #[must_use]
    pub fn doc(mut self, doc: impl Into<String>) -> HostEnum {
        self.doc = doc.into();
        self
    }

    /// Adds a member.
    #[must_use]
    pub fn member(mut self, name: impl Into<String>, value: i32) -> HostEnum {
        self.members.push((name.into(), value, String::new()));
        self
    }

    /// Adds a documented member.
    #[must_use]
    pub fn documented(
        mut self,
        name: impl Into<String>,
        value: i32,
        doc: impl Into<String>,
    ) -> HostEnum {
        self.members.push((name.into(), value, doc.into()));
        self
    }
}

/// A global constant.
#[derive(Debug, Clone, PartialEq)]
pub struct HostConst {
    /// Name.
    pub name: String,
    /// Value; the constant's type follows from it.
    pub value: ConstValue,
    /// Documentation text.
    pub doc: String,
}

impl HostConst {
    /// A constant.
    pub fn new(name: impl Into<String>, value: impl Into<ConstValue>) -> HostConst {
        HostConst {
            name: name.into(),
            value: value.into(),
            doc: String::new(),
        }
    }

    /// Sets the documentation text.
    #[must_use]
    pub fn doc(mut self, doc: impl Into<String>) -> HostConst {
        self.doc = doc.into();
        self
    }
}

/// A predeclared global object (`Application`, `DoCmd`).
#[derive(Debug, Clone, PartialEq)]
pub struct HostGlobal {
    /// Name.
    pub name: String,
    /// Its class.
    pub ty: TypeRef,
    /// Documentation text.
    pub doc: String,
}

impl HostGlobal {
    /// A global of class `ty`.
    pub fn new(name: impl Into<String>, ty: impl Into<TypeRef>) -> HostGlobal {
        HostGlobal {
            name: name.into(),
            ty: ty.into(),
            doc: String::new(),
        }
    }

    /// Sets the documentation text.
    #[must_use]
    pub fn doc(mut self, doc: impl Into<String>) -> HostGlobal {
        self.doc = doc.into();
        self
    }
}

/// A named set of classes, enumerations, constants, global functions and
/// predeclared objects: everything one embedding application adds to VBA.
#[derive(Debug, Clone, PartialEq)]
pub struct HostLibrary {
    /// Library name; code may qualify names with it (`DbLib.Recordset`).
    pub name: String,
    /// Documentation text.
    pub doc: String,
    /// Classes.
    pub classes: Vec<HostClass>,
    /// Enumerations.
    pub enums: Vec<HostEnum>,
    /// Constants.
    pub consts: Vec<HostConst>,
    /// Global functions and subs (`Nz`, `DLookup`); methods only, and
    /// every parameter is `ByVal`.
    pub functions: Vec<HostMember>,
    /// Predeclared global objects.
    pub globals: Vec<HostGlobal>,
}

impl HostLibrary {
    /// An empty library.
    pub fn new(name: impl Into<String>) -> HostLibrary {
        HostLibrary {
            name: name.into(),
            doc: String::new(),
            classes: Vec::new(),
            enums: Vec::new(),
            consts: Vec::new(),
            functions: Vec::new(),
            globals: Vec::new(),
        }
    }

    /// Sets the documentation text.
    #[must_use]
    pub fn doc(mut self, doc: impl Into<String>) -> HostLibrary {
        self.doc = doc.into();
        self
    }

    /// Adds a class.
    #[must_use]
    pub fn class(mut self, c: HostClass) -> HostLibrary {
        self.classes.push(c);
        self
    }

    /// Adds an enumeration.
    #[must_use]
    pub fn enumeration(mut self, e: HostEnum) -> HostLibrary {
        self.enums.push(e);
        self
    }

    /// Adds a constant.
    #[must_use]
    pub fn constant(mut self, c: HostConst) -> HostLibrary {
        self.consts.push(c);
        self
    }

    /// Adds a global function or sub.
    #[must_use]
    pub fn function(mut self, f: HostMember) -> HostLibrary {
        self.functions.push(f);
        self
    }

    /// Adds a predeclared global object.
    #[must_use]
    pub fn global(mut self, g: HostGlobal) -> HostLibrary {
        self.globals.push(g);
        self
    }
}
