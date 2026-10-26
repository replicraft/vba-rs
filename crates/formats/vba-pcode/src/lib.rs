//! Decoder and disassembler for the compiled-code ("p-code") cache that
//! precedes the source text in the module streams of a VBA project.
//!
//! Facts come from the repository documents `docs/pcode-format.md`,
//! `docs/pcode-opcodes.md` and `docs/pcode.md`, the public MS-OVBA
//! specification, and direct observation of a test project. The crate takes
//! plain byte slices; reading the container is the caller's job.
//!
//! Only VBA 7.0 on 32-bit Windows is verified against real material. Every
//! other layout is implemented from the specification text and reported as
//! [`Verification::SpecUnverified`].

mod cache;
mod decode;
mod error;
mod layout;
mod lines;
mod module;
mod names;
mod opcode;
mod project_stream;
mod reader;
mod recordmap;
mod records;
mod sections;

pub use cache::{
    CodeAreaCheck, Coverage, ModuleCache, OpaqueKind, Region, RegionKind, TRAILER_SIZE,
};
pub use decode::{
    DecodeError, DecodedLine, Instruction, NameRef, ObjRef, Operands, RecRef, decode_line,
};
pub use error::{PcodeError, Result, UnsupportedLayout};
pub use layout::{Bitness, Layout, VERIFIED_VERSION, Verification, VersionFamily};
pub use lines::{
    CODE_ALIGNMENT, LINE_MARKER, LINE_RECORD_SIZE, LINE_TRAILER_SIZE, LineRecord, LineTable,
};
pub use module::{Span, TableLocations};
pub use names::{CONFIRMED_BUILTINS, ME_WORD, Name, Resolver, builtin_name, name_suffix};
pub use opcode::{Opcode, Shape};
pub use project_stream::{IdentTable, ProjectCache, SIGNATURE, parse_identifiers, version_word};
pub use recordmap::{RecordCoverage, RecordKind, RecordMap};
pub use records::{
    ArgumentRecord, DeclarationRecord, FunctionRecord, IntrinsicType, OBJECT_ENTRY_SIZE,
    ObjectEntry, ProcKind, TypeId, TypeRef, UdtRecord, VariableRecord, argument_chain,
    object_entry, type_descriptor, type_from_field, variable_type,
};
pub use sections::{
    MEMBER_RECORD_SIZE, MemberRecord, MemberTable, REFERENCE_LIST_END, REFERENCE_LIST_MAGIC,
    ReferenceList,
};
