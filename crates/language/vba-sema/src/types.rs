//! Identifiers and the static type system.

use std::fmt;

use vba_ast::BinaryOp;
use vba_value::VarType;

macro_rules! id {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);
        impl $name {
            /// The index into the owning table.
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

id!(
    /// A module of the project.
    ModuleId
);
id!(
    /// A procedure (`Sub`, `Function`, `Property`).
    ProcId
);
id!(
    /// A module-level variable (a static variable of a standard module, an
    /// instance field of a class module, or a hoisted `Static` local).
    GlobalId
);
id!(
    /// A named constant.
    ConstId
);
id!(
    /// A user-defined type.
    UdtId
);
id!(
    /// An enumeration.
    EnumId
);
id!(
    /// A `Declare` statement.
    DeclareId
);
id!(
    /// An `Event` declaration.
    EventId
);
id!(
    /// A local slot of a procedure: parameter, local variable, function
    /// result or compiler temporary.
    LocalId
);
id!(
    /// A label of a procedure.
    LabelId
);

/// A type provided by an external library (host object model, COM type
/// library), resolved through a [`TypeProvider`](crate::TypeProvider).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LibraryType {
    /// Library name (`Excel`, `Scripting`, `VBA`).
    pub library: String,
    /// Type name (`Range`, `Dictionary`).
    pub name: String,
}

impl fmt::Display for LibraryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.library.is_empty() {
            f.write_str(&self.name)
        } else {
            write!(f, "{}.{}", self.library, self.name)
        }
    }
}

/// A static type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// `Boolean`
    Boolean,
    /// `Byte`
    Byte,
    /// `Integer`
    Integer,
    /// `Long`
    Long,
    /// `LongLong` (`LongPtr` resolves to `Long` or `LongLong` by profile).
    LongLong,
    /// `Single`
    Single,
    /// `Double`
    Double,
    /// `Currency`
    Currency,
    /// `Date`
    Date,
    /// `Decimal` (only reachable as a Variant subtype in VBA; used for
    /// constant folding results)
    Decimal,
    /// Variable-length `String`
    String,
    /// `String * n`
    FixedString(u32),
    /// `Variant`
    Variant,
    /// The generic `Object`
    Object,
    /// A class module of the project.
    Class(ModuleId),
    /// A `Type ... End Type`.
    Udt(UdtId),
    /// An `Enum` (stored as `Long`).
    Enum(EnumId),
    /// A library type (interface, dispinterface or coclass).
    Library(LibraryType),
    /// An enumeration of a type library (stored as `Long`).
    LibEnum(LibraryType),
    /// A record (UDT) of a type library; a value type like [`Type::Udt`].
    LibRecord(LibraryType),
    /// An array; `rank` is `None` when unknown (`Dim a()`, parameters).
    Array {
        /// Element type.
        elem: Box<Type>,
        /// Number of dimensions when known.
        rank: Option<u8>,
    },
    /// The type of the literal `Nothing`.
    Nothing,
    /// `As Any` of `Declare` parameters.
    Any,
    /// A type that could not be determined; a diagnostic was reported.
    Error,
}

impl Type {
    /// Whether this is the poisoned error type.
    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    /// Whether values of this type are object references.
    pub fn is_object(&self) -> bool {
        matches!(
            self,
            Type::Object | Type::Class(_) | Type::Library(_) | Type::Nothing
        )
    }

    /// Whether the type is numeric (including `Boolean`, `Date`, enums).
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::Byte
                | Type::Integer
                | Type::Long
                | Type::LongLong
                | Type::Single
                | Type::Double
                | Type::Currency
                | Type::Decimal
                | Type::Enum(_)
                | Type::LibEnum(_)
        )
    }

    /// Whether the type is an integer type (or `Boolean`/enum).
    pub fn is_integral(&self) -> bool {
        matches!(
            self,
            Type::Byte
                | Type::Integer
                | Type::Long
                | Type::LongLong
                | Type::Enum(_)
                | Type::LibEnum(_)
        )
    }

    /// Whether this is a project or library enumeration.
    pub fn is_enum(&self) -> bool {
        matches!(self, Type::Enum(_) | Type::LibEnum(_))
    }

    /// Whether this is a user-defined type of the project or of a library.
    pub fn is_record(&self) -> bool {
        matches!(self, Type::Udt(_) | Type::LibRecord(_))
    }

    /// Whether the type is `String` or `String * n`.
    pub fn is_string(&self) -> bool {
        matches!(self, Type::String | Type::FixedString(_))
    }

    /// Whether the type is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, Type::Array { .. })
    }

    /// Whether this is `Variant`.
    pub fn is_variant(&self) -> bool {
        matches!(self, Type::Variant)
    }

    /// The `vba-value` type that stores values of this type, if scalar.
    pub fn var_type(&self) -> Option<VarType> {
        Some(match self {
            Type::Boolean => VarType::Boolean,
            Type::Byte => VarType::Byte,
            Type::Integer => VarType::Integer,
            Type::Long | Type::Enum(_) | Type::LibEnum(_) => VarType::Long,
            Type::LongLong => VarType::LongLong,
            Type::Single => VarType::Single,
            Type::Double => VarType::Double,
            Type::Currency => VarType::Currency,
            Type::Date => VarType::Date,
            Type::Decimal => VarType::Decimal,
            Type::String | Type::FixedString(_) => VarType::String,
            Type::Variant => VarType::Variant,
            Type::Object | Type::Class(_) | Type::Library(_) | Type::Nothing => VarType::Object,
            Type::Udt(_) | Type::LibRecord(_) => VarType::UserDefinedType,
            Type::Array { .. } | Type::Any | Type::Error => return None,
        })
    }

    /// An array of `elem` with unknown rank.
    pub fn array_of(elem: Type, rank: Option<u8>) -> Type {
        Type::Array {
            elem: Box::new(elem),
            rank,
        }
    }

    /// Integer width class (1 = Byte, 2 = Integer/Boolean, 3 = Long,
    /// 4 = LongLong), for logical operator typing.
    fn int_rank(&self) -> Option<u8> {
        Some(match self {
            Type::Byte => 1,
            Type::Boolean | Type::Integer => 2,
            Type::Long | Type::Enum(_) | Type::LibEnum(_) => 3,
            Type::LongLong => 4,
            _ => return None,
        })
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Boolean => f.write_str("Boolean"),
            Type::Byte => f.write_str("Byte"),
            Type::Integer => f.write_str("Integer"),
            Type::Long => f.write_str("Long"),
            Type::LongLong => f.write_str("LongLong"),
            Type::Single => f.write_str("Single"),
            Type::Double => f.write_str("Double"),
            Type::Currency => f.write_str("Currency"),
            Type::Date => f.write_str("Date"),
            Type::Decimal => f.write_str("Decimal"),
            Type::String => f.write_str("String"),
            Type::FixedString(n) => write!(f, "String*{n}"),
            Type::Variant => f.write_str("Variant"),
            Type::Object => f.write_str("Object"),
            Type::Class(m) => write!(f, "class#{}", m.0),
            Type::Udt(u) => write!(f, "udt#{}", u.0),
            Type::Enum(e) => write!(f, "enum#{}", e.0),
            Type::Library(l) | Type::LibEnum(l) | Type::LibRecord(l) => write!(f, "{l}"),
            Type::Array { elem, rank } => match rank {
                Some(r) => write!(f, "{elem}[{r}]"),
                None => write!(f, "{elem}[]"),
            },
            Type::Nothing => f.write_str("Nothing"),
            Type::Any => f.write_str("Any"),
            Type::Error => f.write_str("<error>"),
        }
    }
}

/// How a value of one type is stored into a location of another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coercion {
    /// Same type, nothing to do.
    Identity,
    /// An implicit conversion (checked at run time).
    Convert,
    /// An object reference narrowing or widening that needs a run-time
    /// interface check (`Object` to a class, unrelated library types).
    RuntimeCast,
    /// Not allowed.
    Invalid,
}

/// The result type of a binary operator over statically known operand types
/// (MS-VBAL §5.6, "operator type rules"). `Variant` operands give
/// `Variant`; the result of an operation on an error type is `Error`.
pub fn binary_result(op: BinaryOp, l: &Type, r: &Type) -> Type {
    use BinaryOp::*;
    if l.is_error() || r.is_error() {
        return Type::Error;
    }
    let either_variant = l.is_variant() || r.is_variant();
    match op {
        Concat => {
            if either_variant {
                Type::Variant
            } else {
                Type::String
            }
        }
        Eq | Ne | Lt | Gt | Le | Ge | Like => {
            if either_variant {
                Type::Variant
            } else {
                Type::Boolean
            }
        }
        Is => Type::Boolean,
        And | Or | Xor | Eqv | Imp => {
            if either_variant {
                return Type::Variant;
            }
            logic_result(l, r)
        }
        Add | Sub | Mul | Div | IntDiv | Mod | Pow => {
            if either_variant {
                return Type::Variant;
            }
            arith_result(op, l, r)
        }
    }
}

fn logic_result(l: &Type, r: &Type) -> Type {
    if *l == Type::Boolean && *r == Type::Boolean {
        return Type::Boolean;
    }
    let rank = |t: &Type| t.int_rank().unwrap_or(3);
    match rank(l).max(rank(r)) {
        1 => Type::Byte,
        2 => Type::Integer,
        3 => Type::Long,
        _ => Type::LongLong,
    }
}

fn arith_result(op: BinaryOp, l: &Type, r: &Type) -> Type {
    use BinaryOp::*;
    let norm = |t: &Type| match t {
        Type::Boolean => Type::Integer,
        Type::Enum(_) | Type::LibEnum(_) => Type::Long,
        Type::String | Type::FixedString(_) => Type::Double,
        t => t.clone(),
    };
    let (a, b) = (norm(l), norm(r));
    match op {
        Pow => Type::Double,
        Div => {
            if a == Type::Single && b == Type::Single {
                Type::Single
            } else if a == Type::Decimal || b == Type::Decimal {
                Type::Decimal
            } else {
                Type::Double
            }
        }
        IntDiv | Mod => {
            let rank = |t: &Type| t.int_rank().unwrap_or(3);
            if a == Type::LongLong || b == Type::LongLong {
                Type::LongLong
            } else if rank(&a).max(rank(&b)) <= 2 {
                Type::Integer
            } else {
                Type::Long
            }
        }
        Add | Sub | Mul => {
            // Date arithmetic.
            if op == Sub && a == Type::Date && b == Type::Date {
                return Type::Double;
            }
            if a == Type::Date || b == Type::Date {
                return if l.is_string() || r.is_string() {
                    Type::Double
                } else {
                    Type::Date
                };
            }
            if op == Add && l.is_string() && r.is_string() {
                return Type::String;
            }
            let order = |t: &Type| match t {
                Type::Byte => 1,
                Type::Integer => 2,
                Type::Long => 3,
                Type::LongLong => 4,
                Type::Single => 5,
                Type::Currency => 6,
                Type::Decimal => 7,
                _ => 8,
            };
            let (oa, ob) = (order(&a), order(&b));
            // Currency combined with Double is Double; with Single it stays
            // Currency; Decimal wins over everything but Double.
            let pick = |x: &Type, y: &Type| -> Type {
                if *x == Type::Double || *y == Type::Double {
                    Type::Double
                } else if oa >= ob {
                    x.clone()
                } else {
                    y.clone()
                }
            };
            pick(&a, &b)
        }
        _ => Type::Variant,
    }
}

/// The result type of unary minus / plus on `t`.
pub fn negate_result(t: &Type) -> Type {
    match t {
        Type::Variant => Type::Variant,
        Type::Boolean | Type::Byte => Type::Integer,
        Type::Enum(_) | Type::LibEnum(_) => Type::Long,
        Type::String | Type::FixedString(_) => Type::Double,
        Type::Error => Type::Error,
        t => t.clone(),
    }
}

/// The result type of `Not` on `t`.
pub fn not_result(t: &Type) -> Type {
    match t {
        Type::Variant => Type::Variant,
        Type::Boolean => Type::Boolean,
        Type::Error => Type::Error,
        t if t.int_rank().is_some() => t.clone(),
        _ => Type::Long,
    }
}
