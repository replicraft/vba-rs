//! The in-memory model of a type library.
//!
//! The model keeps what a compiler or an early-binding dispatch table needs
//! (names, member ids, invoke kinds, parameter types, flags, defaults,
//! constants, GUIDs, documentation) and drops the on-disk bookkeeping
//! (hash tables, segment offsets, reserved words).

use std::fmt;

use bitflags::bitflags;

/// A GUID in its in-memory (mixed-endian) byte layout.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Guid(pub [u8; 16]);

impl Guid {
    /// Builds a GUID from its textual fields.
    pub const fn from_fields(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> Guid {
        let a = d1.to_le_bytes();
        let b = d2.to_le_bytes();
        let c = d3.to_le_bytes();
        Guid([
            a[0], a[1], a[2], a[3], b[0], b[1], c[0], c[1], d4[0], d4[1], d4[2], d4[3], d4[4],
            d4[5], d4[6], d4[7],
        ])
    }

    /// `IUnknown` (`00000000-0000-0000-C000-000000000046`).
    pub const IUNKNOWN: Guid = Guid::from_fields(0, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
    /// `IDispatch` (`00020400-0000-0000-C000-000000000046`).
    pub const IDISPATCH: Guid =
        Guid::from_fields(0x0002_0400, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = &self.0;
        write!(
            f,
            "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            b[3],
            b[2],
            b[1],
            b[0],
            b[5],
            b[4],
            b[7],
            b[6],
            b[8],
            b[9],
            b[10],
            b[11],
            b[12],
            b[13],
            b[14],
            b[15]
        )
    }
}

impl fmt::Debug for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{self}}}")
    }
}

/// A `major.minor` version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Version {
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
}

bitflags! {
    /// Library flags (`LIBFLAG_*`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct LibFlags: u32 {
        /// Not for use by macro-language users.
        const RESTRICTED = 0x1;
        /// A control library.
        const CONTROL = 0x2;
        /// Not shown in browsers.
        const HIDDEN = 0x4;
        /// Has a disk image.
        const HAS_DISK_IMAGE = 0x8;
    }

    /// Type flags (`TYPEFLAG_*`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TypeFlags: u32 {
        /// A coclass that is the application object.
        const APPOBJECT = 0x1;
        /// Instantiable.
        const CAN_CREATE = 0x2;
        /// Needs a license.
        const LICENSED = 0x4;
        /// Predeclared identifier.
        const PREDECLARED_ID = 0x8;
        /// Hidden.
        const HIDDEN = 0x10;
        /// A control.
        const CONTROL = 0x20;
        /// Dual interface.
        const DUAL = 0x40;
        /// Non-extensible.
        const NON_EXTENSIBLE = 0x80;
        /// OLE automation compatible.
        const OLE_AUTOMATION = 0x100;
        /// Restricted.
        const RESTRICTED = 0x200;
        /// Aggregatable.
        const AGGREGATABLE = 0x400;
        /// Replaceable.
        const REPLACEABLE = 0x800;
        /// Dispatchable.
        const DISPATCHABLE = 0x1000;
        /// Reverse binding.
        const REVERSE_BIND = 0x2000;
        /// Proxy.
        const PROXY = 0x4000;
    }

    /// Function flags (`FUNCFLAG_*`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FuncFlags: u32 {
        /// Not callable from macro languages.
        const RESTRICTED = 0x1;
        /// Raises an event.
        const SOURCE = 0x2;
        /// Bindable property.
        const BINDABLE = 0x4;
        /// Requests edit.
        const REQUEST_EDIT = 0x8;
        /// Display bindable.
        const DISPLAY_BIND = 0x10;
        /// Default bindable.
        const DEFAULT_BIND = 0x20;
        /// Hidden.
        const HIDDEN = 0x40;
        /// Sets the last error.
        const USES_GET_LAST_ERROR = 0x80;
        /// Default collection element.
        const DEFAULT_COLL_ELEM = 0x100;
        /// UI default.
        const UI_DEFAULT = 0x200;
        /// Not browsable.
        const NON_BROWSABLE = 0x400;
        /// Replaceable.
        const REPLACEABLE = 0x800;
        /// Immediate bind.
        const IMMEDIATE_BIND = 0x1000;
    }

    /// Variable flags (`VARFLAG_*`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct VarFlags: u32 {
        /// Read-only.
        const READ_ONLY = 0x1;
        /// Raises an event.
        const SOURCE = 0x2;
        /// Bindable.
        const BINDABLE = 0x4;
        /// Requests edit.
        const REQUEST_EDIT = 0x8;
        /// Display bindable.
        const DISPLAY_BIND = 0x10;
        /// Default bindable.
        const DEFAULT_BIND = 0x20;
        /// Hidden.
        const HIDDEN = 0x40;
        /// Restricted.
        const RESTRICTED = 0x80;
        /// Default collection element.
        const DEFAULT_COLL_ELEM = 0x100;
        /// UI default.
        const UI_DEFAULT = 0x200;
        /// Not browsable.
        const NON_BROWSABLE = 0x400;
        /// Replaceable.
        const REPLACEABLE = 0x800;
        /// Immediate bind.
        const IMMEDIATE_BIND = 0x1000;
    }

    /// Parameter flags (`PARAMFLAG_*`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ParamFlags: u32 {
        /// Input.
        const IN = 0x1;
        /// Output.
        const OUT = 0x2;
        /// Locale id.
        const LCID = 0x4;
        /// The return value.
        const RETVAL = 0x8;
        /// Optional.
        const OPTIONAL = 0x10;
        /// Has a default value.
        const HAS_DEFAULT = 0x20;
        /// Has custom data.
        const HAS_CUSTDATA = 0x40;
    }

    /// Flags of an implemented type (`IMPLTYPEFLAG_*`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ImplFlags: u32 {
        /// The default interface or source.
        const DEFAULT = 0x1;
        /// An event source.
        const SOURCE = 0x2;
        /// Restricted.
        const RESTRICTED = 0x4;
        /// Default vtable.
        const DEFAULT_VTABLE = 0x8;
    }
}

/// `VARENUM` values that appear in type descriptions.
pub mod vt {
    /// `VT_EMPTY`.
    pub const EMPTY: u16 = 0;
    /// `VT_NULL`.
    pub const NULL: u16 = 1;
    /// `VT_I2`.
    pub const I2: u16 = 2;
    /// `VT_I4`.
    pub const I4: u16 = 3;
    /// `VT_R4`.
    pub const R4: u16 = 4;
    /// `VT_R8`.
    pub const R8: u16 = 5;
    /// `VT_CY`.
    pub const CY: u16 = 6;
    /// `VT_DATE`.
    pub const DATE: u16 = 7;
    /// `VT_BSTR`.
    pub const BSTR: u16 = 8;
    /// `VT_DISPATCH`.
    pub const DISPATCH: u16 = 9;
    /// `VT_ERROR`.
    pub const ERROR: u16 = 10;
    /// `VT_BOOL`.
    pub const BOOL: u16 = 11;
    /// `VT_VARIANT`.
    pub const VARIANT: u16 = 12;
    /// `VT_UNKNOWN`.
    pub const UNKNOWN: u16 = 13;
    /// `VT_DECIMAL`.
    pub const DECIMAL: u16 = 14;
    /// `VT_I1`.
    pub const I1: u16 = 16;
    /// `VT_UI1`.
    pub const UI1: u16 = 17;
    /// `VT_UI2`.
    pub const UI2: u16 = 18;
    /// `VT_UI4`.
    pub const UI4: u16 = 19;
    /// `VT_I8`.
    pub const I8: u16 = 20;
    /// `VT_UI8`.
    pub const UI8: u16 = 21;
    /// `VT_INT`.
    pub const INT: u16 = 22;
    /// `VT_UINT`.
    pub const UINT: u16 = 23;
    /// `VT_VOID`.
    pub const VOID: u16 = 24;
    /// `VT_HRESULT`.
    pub const HRESULT: u16 = 25;
    /// `VT_PTR`.
    pub const PTR: u16 = 26;
    /// `VT_SAFEARRAY`.
    pub const SAFEARRAY: u16 = 27;
    /// `VT_CARRAY`.
    pub const CARRAY: u16 = 28;
    /// `VT_USERDEFINED`.
    pub const USERDEFINED: u16 = 29;
    /// `VT_LPSTR`.
    pub const LPSTR: u16 = 30;
    /// `VT_LPWSTR`.
    pub const LPWSTR: u16 = 31;
}

/// A reference to another type: in this library or imported from another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeRef {
    /// Index into [`TypeLib::types`].
    Local(usize),
    /// Index into [`TypeLib::imports`].
    Imported(usize),
}

/// A type description (`TYPEDESC`).
#[derive(Debug, Clone, PartialEq)]
pub enum TypeDesc {
    /// A primitive `VT_*` type.
    Prim(u16),
    /// `VT_PTR` to a type.
    Ptr(Box<TypeDesc>),
    /// `VT_SAFEARRAY` of a type.
    SafeArray(Box<TypeDesc>),
    /// `VT_CARRAY`: a fixed array.
    CArray {
        /// Element type.
        elem: Box<TypeDesc>,
        /// `(element count, lower bound)` per dimension.
        bounds: Vec<(u32, i32)>,
    },
    /// `VT_USERDEFINED`: an enum, record, interface, alias, ...
    User(TypeRef),
}

/// The kind of a type (`TYPEKIND`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeKind {
    /// `TKIND_ENUM`.
    Enum,
    /// `TKIND_RECORD`.
    Record,
    /// `TKIND_MODULE`.
    Module,
    /// `TKIND_INTERFACE`.
    Interface,
    /// `TKIND_DISPATCH`.
    Dispatch,
    /// `TKIND_COCLASS`.
    CoClass,
    /// `TKIND_ALIAS`.
    Alias,
    /// `TKIND_UNION`.
    Union,
}

/// Kind of a function (`FUNCKIND`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FuncKind {
    /// `FUNC_VIRTUAL`.
    Virtual,
    /// `FUNC_PUREVIRTUAL`.
    PureVirtual,
    /// `FUNC_NONVIRTUAL`.
    NonVirtual,
    /// `FUNC_STATIC`.
    Static,
    /// `FUNC_DISPATCH`.
    Dispatch,
}

/// Invoke kind (`INVOKEKIND`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvokeKind {
    /// `INVOKE_FUNC`: method.
    Func,
    /// `INVOKE_PROPERTYGET`.
    PropertyGet,
    /// `INVOKE_PROPERTYPUT`: `Let`.
    PropertyPut,
    /// `INVOKE_PROPERTYPUTREF`: `Set`.
    PropertyPutRef,
}

/// Calling convention (`CALLCONV`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallConv {
    /// `CC_FASTCALL`.
    Fastcall,
    /// `CC_CDECL`.
    Cdecl,
    /// `CC_MSCPASCAL` / `CC_PASCAL`.
    Pascal,
    /// `CC_MACPASCAL`.
    MacPascal,
    /// `CC_STDCALL`.
    Stdcall,
    /// `CC_FPFASTCALL`.
    FpFastcall,
    /// `CC_SYSCALL`.
    Syscall,
    /// `CC_MPWCDECL`.
    MpwCdecl,
    /// `CC_MPWPASCAL`.
    MpwPascal,
    /// A value outside the documented range.
    Other(u8),
}

/// Kind of a variable (`VARKIND`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarKind {
    /// `VAR_PERINSTANCE`: a record field.
    PerInstance,
    /// `VAR_STATIC`.
    Static,
    /// `VAR_CONST`: an enum member or constant.
    Const,
    /// `VAR_DISPATCH`: a dispinterface property.
    Dispatch,
}

/// A constant value stored in a library (a default value or a constant).
#[derive(Debug, Clone, PartialEq)]
pub struct Const {
    /// The `VT_*` type tag.
    pub vt: u16,
    /// The value.
    pub value: ConstValue,
}

/// The payload of a [`Const`].
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    /// A signed integer (`I1`, `I2`, `I4`, `I8`, `INT`, `ERROR`).
    Int(i64),
    /// An unsigned integer (`UI1`, `UI2`, `UI4`, `UI8`, `UINT`).
    UInt(u64),
    /// A floating-point value (`R4`, `R8`, `DATE`).
    Real(f64),
    /// A `Currency` value scaled by 10000.
    Currency(i64),
    /// A `Boolean`.
    Bool(bool),
    /// A string.
    Str(String),
    /// A null object reference (`Nothing`): `VT_DISPATCH`/`VT_UNKNOWN`
    /// defaults.
    Nothing,
    /// A type tag whose payload layout is not understood.
    Unsupported,
}

/// A parameter of a function.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// Name, if the library records one.
    pub name: Option<String>,
    /// Declared type.
    pub ty: TypeDesc,
    /// Flags.
    pub flags: ParamFlags,
    /// Default value (when [`ParamFlags::HAS_DEFAULT`] is set and readable).
    pub default: Option<Const>,
}

/// A function, method or property accessor.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Name.
    pub name: String,
    /// Member id (the DISPID for dispatch members).
    pub memid: i32,
    /// Function kind.
    pub kind: FuncKind,
    /// Invoke kind.
    pub invoke: InvokeKind,
    /// Calling convention.
    pub call_conv: CallConv,
    /// Flags.
    pub flags: FuncFlags,
    /// Offset in the vtable in bytes (for virtual members).
    pub vtable_offset: u16,
    /// Declared return type (`HRESULT` for most dual members, `VOID` for subs).
    pub ret: TypeDesc,
    /// Parameters, in order.
    pub params: Vec<Param>,
    /// Number of optional parameters; `-1` means a `ParamArray`-style tail.
    pub optional_count: i16,
    /// Help string.
    pub doc: Option<String>,
    /// Help context id (`0` when absent).
    pub help_context: u32,
}

/// A variable, record field, enum member or constant.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    /// Name.
    pub name: String,
    /// Member id.
    pub memid: i32,
    /// Variable kind.
    pub kind: VarKind,
    /// Declared type.
    pub ty: TypeDesc,
    /// Flags.
    pub flags: VarFlags,
    /// Byte offset in the record (for [`VarKind::PerInstance`]).
    pub offset: Option<u32>,
    /// Constant value (for [`VarKind::Const`]).
    pub value: Option<Const>,
    /// Help string.
    pub doc: Option<String>,
    /// Help context id.
    pub help_context: u32,
}

/// An implemented interface or coclass member type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplType {
    /// The implemented type.
    pub target: TypeRef,
    /// Flags (default, source, restricted).
    pub flags: ImplFlags,
}

/// One type of a library.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    /// Name.
    pub name: String,
    /// Kind.
    pub kind: TypeKind,
    /// GUID (IID/CLSID), if any.
    pub guid: Option<Guid>,
    /// Flags.
    pub flags: TypeFlags,
    /// Version.
    pub version: Version,
    /// Help string.
    pub doc: Option<String>,
    /// Help context id.
    pub help_context: u32,
    /// Required alignment of a record, in bytes.
    pub alignment: u8,
    /// Size of an instance (records) in bytes.
    pub instance_size: u32,
    /// Size of the vtable in bytes (interfaces).
    pub vtable_size: u16,
    /// The aliased type (for [`TypeKind::Alias`]).
    pub alias: Option<TypeDesc>,
    /// Implemented/inherited types (base interface, coclass members).
    pub implements: Vec<ImplType>,
    /// Functions in file order.
    pub functions: Vec<Function>,
    /// Variables in file order.
    pub variables: Vec<Variable>,
    /// DLL name (for [`TypeKind::Module`]).
    pub dll_name: Option<String>,
}

/// A library the current library imports types from.
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    /// The imported library's GUID (`None` if unreadable).
    pub library_guid: Option<Guid>,
    /// Its LCID.
    pub lcid: u32,
    /// Its version.
    pub version: Version,
    /// File name recorded for it (for example `stdole2.tlb`).
    pub file: String,
    /// The GUID of the imported type.
    pub type_guid: Option<Guid>,
    /// Kind of the imported type, if recorded.
    pub kind: Option<TypeKind>,
}

/// A loaded type library.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeLib {
    /// Library name (`stdole`, `Scripting`).
    pub name: String,
    /// LIBID.
    pub guid: Guid,
    /// LCID of the library.
    pub lcid: u32,
    /// Version.
    pub version: Version,
    /// Library flags.
    pub flags: LibFlags,
    /// Target system kind recorded in the header (`0` Win16, `1` Win32,
    /// `2` Mac, `3` Win64), if in range.
    pub sys_kind: Option<u8>,
    /// Library help string.
    pub doc: Option<String>,
    /// Help context id.
    pub help_context: u32,
    /// Imported libraries/types, referenced by [`TypeRef::Imported`].
    pub imports: Vec<Import>,
    /// All types in file order.
    pub types: Vec<TypeInfo>,
    /// Soft problems found while reading (data that was ignored).
    pub warnings: Vec<String>,
}

impl TypeLib {
    /// Finds a type by name (ASCII case-insensitive).
    pub fn type_by_name(&self, name: &str) -> Option<(usize, &TypeInfo)> {
        self.types
            .iter()
            .enumerate()
            .find(|(_, t)| t.name.eq_ignore_ascii_case(name))
    }

    /// Finds a type by GUID.
    pub fn type_by_guid(&self, guid: &Guid) -> Option<(usize, &TypeInfo)> {
        self.types
            .iter()
            .enumerate()
            .find(|(_, t)| t.guid.as_ref() == Some(guid))
    }

    /// The type a [`TypeRef::Local`] designates.
    pub fn local(&self, r: TypeRef) -> Option<&TypeInfo> {
        match r {
            TypeRef::Local(i) => self.types.get(i),
            TypeRef::Imported(_) => None,
        }
    }

    /// The import a [`TypeRef::Imported`] designates.
    pub fn import(&self, r: TypeRef) -> Option<&Import> {
        match r {
            TypeRef::Imported(i) => self.imports.get(i),
            TypeRef::Local(_) => None,
        }
    }
}
