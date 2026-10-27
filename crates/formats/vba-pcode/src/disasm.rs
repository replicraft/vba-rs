//! Typed disassembly with resolved operands and a stable text format.
//!
//! # Text format (version 1)
//!
//! ```text
//! ; vba-pcode disassembly v1
//! ; layout VBA7 x86 version=0x0097 verified
//! ; lines 3
//! L0000 +0000 OPTION_STMT mod=7
//! L0002 +0000 PROC_HEADER mod=6 proc{Public Function F(ByVal a As String) As Boolean}
//! L0003 +0000 CONST_TEXT "hi" READ_SIMPLE ...
//! ```
//!
//! One instruction per row: `L<line, 4 digits>`, `+<offset in line, 4 hex
//! digits>`, the opcode label of `pcode-opcodes.md`, ` mod=<n>` when the
//! modifier is non-zero, then the operands separated by spaces. Lines whose
//! code stops decoding end with a row `L<line> ERROR <message>`. Operand
//! spellings:
//!
//! * names: the identifier with its type suffix, `[name]` when bracketed,
//!   `Me`, `-` for the empty name, `?builtin(0xNN)` and `?ident(N)` for
//!   references that cannot be resolved;
//! * counts `argc=N`, plain words `w=N`;
//! * integers in the radix they were written (`12`, `&H1F`, `&O17`);
//!   `f32:`, `f64:`, `cur:` and `date:` prefixes for the other numbers;
//! * strings `"..."` with `"` doubled; bytes outside printable ASCII and the
//!   backslash are written `\xNN`; comments `'text` in the same escaping;
//! * objects `obj@<offset>(<class name>)`;
//! * records `proc{...}`, `var{...}`, `udt{...}` or `rec@0x<offset>`.

use std::fmt::{self, Write as _};

use crate::cache::ModuleCache;
use crate::decode::{DecodeError, Instruction, Operands, RecRef, decode_line};
use crate::layout::{Bitness, Verification};
use crate::names::{Name, Resolver, name_suffix};
use crate::opcode::Opcode;
use crate::project_stream::IdentTable;
use crate::records::{
    ArgumentRecord, DeclarationRecord, FunctionRecord, OBJECT_ENTRY_SIZE, ProcKind, TypeId,
    TypeRef, UdtRecord, VariableRecord, argument_chain, object_entry, type_descriptor,
    type_ref_area, type_ref_record, variable_type,
};

/// Radix an integer literal was written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radix {
    /// Decimal.
    Decimal,
    /// Hexadecimal (`&H`).
    Hex,
    /// Octal (`&O`).
    Octal,
}

/// A resolved argument of a procedure.
#[derive(Debug, Clone, PartialEq)]
pub struct ArgInfo {
    /// Name of the argument (`Name::None` for the unnamed return slot).
    pub name: Name,
    /// `ByVal` was written.
    pub by_val: bool,
    /// `Optional` was written.
    pub optional: bool,
    /// An `Optional` argument with a default value (`= expr`).
    pub has_default: bool,
    /// `ParamArray` was written.
    pub param_array: bool,
    /// Declared type.
    pub ty: ResolvedType,
}

/// A type with the names resolved.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedType {
    /// No type.
    None,
    /// Intrinsic type id.
    Intrinsic(TypeId),
    /// A class or user type.
    Named(Name),
    /// A library-qualified type (`Lib.Type`): the qualifier and the type name.
    Qualified(QualifiedType),
    /// Unresolved reference (raw value).
    Unknown(u32),
}

/// A library-qualified type reference with the name resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedType {
    /// Library handle of the type-reference record: the identifier
    /// reference of the qualifier (`[obs]`, equal for the types of one
    /// library within a project).
    pub library: u16,
    /// The qualifier as an identifier (the project or library name).
    pub qualifier: Name,
    /// The type name.
    pub name: Name,
}

/// A resolved procedure header record.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcedureInfo {
    /// Procedure name.
    pub name: Name,
    /// Kind from the record flags.
    pub kind: ProcKind,
    /// Whether the header instruction says `Function` (MOD 2 or 6).
    pub is_function: bool,
    /// `Public` according to the header MOD (bit 4).
    pub public: bool,
    /// `Static` flag of the record.
    pub is_static: bool,
    /// External `Declare`.
    pub is_declare: bool,
    /// Library of a `Declare`, when it resolves.
    pub library: Option<Name>,
    /// Entry-point text of a `Declare` (the `Alias`, or the name itself).
    pub entry_point: Option<Vec<u8>>,
    /// Arguments.
    pub args: Vec<ArgInfo>,
    /// Declared return type.
    pub return_type: ResolvedType,
    /// The raw record (for consumers needing the unresolved fields).
    pub record: FunctionRecord,
}

/// A resolved variable record.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableInfo {
    /// Variable name.
    pub name: Name,
    /// Declared type.
    pub ty: ResolvedType,
    /// Declared `New`.
    pub is_new: bool,
}

/// A resolved operand.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// An identifier reference.
    Name {
        /// The resolved name.
        name: Name,
        /// Type-suffix character selected by the modifier.
        suffix: Option<char>,
        /// Written in square brackets.
        bracketed: bool,
    },
    /// An argument or element count.
    Count(u16),
    /// A plain word.
    Word(u16),
    /// A source column: the position of a `'` comment in its line (first
    /// word of `COMMENT_QUOTE`, `[obs]`: 2636 of 2640 comments match the
    /// column of the apostrophe) or of a statement start (`STMT_BEGIN`).
    Column(u16),
    /// Line-continuation breaks: per break the number of tokens before it
    /// and the indentation of the continuation line (`[obs]`, 292 of 318
    /// pairs match; negative values occur inside comments).
    Continuation(Vec<(u16, i16)>),
    /// An integer literal.
    Int {
        /// The value (sign-extended).
        value: i64,
        /// Radix written.
        radix: Radix,
    },
    /// A single-precision literal.
    Float32(f32),
    /// A double-precision literal.
    Float64(f64),
    /// A Currency literal (value times 10000).
    Currency(i64),
    /// A Date literal (serial number).
    Date(f64),
    /// A string literal (project code page bytes).
    Text(Vec<u8>),
    /// A comment; the word is the first operand of the instruction when it
    /// has one (meaning unknown).
    Comment(Vec<u8>),
    /// Unparsed text or other opaque trailer bytes.
    Bytes(Vec<u8>),
    /// A list of names (computed branch targets).
    NameList(Vec<Name>),
    /// An object-table reference.
    Object {
        /// The operand as stored: the entry index times the pointer size
        /// (4 on x86, 8 on x64).
        offset: u16,
        /// Class name, when the entry is in range and not library-qualified.
        class: Option<Name>,
        /// The type, when the entry names a library-qualified type.
        qualified: Option<QualifiedType>,
    },
    /// A procedure header record.
    Procedure(Box<ProcedureInfo>),
    /// A variable record.
    Variable(VariableInfo),
    /// A user-defined-type record.
    UserType {
        /// Type name.
        name: Name,
        /// Declared `Private`.
        private: bool,
    },
    /// An indirect-table offset whose record kind is not decoded.
    Record(u32),
    /// A record offset that points outside the table or fails to parse.
    BadRecord(u32),
    /// Procedure context dword(s).
    Context(u32, Option<u32>),
    /// Several plain words (shapes without a known meaning).
    Words(Vec<u16>),
    /// The interface of an `Implements` statement: the first word of the
    /// statement is the position of the interface record in the indirect
    /// table plus 8, and the record starts with a type descriptor.
    Interface(ResolvedType),
}

/// One instruction with resolved operands.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    /// Offset in the line.
    pub offset: usize,
    /// Opcode.
    pub opcode: Opcode,
    /// Modifier field.
    pub modifier: u8,
    /// Operands.
    pub operands: Vec<Operand>,
}

/// The listing of one source line.
#[derive(Debug, Clone, PartialEq)]
pub struct LineListing {
    /// Line index (0-based, after the attribute lines).
    pub index: usize,
    /// Instructions.
    pub instructions: Vec<Listed>,
    /// Why decoding stopped, if it did.
    pub error: Option<DecodeError>,
}

/// A disassembled module.
#[derive(Debug, Clone, PartialEq)]
pub struct Disassembly {
    /// Header lines describing the layout.
    pub layout_note: String,
    /// One entry per line of the line table.
    pub lines: Vec<LineListing>,
}

impl Disassembly {
    /// Disassembles every line of `cache`.
    pub fn of_module(cache: &ModuleCache<'_>, idents: &IdentTable) -> Self {
        let d = Disassembler::new(cache, idents);
        let mut lines = Vec::with_capacity(cache.lines.lines.len());
        for i in 0..cache.lines.lines.len() {
            lines.push(d.line(i));
        }
        let l = cache.layout;
        let verified = match l.verification() {
            Verification::Verified => "verified",
            Verification::SpecUnverified => "spec-unverified",
        };
        Self {
            layout_note: format!(
                "{:?} {} version={:#06x} {verified}",
                l.family,
                match l.bitness {
                    Bitness::X86 => "x86",
                    Bitness::X64 => "x64",
                },
                l.version
            ),
            lines,
        }
    }
}

/// Disassembles lines of one module cache.
#[derive(Debug, Clone)]
pub struct Disassembler<'a> {
    cache: &'a ModuleCache<'a>,
    resolver: Resolver<'a>,
    indirect: &'a [u8],
    objects: &'a [u8],
    type_refs: &'a [u8],
    declarations: &'a [u8],
    /// Argument records this disassembler may still list. Many instructions
    /// can name one procedure record, and each listing copies its argument
    /// chain, so without a budget the output grows with instructions times
    /// chain length.
    arg_budget: std::cell::Cell<usize>,
}

/// Most argument records listed over all procedure operands of one
/// [`Disassembler`].
const MAX_LISTED_ARGS: usize = 1 << 18;

impl<'a> Disassembler<'a> {
    /// Creates a disassembler for `cache` using the project identifiers.
    pub fn new(cache: &'a ModuleCache<'a>, idents: &'a IdentTable) -> Self {
        Self {
            cache,
            resolver: Resolver::new(idents, cache.layout),
            indirect: &cache.bytes[cache.tables.indirect.clone()],
            objects: &cache.bytes[cache.tables.objects.clone()],
            type_refs: type_ref_area(
                cache.bytes,
                cache.tables.objects.end,
                cache.tables.indirect.start,
            )
            .map_or(&[][..], |r| &cache.bytes[r]),
            declarations: &cache.bytes[cache.tables.declarations.clone()],
            arg_budget: std::cell::Cell::new(MAX_LISTED_ARGS),
        }
    }

    /// The raw decode of line `index` (no operand resolution).
    pub fn decode(&self, index: usize) -> Option<crate::decode::DecodedLine> {
        let range = self
            .cache
            .lines
            .code_range(index, self.cache.bytes.len())
            .ok()??;
        Some(decode_line(&self.cache.bytes[range], &self.cache.layout))
    }

    /// Disassembles line `index`.
    pub fn line(&self, index: usize) -> LineListing {
        match self.cache.lines.code_range(index, self.cache.bytes.len()) {
            Ok(None) => LineListing {
                index,
                instructions: Vec::new(),
                error: None,
            },
            Ok(Some(range)) => {
                let d = decode_line(&self.cache.bytes[range], &self.cache.layout);
                LineListing {
                    index,
                    instructions: d.instructions.iter().map(|i| self.list(i)).collect(),
                    error: d.error,
                }
            }
            Err(_) => LineListing {
                index,
                instructions: Vec::new(),
                error: Some(DecodeError::Truncated { offset: 0 }),
            },
        }
    }

    /// Byte offset in the object table of the entry an `obj` operand names:
    /// the operand is the entry index times the pointer size (4 or 8).
    fn object_entry_offset(&self, operand: u16) -> usize {
        let shift = if self.cache.layout.bitness == Bitness::X64 {
            3
        } else {
            2
        };
        usize::from(operand >> shift) * OBJECT_ENTRY_SIZE
    }

    /// Resolves the operands of one instruction.
    pub fn list(&self, ins: &Instruction) -> Listed {
        Listed {
            offset: ins.offset,
            opcode: ins.opcode,
            modifier: ins.modifier,
            operands: self.operands(ins),
        }
    }

    fn name(&self, ins: &Instruction, word: u16) -> Operand {
        let name = self.resolver.resolve(word);
        let m = match ins.opcode {
            Opcode::InvokeSimple | Opcode::InvokeDot | Opcode::InvokeWithDot => {
                ins.modifier.checked_sub(16).unwrap_or(ins.modifier)
            }
            Opcode::ErrHandlerSet | Opcode::ErrResume => 0,
            _ => ins.modifier,
        };
        let (suffix, bracketed) = name_suffix(m);
        Operand::Name {
            name,
            suffix,
            bracketed,
        }
    }

    fn operands(&self, ins: &Instruction) -> Vec<Operand> {
        match &ins.operands {
            Operands::None => Vec::new(),
            Operands::Name(n) => vec![self.name(ins, n.0)],
            Operands::NameArgs(n, c) => vec![self.name(ins, n.0), Operand::Count(*c)],
            Operands::Word(w) if ins.opcode == Opcode::StmtBegin => vec![Operand::Column(*w)],
            Operands::Word(w) => self.word_operand(ins.opcode, *w),
            Operands::Words2(w) => match ins.opcode {
                Opcode::ConstInt32Decimal => vec![int32(w, Radix::Decimal)],
                Opcode::ConstInt32Hex => vec![int32(w, Radix::Hex)],
                Opcode::ConstInt32Octal => vec![int32(w, Radix::Octal)],
                Opcode::ConstFloat32 => {
                    let bits = u32::from(w[0]) | (u32::from(w[1]) << 16);
                    vec![Operand::Float32(f32::from_bits(bits))]
                }
                Opcode::ImplementsStmt => {
                    let ty = match u32::from(w[0]).checked_sub(8) {
                        Some(pos) => match type_descriptor(
                            self.indirect,
                            self.objects,
                            pos,
                            &self.cache.layout,
                        ) {
                            Ok(t) => self.resolve_type(t),
                            Err(_) => ResolvedType::Unknown(pos),
                        },
                        None => ResolvedType::Unknown(0),
                    };
                    vec![Operand::Words(w.to_vec()), Operand::Interface(ty)]
                }
                _ => vec![Operand::Words(w.to_vec())],
            },
            Operands::Words4(w) => {
                let bits = w
                    .iter()
                    .enumerate()
                    .fold(0u64, |a, (i, &x)| a | (u64::from(x) << (16 * i)));
                match ins.opcode {
                    Opcode::ConstFloat64 => vec![Operand::Float64(f64::from_bits(bits))],
                    Opcode::ConstDate => vec![Operand::Date(f64::from_bits(bits))],
                    Opcode::ConstCurrency => vec![Operand::Currency(bits as i64)],
                    Opcode::ConstInt64Decimal => vec![Operand::Int {
                        value: bits as i64,
                        radix: Radix::Decimal,
                    }],
                    Opcode::ConstInt64Hex => vec![Operand::Int {
                        value: bits as i64,
                        radix: Radix::Hex,
                    }],
                    Opcode::ConstInt64Octal => vec![Operand::Int {
                        value: bits as i64,
                        radix: Radix::Octal,
                    }],
                    _ => vec![Operand::Words(w.to_vec())],
                }
            }
            Operands::Obj(o) => {
                let entry = object_entry(self.objects, self.object_entry_offset(o.0)).ok();
                vec![Operand::Object {
                    offset: o.0,
                    class: entry
                        .filter(|e| !e.is_qualified())
                        .map(|e| self.resolver.resolve_symbol(e.name)),
                    qualified: entry
                        .filter(|e| e.is_qualified() && e.name != 0xFFFF)
                        .and_then(|e| self.qualified(e.name >> self.pointer_shift())),
                }]
            }
            Operands::Rec(r) => vec![self.record(ins, *r)],
            Operands::VarDecl(r, extra) => {
                let mut v = vec![self.record(ins, *r)];
                if let Some(e) = extra {
                    v.push(Operand::Word(*e));
                }
                v
            }
            Operands::Ctx(a, b) => vec![Operand::Context(*a, *b)],
            Operands::NameArgsRec(n, c, r) => {
                vec![
                    self.name(ins, n.0),
                    Operand::Count(*c),
                    Operand::Record(r.0),
                ]
            }
            Operands::Trailer(t) => match ins.opcode {
                Opcode::ConstText => vec![Operand::Text(t.clone())],
                Opcode::CommentRemKeyword => vec![Operand::Comment(t.clone())],
                Opcode::LineContinuation if t.len() % 4 == 0 => {
                    vec![Operand::Continuation(
                        t.as_chunks::<4>()
                            .0
                            .iter()
                            .map(|c| {
                                (
                                    u16::from_le_bytes([c[0], c[1]]),
                                    i16::from_le_bytes([c[2], c[3]]),
                                )
                            })
                            .collect(),
                    )]
                }
                Opcode::BranchComputedGoto | Opcode::BranchComputedSubroutine => {
                    vec![Operand::NameList(
                        t.as_chunks::<2>()
                            .0
                            .iter()
                            .map(|&c| self.resolver.resolve(u16::from_le_bytes(c)))
                            .collect(),
                    )]
                }
                _ => vec![Operand::Bytes(t.clone())],
            },
            Operands::WordTrailer(w, t) => {
                vec![Operand::Column(*w), Operand::Comment(t.clone())]
            }
        }
    }

    fn word_operand(&self, op: Opcode, w: u16) -> Vec<Operand> {
        match op {
            Opcode::ConstInt16Decimal => vec![int16(w, Radix::Decimal)],
            Opcode::ConstInt16Hex => vec![int16(w, Radix::Hex)],
            Opcode::ConstInt16Octal => vec![int16(w, Radix::Octal)],
            Opcode::ReadDefaultIndexed
            | Opcode::WriteDefaultIndexed
            | Opcode::WriteRefDefaultIndexed
            | Opcode::EraseArrays
            | Opcode::FileClose
            | Opcode::IntrinsicLowerBound
            | Opcode::IntrinsicUpperBound => vec![Operand::Count(w)],
            _ => vec![Operand::Word(w)],
        }
    }

    fn resolve_type(&self, t: TypeRef) -> ResolvedType {
        match t {
            TypeRef::None => ResolvedType::None,
            TypeRef::Intrinsic(i) => ResolvedType::Intrinsic(i),
            TypeRef::Named(w) => ResolvedType::Named(self.resolver.resolve_symbol(w)),
            TypeRef::Qualified(slot) => match self.qualified(slot) {
                Some(q) => ResolvedType::Qualified(q),
                None => ResolvedType::Unknown(u32::from(slot)),
            },
            TypeRef::Unknown(u) => ResolvedType::Unknown(u),
        }
    }

    fn pointer_shift(&self) -> u16 {
        if self.cache.layout.bitness == Bitness::X64 {
            3
        } else {
            2
        }
    }

    /// The library-qualified type at `slot` of the type-reference list.
    fn qualified(&self, slot: u16) -> Option<QualifiedType> {
        let r = type_ref_record(self.type_refs, usize::from(slot)).ok()?;
        Some(QualifiedType {
            library: r.library,
            qualifier: self.resolver.resolve_symbol(r.library),
            name: self.resolver.resolve_symbol(r.name),
        })
    }

    fn record(&self, ins: &Instruction, r: RecRef) -> Operand {
        let layout = &self.cache.layout;
        match ins.opcode {
            Opcode::ProcHeader | Opcode::ProcHeaderAlt | Opcode::DeclareEvent => {
                match FunctionRecord::parse(self.indirect, r.0, layout) {
                    // Out of budget: report the record as unreadable rather
                    // than list a truncated argument chain as complete.
                    Ok(f) if f.first_arg.is_some() && self.arg_budget.get() == 0 => {
                        Operand::BadRecord(r.0)
                    }
                    Ok(f) => Operand::Procedure(Box::new(self.procedure(ins, f))),
                    Err(_) => Operand::BadRecord(r.0),
                }
            }
            Opcode::VarDecl => match VariableRecord::parse(self.indirect, r.0, layout) {
                Ok(v) => {
                    let ty = match variable_type(&v, self.indirect, self.objects, layout) {
                        Ok(t) => self.resolve_type(t),
                        Err(_) => ResolvedType::Unknown(v.type_pos),
                    };
                    Operand::Variable(VariableInfo {
                        name: self.resolver.resolve_symbol(v.name),
                        ty,
                        is_new: v.is_new(),
                    })
                }
                Err(_) => Operand::BadRecord(r.0),
            },
            Opcode::TypeDecl => match UdtRecord::parse(self.indirect, r.0) {
                Ok(u) => Operand::UserType {
                    name: self.resolver.resolve_symbol(u.name),
                    private: u.is_private(),
                },
                Err(_) => Operand::BadRecord(r.0),
            },
            _ => Operand::Record(r.0),
        }
    }

    fn procedure(&self, ins: &Instruction, f: FunctionRecord) -> ProcedureInfo {
        let layout = &self.cache.layout;
        let left = self.arg_budget.get();
        let chain = argument_chain(self.indirect, f.first_arg, layout, 4096.min(left));
        // A failed walk cost as much as a long one: charge it in full.
        let spent = chain.as_ref().map_or(4096.min(left), Vec::len);
        self.arg_budget.set(left - spent.min(left));
        let args = chain
            .unwrap_or_default()
            .iter()
            .map(|a| self.arg(a))
            .collect();
        let decl = f
            .is_declare()
            .then(|| DeclarationRecord::parse(self.declarations, f.decl_offset).ok())
            .flatten();
        let library = decl
            .as_ref()
            .map(|d| self.resolver.resolve_symbol(d.library));
        let entry_point = decl.and_then(|d| d.entry_point);
        let m = ins.modifier & 0x3F;
        ProcedureInfo {
            name: self.resolver.resolve_symbol(f.name),
            kind: f.kind(),
            is_function: m & 0x07 == 2 || m & 0x07 == 6,
            public: m & 0x04 != 0,
            is_static: f.is_static(),
            is_declare: f.is_declare(),
            library,
            entry_point,
            args,
            return_type: self.return_type(&f),
            record: f,
        }
    }

    fn return_type(&self, f: &FunctionRecord) -> ResolvedType {
        match f.return_type_ref(self.indirect, self.objects, &self.cache.layout) {
            Ok(t) => self.resolve_type(t),
            Err(_) => ResolvedType::Unknown(f.return_type),
        }
    }

    fn arg(&self, a: &ArgumentRecord) -> ArgInfo {
        ArgInfo {
            name: self.resolver.resolve_symbol(a.name),
            by_val: a.is_by_val(),
            optional: a.is_optional(),
            has_default: a.has_default(),
            param_array: a.is_param_array(),
            ty: match a.type_ref(self.indirect, self.objects, &self.cache.layout) {
                Ok(t) => self.resolve_type(t),
                Err(_) => ResolvedType::Unknown(a.type_raw),
            },
        }
    }
}

fn int16(w: u16, radix: Radix) -> Operand {
    let value = match radix {
        Radix::Decimal => i64::from(w as i16),
        _ => i64::from(w),
    };
    Operand::Int { value, radix }
}

fn int32(w: &[u16; 2], radix: Radix) -> Operand {
    let bits = u32::from(w[0]) | (u32::from(w[1]) << 16);
    let value = match radix {
        Radix::Decimal => i64::from(bits as i32),
        _ => i64::from(bits),
    };
    Operand::Int { value, radix }
}

// ---- text rendering -------------------------------------------------------

fn escape(bytes: &[u8], out: &mut String, double_quote: bool) {
    for &b in bytes {
        match b {
            b'"' if double_quote => out.push_str("\"\""),
            b'\\' => out.push_str("\\x5C"),
            0x20..=0x7E => out.push(char::from(b)),
            _ => {
                let _ = write!(out, "\\x{b:02X}");
            }
        }
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Name::Me => f.write_str("Me"),
            Name::None => f.write_str("-"),
            Name::User(s) => f.write_str(s),
            Name::Builtin(s) => f.write_str(s),
            Name::UnknownBuiltin(k) => write!(f, "?builtin({k:#04x})"),
            Name::OutOfRange(i) => write!(f, "?ident({i})"),
        }
    }
}

impl fmt::Display for ResolvedType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolvedType::None => f.write_str("?"),
            ResolvedType::Intrinsic(t) => match t.intrinsic.name() {
                Some(n) => f.write_str(n),
                None => write!(f, "?type({:?})", t.intrinsic),
            },
            ResolvedType::Named(n) => write!(f, "{n}"),
            ResolvedType::Qualified(q) => write!(f, "{}.{}", q.qualifier, q.name),
            ResolvedType::Unknown(u) => write!(f, "?type({u:#x})"),
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Name {
                name,
                suffix,
                bracketed,
            } => {
                if *bracketed {
                    write!(f, "[{name}]")
                } else {
                    write!(f, "{name}")?;
                    match (suffix, name.spelling()) {
                        (Some(c), Some(_)) => write!(f, "{c}"),
                        _ => Ok(()),
                    }
                }
            }
            Operand::Count(c) => write!(f, "argc={c}"),
            Operand::Word(w) => write!(f, "w={w}"),
            Operand::Column(c) => write!(f, "col={c}"),
            Operand::Continuation(v) => {
                f.write_str("cont[")?;
                for (i, (t, n)) in v.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{t}:{n}")?;
                }
                f.write_str("]")
            }
            Operand::Int { value, radix } => match radix {
                Radix::Decimal => write!(f, "{value}"),
                Radix::Hex => write!(f, "&H{value:X}"),
                Radix::Octal => write!(f, "&O{value:o}"),
            },
            Operand::Float32(v) => write!(f, "f32:{v:?}"),
            Operand::Float64(v) => write!(f, "f64:{v:?}"),
            Operand::Currency(v) => {
                let sign = if *v < 0 { "-" } else { "" };
                let a = v.unsigned_abs();
                write!(f, "cur:{sign}{}.{:04}", a / 10000, a % 10000)
            }
            Operand::Date(v) => write!(f, "date:{v:?}"),
            Operand::Text(t) => {
                let mut s = String::new();
                escape(t, &mut s, true);
                write!(f, "\"{s}\"")
            }
            Operand::Comment(t) => {
                let mut s = String::new();
                escape(t, &mut s, false);
                write!(f, "'{s}")
            }
            Operand::Bytes(b) => {
                f.write_str("bytes:")?;
                for x in b {
                    write!(f, "{x:02X}")?;
                }
                Ok(())
            }
            Operand::NameList(v) => {
                f.write_str("[")?;
                for (i, n) in v.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{n}")?;
                }
                f.write_str("]")
            }
            Operand::Object {
                offset,
                class,
                qualified,
            } => match (class, qualified) {
                (Some(c), _) => write!(f, "obj@{offset}({c})"),
                (None, Some(q)) => write!(f, "obj@{offset}({}.{})", q.qualifier, q.name),
                (None, None) => write!(f, "obj@{offset}(?)"),
            },
            Operand::Procedure(p) => write!(f, "proc{{{}}}", ProcText(p)),
            Operand::Variable(v) => {
                write!(f, "var{{{}", v.name)?;
                if !matches!(v.ty, ResolvedType::None) {
                    f.write_str(if v.is_new { " As New " } else { " As " })?;
                    write!(f, "{}", v.ty)?;
                }
                f.write_str("}")
            }
            Operand::UserType { name, private } => write!(
                f,
                "udt{{{} {name}}}",
                if *private { "Private" } else { "Public" }
            ),
            Operand::Record(r) => write!(f, "rec@{r:#x}"),
            Operand::BadRecord(r) => write!(f, "rec@{r:#x}(invalid)"),
            Operand::Context(a, b) => match b {
                Some(b) => write!(f, "ctx@{a:#x},{b:#x}"),
                None => write!(f, "ctx@{a:#x}"),
            },
            Operand::Words(w) => {
                let v: Vec<String> = w.iter().map(|x| format!("{x}")).collect();
                write!(f, "w=[{}]", v.join(","))
            }
            Operand::Interface(t) => write!(f, "interface({t})"),
        }
    }
}

struct ProcText<'a>(&'a ProcedureInfo);

impl fmt::Display for ProcText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = self.0;
        if p.public {
            f.write_str("Public ")?;
        } else {
            f.write_str("Private ")?;
        }
        if p.is_static {
            f.write_str("Static ")?;
        }
        if p.is_declare {
            f.write_str("Declare ")?;
        }
        let word = match p.kind {
            ProcKind::PropertyGet => "Property Get",
            ProcKind::PropertyLet => "Property Let",
            ProcKind::PropertySet => "Property Set",
            _ if p.is_function => "Function",
            _ => "Sub",
        };
        write!(f, "{word} {}", p.name)?;
        if let Some(lib) = &p.library {
            write!(f, " Lib \"{lib}\"")?;
            if let Some(e) = p
                .entry_point
                .as_ref()
                .filter(|e| p.name.spelling().map(str::as_bytes) != Some(e.as_slice()))
            {
                let mut s = String::new();
                escape(e, &mut s, true);
                write!(f, " Alias \"{s}\"")?;
            }
        }
        f.write_str("(")?;
        for (i, a) in p.args.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            if a.optional {
                f.write_str("Optional ")?;
            }
            if a.param_array {
                f.write_str("ParamArray ")?;
            }
            if a.by_val {
                f.write_str("ByVal ")?;
            }
            write!(f, "{}", a.name)?;
            if !matches!(a.ty, ResolvedType::None) {
                write!(f, " As {}", a.ty)?;
            }
        }
        f.write_str(")")?;
        if !matches!(p.return_type, ResolvedType::None) && p.record.has_return_type() {
            write!(f, " As {}", p.return_type)?;
        }
        Ok(())
    }
}

impl fmt::Display for Listed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "+{:04X} {}", self.offset, self.opcode.label())?;
        if self.modifier != 0 {
            write!(f, " mod={}", self.modifier)?;
        }
        for o in &self.operands {
            write!(f, " {o}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Disassembly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "; vba-pcode disassembly v1")?;
        writeln!(f, "; layout {}", self.layout_note)?;
        writeln!(f, "; lines {}", self.lines.len())?;
        for l in &self.lines {
            for i in &l.instructions {
                writeln!(f, "L{:04} {i}", l.index)?;
            }
            if let Some(e) = &l.error {
                writeln!(f, "L{:04} ERROR {e}", l.index)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operand_text() {
        assert_eq!(
            Operand::Text(b"a\"b\\\xE9".to_vec()).to_string(),
            "\"a\"\"b\\x5C\\xE9\""
        );
        assert_eq!(Operand::Comment(b"it's".to_vec()).to_string(), "'it's");
        assert_eq!(
            Operand::Int {
                value: -5,
                radix: Radix::Decimal
            }
            .to_string(),
            "-5"
        );
        assert_eq!(
            Operand::Int {
                value: 255,
                radix: Radix::Hex
            }
            .to_string(),
            "&HFF"
        );
        assert_eq!(Operand::Currency(-12345).to_string(), "cur:-1.2345");
        assert_eq!(
            Operand::Name {
                name: Name::User("x".into()),
                suffix: Some('$'),
                bracketed: false
            }
            .to_string(),
            "x$"
        );
        assert_eq!(
            Operand::Name {
                name: Name::UnknownBuiltin(0x56),
                suffix: Some('$'),
                bracketed: false
            }
            .to_string(),
            "?builtin(0x56)"
        );
        assert_eq!(int16(0xFFFF, Radix::Decimal).to_string(), "-1");
        assert_eq!(
            int32(&[0, 0x8000], Radix::Decimal).to_string(),
            "-2147483648"
        );
    }
}
