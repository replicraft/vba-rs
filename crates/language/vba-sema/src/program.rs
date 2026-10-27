//! The project-level semantic model: modules, symbols, signatures and
//! bound procedure bodies.

use std::collections::HashMap;

use vba_ast::{CompareMode, Name, ProcKind, Visibility};
use vba_diagnostics::Diagnostic;
use vba_profile::Profile;
use vba_project::ModuleKind;
use vba_source::{FileId, SourceMap, Span};

use crate::bound::{BStmt, Constant};
use crate::types::{
    ConstId, DeclareId, EnumId, EventId, GlobalId, LabelId, LibraryType, LocalId, ModuleId, ProcId,
    Type, UdtId,
};

/// A `Property Get/Let/Set` triple sharing one name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PropGroup {
    /// `Property Get`.
    pub get: Option<ProcId>,
    /// `Property Let`.
    pub let_: Option<ProcId>,
    /// `Property Set`.
    pub set: Option<ProcId>,
}

/// What a module-scope name denotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbol {
    /// A module-level variable.
    Var(GlobalId),
    /// A constant.
    Const(ConstId),
    /// A `Sub` or `Function`.
    Proc(ProcId),
    /// A property group.
    Property(PropGroup),
    /// A `Declare`.
    Declare(DeclareId),
    /// An `Enum` type.
    Enum(EnumId),
    /// An enum member (enum, member index).
    EnumMember(EnumId, u32),
    /// A `Type`.
    Udt(UdtId),
    /// An `Event`.
    Event(EventId),
}

/// `Option` settings and `Def<Type>` defaults of a module.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleOptions {
    /// `Option Explicit`.
    pub explicit: bool,
    /// `Option Base` (0 or 1).
    pub base: u8,
    /// `Option Compare`.
    pub compare: CompareMode,
    /// `Option Private Module`.
    pub private_module: bool,
    /// Default type of implicitly typed names starting with each letter
    /// `A..=Z` (from `Def<Type>`), `Variant` unless declared.
    pub def_types: Vec<Type>,
}

impl Default for ModuleOptions {
    fn default() -> Self {
        ModuleOptions {
            explicit: false,
            base: 0,
            compare: CompareMode::Binary,
            private_module: false,
            def_types: vec![Type::Variant; 26],
        }
    }
}

impl ModuleOptions {
    /// The default type for a name by its first letter.
    pub fn default_type_for(&self, name: &Name) -> Type {
        match name.as_bytes().first() {
            Some(c) if c.is_ascii_alphabetic() => {
                self.def_types[(c.to_ascii_uppercase() - b'A') as usize].clone()
            }
            _ => Type::Variant,
        }
    }
}

/// The reference from an event handler to the event it handles.
#[derive(Debug, Clone, PartialEq)]
pub enum EventRef {
    /// An event declared in a project class.
    Project(EventId),
    /// An event of a library type.
    Library {
        /// The library type raising it.
        ty: LibraryType,
        /// Event name.
        name: Name,
    },
}

/// A procedure that handles an event of a `WithEvents` variable.
#[derive(Debug, Clone, PartialEq)]
pub struct EventHandler {
    /// The `WithEvents` variable.
    pub var: GlobalId,
    /// The event.
    pub event: EventRef,
    /// The handler.
    pub proc: ProcId,
}

/// One implemented interface and the mapping of its members.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplementsInfo {
    /// The interface (a class of the project or a library type).
    pub interface: Type,
    /// `(interface member, implementing procedure)` pairs. For a project
    /// interface the first element is the interface's procedure; public
    /// variables of the interface appear through their synthesized accessors
    /// only if the interface declares properties (variables are not
    /// implementable and are reported).
    pub members: Vec<(ProcId, ProcId)>,
    /// Source span of the `Implements` statement.
    pub span: Span,
}

/// A module of the project.
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Identifier.
    pub id: ModuleId,
    /// Name (`VB_Name`).
    pub name: Name,
    /// Role.
    pub kind: ModuleKind,
    /// File of the module source.
    pub file: FileId,
    /// Options.
    pub options: ModuleOptions,
    /// Module-scope names.
    pub symbols: HashMap<Name, (Symbol, Span)>,
    /// Procedures in source order.
    pub procs: Vec<ProcId>,
    /// Module-level variables in source order.
    pub globals: Vec<GlobalId>,
    /// Constants.
    pub consts: Vec<ConstId>,
    /// User-defined types.
    pub udts: Vec<UdtId>,
    /// Enumerations.
    pub enums: Vec<EnumId>,
    /// `Declare`s.
    pub declares: Vec<DeclareId>,
    /// Events declared by the module.
    pub events: Vec<EventId>,
    /// Implemented interfaces.
    pub implements: Vec<ImplementsInfo>,
    /// Event handlers for `WithEvents` variables.
    pub handlers: Vec<EventHandler>,
    /// Whether a default instance exists (`VB_PredeclaredId`, forms and
    /// document modules).
    pub predeclared: bool,
    /// The class default member (`Attribute x.VB_UserMemId = 0`).
    pub default_member: Option<Name>,
    /// The member `For Each` enumerates with (`Attribute x.VB_UserMemId = -4`,
    /// `DISPID_NEWENUM`): it returns the enumerator object.
    pub enumerator_member: Option<Name>,
    /// `Class_Initialize`.
    pub initialize: Option<ProcId>,
    /// `Class_Terminate`.
    pub terminate: Option<ProcId>,
    /// Whether the source had syntax errors; such modules are analysed on a
    /// best-effort basis and the program is not executable.
    pub has_syntax_errors: bool,
}

impl ModuleInfo {
    /// Whether the module is a class-like module (instances are created).
    pub fn is_class_like(&self) -> bool {
        self.kind != ModuleKind::Standard
    }
}

/// A formal parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamInfo {
    /// Name.
    pub name: Name,
    /// Declared (or defaulted) type.
    pub ty: Type,
    /// Passed `ByVal`.
    pub by_val: bool,
    /// `Optional`.
    pub optional: bool,
    /// Constant default of an optional parameter.
    pub default: Option<Constant>,
    /// `ParamArray`.
    pub param_array: bool,
    /// Declared with `()`.
    pub is_array: bool,
    /// Source span.
    pub span: Span,
}

/// How a local slot came to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    /// The n-th parameter.
    Param(u32),
    /// A declared local variable.
    Var,
    /// The implicit variable named like the function, holding its result.
    Result,
    /// An implicitly declared variable (no `Option Explicit`).
    Implicit,
    /// A compiler temporary (`With` object).
    Temp,
}

/// A local slot.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalInfo {
    /// Name (empty for temporaries).
    pub name: Name,
    /// Type.
    pub ty: Type,
    /// Origin.
    pub kind: LocalKind,
    /// Fixed array bounds `(lower, upper)` per dimension.
    pub fixed_bounds: Option<Vec<(i32, i32)>>,
    /// `Dim x As New C`: the variable instantiates itself on first use.
    pub auto_new: bool,
    /// Declaration span.
    pub span: Span,
}

/// How a label is written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LabelKey {
    /// `name:`
    Name(Name),
    /// `100`
    Line(u64),
}

/// A label of a procedure.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelInfo {
    /// How it is written.
    pub key: LabelKey,
    /// Definition span.
    pub span: Span,
    /// Line number in effect for `Erl` (`Some` for numeric labels).
    pub line: Option<u64>,
}

/// A procedure.
#[derive(Debug, Clone)]
pub struct ProcInfo {
    /// Identifier.
    pub id: ProcId,
    /// Owning module.
    pub module: ModuleId,
    /// Name.
    pub name: Name,
    /// Kind.
    pub kind: ProcKind,
    /// Effective visibility.
    pub visibility: Visibility,
    /// Declared `Static`.
    pub is_static: bool,
    /// Parameters.
    pub params: Vec<ParamInfo>,
    /// Result type (`Variant` for `Sub`/`Property Let`/`Set`: meaningless).
    pub ret: Type,
    /// Whether the procedure returns a value.
    pub has_result: bool,
    /// Local slots: parameters first (in order), then the result slot (if
    /// any), then other locals and temporaries.
    pub locals: Vec<LocalInfo>,
    /// The result slot.
    pub result_local: Option<LocalId>,
    /// Labels.
    pub labels: Vec<LabelInfo>,
    /// Bound body.
    pub body: Vec<BStmt>,
    /// Full span.
    pub span: Span,
    /// Span of the name.
    pub name_span: Span,
}

impl ProcInfo {
    /// A display name such as `Property Get Foo`.
    pub fn describe(&self) -> String {
        match self.kind {
            ProcKind::Sub => format!("Sub {}", self.name),
            ProcKind::Function => format!("Function {}", self.name),
            ProcKind::PropertyGet => format!("Property Get {}", self.name),
            ProcKind::PropertyLet => format!("Property Let {}", self.name),
            ProcKind::PropertySet => format!("Property Set {}", self.name),
        }
    }
}

/// A module-level variable.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalInfo {
    /// Identifier.
    pub id: GlobalId,
    /// Owning module.
    pub module: ModuleId,
    /// Name.
    pub name: Name,
    /// Type.
    pub ty: Type,
    /// Effective visibility.
    pub visibility: Visibility,
    /// `WithEvents`.
    pub with_events: bool,
    /// `As New`.
    pub auto_new: bool,
    /// The procedure of a hoisted `Static` local.
    pub static_of: Option<ProcId>,
    /// Fixed array bounds per dimension.
    pub fixed_bounds: Option<Vec<(i32, i32)>>,
    /// Declaration span.
    pub span: Span,
}

/// A named constant.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstInfo {
    /// Identifier.
    pub id: ConstId,
    /// Owning module.
    pub module: ModuleId,
    /// Name.
    pub name: Name,
    /// Type.
    pub ty: Type,
    /// Value.
    pub value: Constant,
    /// Effective visibility.
    pub visibility: Visibility,
    /// Declaration span.
    pub span: Span,
}

/// A field of a user-defined type.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInfo {
    /// Name.
    pub name: Name,
    /// Type.
    pub ty: Type,
    /// Fixed array bounds per dimension.
    pub fixed_bounds: Option<Vec<(i32, i32)>>,
    /// Span.
    pub span: Span,
}

/// A user-defined type.
#[derive(Debug, Clone, PartialEq)]
pub struct UdtInfo {
    /// Identifier.
    pub id: UdtId,
    /// Owning module.
    pub module: ModuleId,
    /// Name.
    pub name: Name,
    /// Visibility.
    pub visibility: Visibility,
    /// Fields in order.
    pub fields: Vec<FieldInfo>,
    /// Span.
    pub span: Span,
}

/// An enumeration.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumInfo {
    /// Identifier.
    pub id: EnumId,
    /// Owning module.
    pub module: ModuleId,
    /// Name.
    pub name: Name,
    /// Visibility.
    pub visibility: Visibility,
    /// Members with their values.
    pub members: Vec<(Name, i32)>,
    /// Span.
    pub span: Span,
}

/// A `Declare` statement: an external procedure.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclareInfo {
    /// Identifier.
    pub id: DeclareId,
    /// Owning module.
    pub module: ModuleId,
    /// Name.
    pub name: Name,
    /// Visibility.
    pub visibility: Visibility,
    /// `Function` (otherwise `Sub`).
    pub is_function: bool,
    /// Library name.
    pub lib: Vec<u8>,
    /// Alias name.
    pub alias: Option<Vec<u8>>,
    /// `PtrSafe` written.
    pub ptr_safe: bool,
    /// Parameters.
    pub params: Vec<ParamInfo>,
    /// Result type.
    pub ret: Type,
    /// Span.
    pub span: Span,
}

/// A library enumeration the project refers to: its members by name, for
/// tools that show enum members (hover, completion) without asking the
/// library again.
#[derive(Debug, Clone, PartialEq)]
pub struct LibEnumInfo {
    /// The library type.
    pub ty: LibraryType,
    /// Members with their values, in declaration order.
    pub members: Vec<(String, i64)>,
}

/// A library record (UDT) the project refers to. Records get a synthetic
/// [`UdtId`] numbered after the project's own user-defined types (so ids at
/// or above `Program::udts.len()` denote library records); use
/// [`Program::udt_name`] and [`Program::udt_fields`] to read either kind.
#[derive(Debug, Clone, PartialEq)]
pub struct LibRecordInfo {
    /// The synthetic id.
    pub id: UdtId,
    /// The library type.
    pub ty: LibraryType,
    /// Fields in declaration order.
    pub fields: Vec<FieldInfo>,
    /// Byte offset of each field in the library's layout, when known.
    pub offsets: Vec<Option<u32>>,
    /// Size of an instance in bytes (`0` when unknown).
    pub size: u32,
    /// Alignment in bytes (`0` when unknown).
    pub alignment: u32,
}

/// An `Event` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct EventInfo {
    /// Identifier.
    pub id: EventId,
    /// Owning (class) module.
    pub module: ModuleId,
    /// Name.
    pub name: Name,
    /// Parameters.
    pub params: Vec<ParamInfo>,
    /// Span.
    pub span: Span,
}

/// The analysed project.
#[derive(Debug, Clone)]
pub struct Program {
    /// The profile the program was analysed for.
    pub profile: Profile,
    /// Modules, indexed by [`ModuleId`].
    pub modules: Vec<ModuleInfo>,
    /// Procedures, indexed by [`ProcId`].
    pub procs: Vec<ProcInfo>,
    /// Variables, indexed by [`GlobalId`].
    pub globals: Vec<GlobalInfo>,
    /// Constants, indexed by [`ConstId`].
    pub consts: Vec<ConstInfo>,
    /// User-defined types, indexed by [`UdtId`].
    pub udts: Vec<UdtInfo>,
    /// Enumerations, indexed by [`EnumId`].
    pub enums: Vec<EnumInfo>,
    /// Declares, indexed by [`DeclareId`].
    pub declares: Vec<DeclareInfo>,
    /// Events, indexed by [`EventId`].
    pub events: Vec<EventInfo>,
    /// Library enumerations used by the project.
    pub lib_enums: Vec<LibEnumInfo>,
    /// Library records used by the project, numbered after `udts`.
    pub lib_records: Vec<LibRecordInfo>,
}

impl Program {
    /// The module with the given name.
    pub fn module_by_name(&self, name: &str) -> Option<&ModuleInfo> {
        let n = Name::new(name);
        self.modules.iter().find(|m| m.name == n)
    }

    /// Module accessor.
    pub fn module(&self, id: ModuleId) -> &ModuleInfo {
        &self.modules[id.index()]
    }

    /// Procedure accessor.
    pub fn proc(&self, id: ProcId) -> &ProcInfo {
        &self.procs[id.index()]
    }

    /// Variable accessor.
    pub fn global(&self, id: GlobalId) -> &GlobalInfo {
        &self.globals[id.index()]
    }

    /// Finds a procedure by module and name (first match; property groups
    /// have several).
    pub fn find_proc(&self, module: &str, name: &str) -> Option<&ProcInfo> {
        let m = self.module_by_name(module)?;
        let n = Name::new(name);
        m.procs.iter().map(|p| self.proc(*p)).find(|p| p.name == n)
    }

    /// The name of a user-defined type of the project or of a library.
    pub fn udt_name(&self, id: UdtId) -> String {
        match self.udts.get(id.index()) {
            Some(u) => u.name.to_string(),
            None => self
                .lib_record(id)
                .map_or_else(|| format!("udt#{}", id.0), |r| r.ty.name.clone()),
        }
    }

    /// The fields of a user-defined type of the project or of a library.
    pub fn udt_fields(&self, id: UdtId) -> &[FieldInfo] {
        match self.udts.get(id.index()) {
            Some(u) => &u.fields,
            None => self.lib_record(id).map_or(&[], |r| &r.fields),
        }
    }

    /// The library record with a synthetic id.
    pub fn lib_record(&self, id: UdtId) -> Option<&LibRecordInfo> {
        let i = id.index().checked_sub(self.udts.len())?;
        self.lib_records.get(i)
    }

    /// The synthetic id of a library record type.
    pub fn lib_record_id(&self, ty: &LibraryType) -> Option<UdtId> {
        self.lib_records.iter().find(|r| r.ty == *ty).map(|r| r.id)
    }

    /// The members of a library enum the project refers to.
    pub fn lib_enum(&self, ty: &LibraryType) -> Option<&LibEnumInfo> {
        self.lib_enums.iter().find(|e| e.ty == *ty)
    }

    /// A readable name of a type.
    pub fn type_name(&self, t: &Type) -> String {
        match t {
            Type::Class(m) => self.module(*m).name.to_string(),
            Type::Udt(u) => self.udts[u.index()].name.to_string(),
            Type::Enum(e) => self.enums[e.index()].name.to_string(),
            Type::LibEnum(l) | Type::LibRecord(l) => l.to_string(),
            Type::Array { elem, rank } => {
                let e = self.type_name(elem);
                match rank {
                    Some(r) => format!("{e}({})", ",".repeat(usize::from(*r).saturating_sub(1))),
                    None => format!("{e}()"),
                }
            }
            t => t.to_string(),
        }
    }

    /// Labels of a procedure by id.
    pub fn label(&self, proc: ProcId, id: LabelId) -> &LabelInfo {
        &self.procs[proc.index()].labels[id.index()]
    }
}

/// The result of analysing a project.
#[derive(Debug, Clone)]
pub struct Analysis {
    /// The bound program.
    pub program: Program,
    /// All diagnostics (lexer, parser, AST lowering and semantic analysis),
    /// in deterministic order.
    pub diagnostics: Vec<Diagnostic>,
    /// The sources, for rendering diagnostics.
    pub sources: SourceMap,
    /// The size budget the project went over, if any. The analysis is then
    /// partial (bodies after the point of exhaustion are not bound) and
    /// carries an error diagnostic.
    pub limit_exceeded: Option<crate::AnalysisLimit>,
}

impl Analysis {
    /// Whether any diagnostic is an error.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == vba_diagnostics::Severity::Error)
    }

    /// Diagnostics with the given code.
    pub fn with_code(&self, code: vba_diagnostics::Code) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(move |d| d.code == code)
    }
}
