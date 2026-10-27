//! Records of the indirect and object tables (`pcode-format.md` section 6).
//!
//! All record offsets are relative to the first byte of the table contents
//! (after the 4-byte length). Layout constants follow the specification; the
//! x64 variants are unverified.

use crate::error::{PcodeError, Result};
use crate::layout::{Bitness, Layout, VersionFamily};
use crate::reader::{slice, u16_at, u32_at};

/// Size of one object-table entry.
pub const OBJECT_ENTRY_SIZE: usize = 10;

/// Intrinsic type ids (spec 6.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntrinsicType {
    /// 1
    Null,
    /// 2
    Integer,
    /// 3
    Long,
    /// 4
    Single,
    /// 5
    Double,
    /// 6
    Currency,
    /// 7
    Date,
    /// 8
    String,
    /// 9
    Object,
    /// 10
    Error,
    /// 11
    Boolean,
    /// 12
    Variant,
    /// 14
    Decimal,
    /// 17
    Byte,
    /// 20: `LongLong`; with the pointer flag 0x80 it is `LongPtr` on x64
    /// (`[obs]`: 8 of 8 `LongPtr` variables of x64 projects).
    LongLong,
    /// 29: a user-defined type (`[obs]`: the id stored for arrays and
    /// records of user-defined types; the type name is not in the field).
    UserDefined,
    /// An id outside the documented list.
    Unknown(u8),
}

impl IntrinsicType {
    /// Interprets a type id byte. Bit 0x80 (pointer) and the other flag bits
    /// of the top three bits are returned separately by [`TypeId::parse`].
    pub fn from_id(id: u8) -> Self {
        match id {
            1 => Self::Null,
            2 => Self::Integer,
            3 => Self::Long,
            4 => Self::Single,
            5 => Self::Double,
            6 => Self::Currency,
            7 => Self::Date,
            8 => Self::String,
            9 => Self::Object,
            10 => Self::Error,
            11 => Self::Boolean,
            12 => Self::Variant,
            14 => Self::Decimal,
            17 => Self::Byte,
            20 => Self::LongLong,
            29 => Self::UserDefined,
            other => Self::Unknown(other),
        }
    }

    /// The VBA spelling of the type, `None` for unknown ids.
    pub fn name(self) -> Option<&'static str> {
        Some(match self {
            Self::Null => "Null",
            Self::Integer => "Integer",
            Self::Long => "Long",
            Self::Single => "Single",
            Self::Double => "Double",
            Self::Currency => "Currency",
            Self::Date => "Date",
            Self::String => "String",
            Self::Object => "Object",
            Self::Error => "Error",
            Self::Boolean => "Boolean",
            Self::Variant => "Variant",
            Self::Decimal => "Decimal",
            Self::Byte => "Byte",
            Self::LongLong => "LongLong",
            Self::UserDefined | Self::Unknown(_) => return None,
        })
    }
}

/// A type id byte split into the type and the flag bits (mask 0xE0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeId {
    /// The type (low five bits).
    pub intrinsic: IntrinsicType,
    /// Flag bits (top three bits of the byte); 0x80 means pointer-sized.
    pub flags: u8,
}

impl TypeId {
    /// Splits a type id byte.
    pub fn parse(byte: u8) -> Self {
        Self {
            intrinsic: IntrinsicType::from_id(byte & 0x1F),
            flags: byte & 0xE0,
        }
    }
}

/// A type reference as stored in variable, argument and function records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRef {
    /// No type information (no `As` clause, or the field is not valid).
    None,
    /// An intrinsic type.
    Intrinsic(TypeId),
    /// A class or user type named through the object table; the value is the
    /// identifier reference word of the type name.
    Named(u16),
    /// An array type (`Dim a() As T`, `a(1 To 3) As T`): a descriptor whose
    /// kind byte is 0x1B; the element type is the type id at +6. The bounds
    /// are not in the descriptor (a declaration with bounds has them as
    /// expressions in front of its `VAR_DECL`).
    Array(TypeId),
    /// A library-qualified type (`Lib.Type`): the slot of its record in the
    /// type-reference list that follows the object table
    /// ([`type_ref_record`]).
    Qualified(u16),
    /// A fixed-length string (`String * n`): the length when the descriptor
    /// holds it, 0 when the length is an expression in front of the
    /// declaration.
    FixedString(u16),
    /// A reference whose layout is not resolved (kept raw).
    Unknown(u32),
}

/// One object-table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectEntry {
    /// Flags word at +0 (bit 0x02: names a list; poorly understood).
    pub flags: u16,
    /// The word at +2. `[obs]`: bit 0 is set exactly for library-qualified
    /// type names (`DAO.Database`), whose word at +6 is not an identifier
    /// reference.
    pub attrs: u16,
    /// Identifier reference at +6 (type or class name) when the entry is not
    /// qualified.
    pub name: u16,
}

impl ObjectEntry {
    /// Whether the entry names a library-qualified type (`[obs]`, see
    /// [`ObjectEntry::attrs`]); its name is not resolved by this crate.
    pub fn is_qualified(&self) -> bool {
        self.attrs & 1 != 0
    }
}

/// Size of a record of the type-reference list.
pub const TYPE_REF_SIZE: usize = 8;

/// `log2` of [`TYPE_REF_SIZE`]: a qualified object entry stores the byte
/// offset of its record in the type-reference list.
pub(crate) const TYPE_REF_SHIFT: u16 = 3;

/// A record of the type-reference list: the name and the library handle of a
/// library-qualified type. `[obs]` (see the format document).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeRefRecord {
    /// The word at +0 (kind of reference; 2, 8 and 0x18 are seen).
    pub kind: u16,
    /// The word at +2: a handle that is the same for all types of one
    /// library within a project (its meaning beyond that is not known).
    pub library: u16,
    /// Identifier reference of the type name (word at +4).
    pub name: u16,
}

/// Locates the type-reference records that follow the object table: a header
/// of 8 bytes (a word, the bytes `01 01` and the byte length of the records as
/// a dword), then the records. `None` when the header does not have that form
/// or the records do not fit before `limit`.
pub fn type_ref_area(
    bytes: &[u8],
    objects_end: usize,
    limit: usize,
) -> Option<std::ops::Range<usize>> {
    let h = bytes.get(objects_end..objects_end.checked_add(8)?)?;
    if h[2] != 1 || h[3] != 1 {
        return None;
    }
    let len = u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as usize;
    let start = objects_end + 8;
    let end = start.checked_add(len)?;
    (len.is_multiple_of(TYPE_REF_SIZE) && end <= limit.min(bytes.len())).then_some(start..end)
}

/// Reads record `slot` of the type-reference list `refs`.
///
/// # Errors
///
/// Bounds errors when the slot lies outside the list.
pub fn type_ref_record(refs: &[u8], slot: usize) -> Result<TypeRefRecord> {
    let at = slot.saturating_mul(TYPE_REF_SIZE);
    Ok(TypeRefRecord {
        kind: u16_at(refs, at, "type reference kind")?,
        library: u16_at(refs, at.wrapping_add(2), "type reference library")?,
        name: u16_at(refs, at.wrapping_add(4), "type reference name")?,
    })
}

/// Reads the object-table entry at byte offset `offset` of `objects`.
///
/// # Errors
///
/// Bounds errors when `offset + 8` exceeds the table.
pub fn object_entry(objects: &[u8], offset: usize) -> Result<ObjectEntry> {
    Ok(ObjectEntry {
        flags: u16_at(objects, offset, "object flags")?,
        attrs: u16_at(objects, offset + 2, "object attributes")?,
        name: u16_at(objects, offset + 6, "object name")?,
    })
}

/// Kind of a procedure as given by the flag bits of the function record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcKind {
    /// Flag 0x1000, a `Sub` or `Function` (see the header MOD for which).
    SubOrFunction,
    /// Flag 0x2000.
    PropertyGet,
    /// Flag 0x4000.
    PropertyLet,
    /// Flag 0x8000.
    PropertySet,
    /// None of the kind flags is set.
    Unknown,
}

/// A function (procedure, property or `Declare`) record, spec 6.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionRecord {
    /// Offset of the record in the indirect table.
    pub offset: u32,
    /// Flags word at +0.
    pub flags: u16,
    /// Identifier reference of the procedure name.
    pub name: u16,
    /// Offset of the first argument record, if any.
    pub first_arg: Option<u32>,
    /// The raw return type descriptor dword.
    pub return_type: u32,
    /// Offset into the declaration table of the external-library record.
    pub decl_offset: u16,
    /// The option byte.
    pub option: u8,
    /// Argument count.
    pub arg_count: u8,
    /// VBA6+ flag byte (bit 0x02 public, 0x04 friend, 0x20 PtrSafe).
    pub flags6: u8,
}

impl FunctionRecord {
    /// Reads the record at `offset` of the indirect table.
    ///
    /// # Errors
    ///
    /// Bounds errors when the record does not fit in the table.
    pub fn parse(indirect: &[u8], offset: u32, layout: &Layout) -> Result<Self> {
        let o = offset as usize;
        let base = fn_base(layout);
        // On x64 the three trailing fields sit 6 bytes further than the
        // pointer-widening shift of the leading ones.
        // `[obs]`: not so from version word 0xDF on.
        let late = base
            + if layout.bitness == Bitness::X64
                && layout.version < crate::layout::EXTENDED_HEADER_FROM
            {
                6
            } else {
                0
            };
        let first_arg = u32_at(indirect, o + 36 + base, "function first argument")?;
        Ok(Self {
            offset,
            flags: u16_at(indirect, o, "function flags")?,
            name: u16_at(indirect, o + 2, "function name")?,
            first_arg: (first_arg != 0 && first_arg != u32::MAX).then_some(first_arg),
            return_type: u32_at(indirect, o + 40 + base, "function return type")?,
            decl_offset: u16_at(indirect, o + 44 + base, "function declaration offset")?,
            option: slice(indirect, o + 54 + late, 1, "function option")?[0],
            arg_count: slice(indirect, o + 55 + late, 1, "function argument count")?[0],
            flags6: if layout.family == VersionFamily::Vba5 {
                0
            } else {
                slice(indirect, o + 57 + late, 1, "function flags6")?[0]
            },
        })
    }

    /// The declared return type (see [`type_from_field`]); `TypeRef::None`
    /// for a `Sub` and when no `As` clause was written.
    ///
    /// # Errors
    ///
    /// Bounds errors when a type descriptor lies outside the tables.
    pub fn return_type_ref(
        &self,
        indirect: &[u8],
        objects: &[u8],
        layout: &Layout,
    ) -> Result<TypeRef> {
        if !self.has_return_type() {
            return Ok(TypeRef::None);
        }
        type_from_field(self.return_type, indirect, objects, layout)
    }

    /// The kind flags.
    pub fn kind(&self) -> ProcKind {
        if self.flags & 0x1000 != 0 {
            ProcKind::SubOrFunction
        } else if self.flags & 0x2000 != 0 {
            ProcKind::PropertyGet
        } else if self.flags & 0x4000 != 0 {
            ProcKind::PropertyLet
        } else if self.flags & 0x8000 != 0 {
            ProcKind::PropertySet
        } else {
            ProcKind::Unknown
        }
    }

    /// Whether a declared return type is present (flag 0x0020).
    pub fn has_return_type(&self) -> bool {
        self.flags & 0x20 != 0
    }

    /// Whether the procedure is `Static` (flag 0x0080).
    pub fn is_static(&self) -> bool {
        self.flags & 0x80 != 0
    }

    /// Whether this is an external `Declare` (spec 6.3).
    pub fn is_declare(&self) -> bool {
        self.option & 0x90 == 0 && self.decl_offset != 0xFFFF
    }

    /// Public flag of VBA6+ layouts (bit 0x02 of the flag byte).
    pub fn is_public(&self) -> bool {
        self.flags6 & 0x02 != 0
    }

    /// Friend flag of VBA6+ layouts (bit 0x04).
    pub fn is_friend(&self) -> bool {
        self.flags6 & 0x04 != 0
    }

    /// PtrSafe flag of VBA6+ layouts (bit 0x20).
    pub fn is_ptr_safe(&self) -> bool {
        self.flags6 & 0x20 != 0
    }
}

fn fn_base(layout: &Layout) -> usize {
    match (layout.family, layout.bitness) {
        (VersionFamily::Vba5, _) => 0,
        (_, Bitness::X86) => 4,
        (_, Bitness::X64) => 4 + 16,
    }
}

/// A declaration-table record of an external `Declare`.
///
/// `[obs]` (extends spec 6.9): a record has a 20-byte header whose word at +2
/// is the identifier reference of the library name and whose word at +4 is
/// the offset, relative to the start of the declaration table contents, of a
/// NUL-terminated ASCII text: the name of the entry point (the `Alias` text,
/// or the procedure name when none was written). Records are variable in
/// size (header plus text, padded to a multiple of 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationRecord {
    /// Identifier reference of the library name (at +2).
    pub library: u16,
    /// The entry-point text, when it lies inside the table and is terminated.
    pub entry_point: Option<Vec<u8>>,
}

impl DeclarationRecord {
    /// Reads the record at `offset` of the declaration table.
    ///
    /// # Errors
    ///
    /// Bounds errors when the header lies outside the table.
    pub fn parse(decl: &[u8], offset: u16) -> Result<Self> {
        let o = usize::from(offset);
        let library = u16_at(decl, o + 2, "declaration library")?;
        let text_at = usize::from(u16_at(decl, o + 4, "declaration entry offset")?);
        let entry_point = decl
            .get(text_at..)
            .and_then(|b| b.iter().position(|&c| c == 0).map(|n| b[..n].to_vec()));
        Ok(Self {
            library,
            entry_point,
        })
    }
}

/// A variable record, spec 6.4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableRecord {
    /// Offset of the record in the indirect table.
    pub offset: u32,
    /// Flags 1 (0x20: has an `As` clause).
    pub flags1: u8,
    /// Flags 2 (0x20: declared `New`).
    pub flags2: u8,
    /// Identifier reference of the variable name.
    pub name: u16,
    /// Position of the type information in the indirect table (valid when
    /// [`VariableRecord::has_as_clause`]).
    pub type_pos: u32,
}

impl VariableRecord {
    /// Reads the record at `offset`.
    ///
    /// # Errors
    ///
    /// Bounds errors.
    pub fn parse(indirect: &[u8], offset: u32, layout: &Layout) -> Result<Self> {
        let o = offset as usize;
        let tp = if layout.bitness == Bitness::X64 {
            16
        } else {
            12
        };
        Ok(Self {
            offset,
            flags1: slice(indirect, o, 1, "variable flags1")?[0],
            flags2: slice(indirect, o + 1, 1, "variable flags2")?[0],
            name: u16_at(indirect, o + 2, "variable name")?,
            type_pos: u32_at(indirect, o + tp, "variable type position")?,
        })
    }

    /// Whether an `As` clause was written.
    pub fn has_as_clause(&self) -> bool {
        self.flags1 & 0x20 != 0
    }

    /// Whether the variable was declared `New`.
    pub fn is_new(&self) -> bool {
        self.flags2 & 0x20 != 0
    }
}

/// A user-defined-type record, spec 6.5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdtRecord {
    /// Offset of the record in the indirect table.
    pub offset: u32,
    /// Identifier reference of the type name.
    pub name: u16,
    /// Option word (bit 0 clear means `Private`).
    pub option: u16,
}

impl UdtRecord {
    /// Reads the record at `offset`.
    ///
    /// # Errors
    ///
    /// Bounds errors.
    pub fn parse(indirect: &[u8], offset: u32) -> Result<Self> {
        let o = offset as usize;
        Ok(Self {
            offset,
            name: u16_at(indirect, o + 2, "udt name")?,
            option: u16_at(indirect, o + 18, "udt option")?,
        })
    }

    /// Whether the type is `Private`.
    pub fn is_private(&self) -> bool {
        self.option & 1 == 0
    }
}

/// An argument record, spec 6.6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentRecord {
    /// Offset of the record in the indirect table.
    pub offset: u32,
    /// Flags (0x20 has `As`; 0x40 is set on most arguments and its meaning
    /// is not determined).
    pub flags: u16,
    /// Identifier reference of the argument name.
    pub name: u16,
    /// The raw type dword.
    pub type_raw: u32,
    /// Offset of the next argument record, if any.
    pub next: Option<u32>,
    /// Option bits (0x1 ParamArray, 0x2 ByRef, 0x4 ByVal, 0x200 Optional).
    pub option: u16,
}

impl ArgumentRecord {
    /// Reads the record at `offset`.
    ///
    /// # Errors
    ///
    /// Bounds errors (a record must fit entirely: offset + 26).
    pub fn parse(indirect: &[u8], offset: u32, layout: &Layout) -> Result<Self> {
        let o = offset as usize;
        let a = if layout.bitness == Bitness::X64 { 4 } else { 0 };
        let next = u32_at(indirect, o + 20 + a, "argument next")?;
        Ok(Self {
            offset,
            flags: u16_at(indirect, o, "argument flags")?,
            name: u16_at(indirect, o + 2, "argument name")?,
            type_raw: u32_at(indirect, o + 12 + a, "argument type")?,
            next: (next != 0 && next != u32::MAX).then_some(next),
            option: u16_at(indirect, o + 24 + a, "argument option")?,
        })
    }

    /// Whether an `As` type was written.
    pub fn has_as_clause(&self) -> bool {
        self.flags & 0x20 != 0
    }

    /// Whether the argument is `ByVal`.
    pub fn is_by_val(&self) -> bool {
        self.option & 0x4 != 0
    }

    /// Whether the argument is `ByRef` (explicit).
    pub fn is_by_ref(&self) -> bool {
        self.option & 0x2 != 0
    }

    /// Whether the argument is a `ParamArray` (option bit 0x1, observed).
    pub fn is_param_array(&self) -> bool {
        self.option & 0x1 != 0
    }

    /// Whether the argument is `Optional`.
    pub fn is_optional(&self) -> bool {
        self.option & 0x200 != 0
    }

    /// Whether an `Optional` argument was declared with a default value
    /// (option bit 0x400, observed: it is set exactly when the header line
    /// carries a default-value expression for the argument).
    pub fn has_default(&self) -> bool {
        self.option & 0x400 != 0
    }

    /// The argument type (see [`type_from_field`]); `TypeRef::None` when no
    /// `As` clause was written.
    ///
    /// # Errors
    ///
    /// Bounds errors when a type descriptor lies outside the tables.
    pub fn type_ref(&self, indirect: &[u8], objects: &[u8], layout: &Layout) -> Result<TypeRef> {
        if !self.has_as_clause() {
            return Ok(TypeRef::None);
        }
        type_from_field(self.type_raw, indirect, objects, layout)
    }
}

/// Walks the argument chain starting at `first`, bounded by `max` records to
/// guard against cycles in hostile input.
///
/// # Errors
///
/// Bounds errors from [`ArgumentRecord::parse`]; [`PcodeError::OutOfBounds`]
/// when the chain is longer than `max`.
pub fn argument_chain(
    indirect: &[u8],
    first: Option<u32>,
    layout: &Layout,
    max: usize,
) -> Result<Vec<ArgumentRecord>> {
    let mut out = Vec::new();
    let mut next = first;
    while let Some(off) = next {
        if out.len() >= max {
            return Err(PcodeError::OutOfBounds {
                what: "argument chain length",
                offset: u64::from(off),
                size: 0,
                limit: max as u64,
            });
        }
        let r = ArgumentRecord::parse(indirect, off, layout)?;
        next = r.next;
        out.push(r);
    }
    Ok(out)
}

/// Resolves a type field as stored in variable, argument and function
/// records.
///
/// `[obs]` (corrects the position rule of spec 6.4): the field is a dword.
/// `0xFFFFFFFF` means no type; when its upper 16 bits are `0xFFFF` the low
/// byte is an intrinsic type id (the spec reads this as "the word after the
/// type position"); otherwise the dword is an offset of a type descriptor in
/// the indirect table.
///
/// # Errors
///
/// Bounds errors when the descriptor lies outside the tables.
pub fn type_from_field(
    raw: u32,
    indirect: &[u8],
    objects: &[u8],
    layout: &Layout,
) -> Result<TypeRef> {
    if raw == u32::MAX {
        Ok(TypeRef::None)
    } else if raw >> 16 == 0xFFFF {
        Ok(TypeRef::Intrinsic(TypeId::parse(raw as u8)))
    } else {
        type_descriptor(indirect, objects, raw, layout)
    }
}

/// Resolves the type of a variable record (spec 6.4, 6.7); see
/// [`type_from_field`].
///
/// # Errors
///
/// Bounds errors when the referenced bytes lie outside the tables.
pub fn variable_type(
    rec: &VariableRecord,
    indirect: &[u8],
    objects: &[u8],
    layout: &Layout,
) -> Result<TypeRef> {
    if !rec.has_as_clause() {
        return Ok(TypeRef::None);
    }
    type_from_field(rec.type_pos, indirect, objects, layout)
}

/// Resolves a type descriptor stored at `offset` in the indirect table
/// (spec 6.7).
///
/// # Errors
///
/// Bounds errors.
pub fn type_descriptor(
    indirect: &[u8],
    objects: &[u8],
    offset: u32,
    layout: &Layout,
) -> Result<TypeRef> {
    // `[obs]`: descriptors are 2-aligned; bit 0 of a stored position was
    // set once (a `String * n` variable).
    let o = (offset & !1) as usize;
    let flags = u16_at(indirect, o, "type descriptor flags")?;
    if flags & 0x20 != 0 && flags & 0x1F == 0 {
        // `[obs]`: flag 0x20 marks a fixed-length string; the word at +2 is
        // the length when it is a literal of three or more digits, else 0.
        return Ok(TypeRef::FixedString(u16_at(
            indirect,
            o + 2,
            "fixed string length",
        )?));
    }
    if flags & 0x1F == 0x1B {
        // `[obs]`: kind byte 0x1B is an array; 0x1D is an object type. The
        // element type id is the byte at +6, as for a simple type.
        let id = slice(indirect, o + 6, 1, "array descriptor element")?[0];
        return Ok(TypeRef::Array(TypeId::parse(id)));
    }
    if flags & 0x02 != 0 {
        let id = slice(indirect, o + 6, 1, "type descriptor id")?[0];
        return Ok(TypeRef::Intrinsic(TypeId::parse(id)));
    }
    // `[obs]`: the word at +2 is the index of the object-table entry times the
    // pointer size of the producing build (4 on x86, 8 on x64); index 0 is a
    // valid entry.
    let v = u16_at(indirect, o + 2, "type descriptor object")?;
    let shift = if layout.bitness == Bitness::X64 { 3 } else { 2 };
    let idx = usize::from(v >> shift);
    let e = object_entry(objects, idx * OBJECT_ENTRY_SIZE)?;
    if e.is_qualified() {
        // `[obs]`: the word at +6 is the byte offset of the type-reference
        // record in the list (records of 8 bytes) on x86 and on x64.
        return Ok(if e.name == 0xFFFF {
            TypeRef::Unknown(offset)
        } else {
            TypeRef::Qualified(e.name >> TYPE_REF_SHIFT)
        });
    }
    if e.name == 0 || e.name == 0xFFFF {
        return Ok(TypeRef::Unknown(offset));
    }
    Ok(TypeRef::Named(e.name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l() -> Layout {
        Layout::select(0x97, 1).unwrap()
    }

    #[test]
    fn function_record_fields() {
        let mut t = vec![0u8; 70];
        t[0..2].copy_from_slice(&0x1020u16.to_le_bytes());
        t[2..4].copy_from_slice(&0x0210u16.to_le_bytes());
        t[40..44].copy_from_slice(&0x30u32.to_le_bytes());
        t[44..48].copy_from_slice(&7u32.to_le_bytes());
        t[48..50].copy_from_slice(&0xFFFFu16.to_le_bytes());
        t[58] = 0x00;
        t[59] = 2;
        t[61] = 0x02;
        let f = FunctionRecord::parse(&t, 0, &l()).unwrap();
        assert_eq!(f.kind(), ProcKind::SubOrFunction);
        assert!(f.has_return_type());
        assert_eq!(f.name, 0x210);
        assert_eq!(f.first_arg, Some(0x30));
        assert_eq!(f.return_type, 7);
        assert!(!f.is_declare());
        assert_eq!(f.arg_count, 2);
        assert!(f.is_public());
        assert!(FunctionRecord::parse(&t[..60], 0, &l()).is_err());
        assert!(FunctionRecord::parse(&t, u32::MAX, &l()).is_err());
    }

    #[test]
    fn argument_chain_terminates() {
        let mut t = vec![0u8; 80];
        // record at 0 -> 40 -> end
        t[20..24].copy_from_slice(&40u32.to_le_bytes());
        let c = argument_chain(&t, Some(0), &l(), 8).unwrap();
        assert_eq!(c.len(), 2);
        // out of range
        assert!(argument_chain(&[0; 4], Some(0), &l(), 8).is_err());
        // cycle is cut by the length bound
        let mut cyc = vec![0u8; 40];
        cyc[20..24].copy_from_slice(&0x10u32.to_le_bytes());
        cyc[0x10 + 20..0x10 + 24].copy_from_slice(&0x10u32.to_le_bytes());
        assert!(argument_chain(&cyc, Some(0), &l(), 8).is_err());
    }

    #[test]
    fn x64_record_offsets() {
        let l64 = Layout::select(0xB5, 3).unwrap();
        // Function record: first argument at +56, return type at +60,
        // declaration offset at +64, option, count and flags at +80.
        let mut f = vec![0u8; 100];
        f[0..2].copy_from_slice(&0x1020u16.to_le_bytes());
        f[56..60].copy_from_slice(&0x40u32.to_le_bytes());
        f[60..64].copy_from_slice(&0xFFFF_0008u32.to_le_bytes());
        f[64..66].copy_from_slice(&0xFFFFu16.to_le_bytes());
        f[80] = 0x8C;
        f[81] = 3;
        f[83] = 0x02;
        let r = FunctionRecord::parse(&f, 0, &l64).unwrap();
        assert_eq!(r.first_arg, Some(0x40));
        assert_eq!(r.return_type, 0xFFFF_0008);
        assert_eq!((r.option, r.arg_count, r.flags6), (0x8C, 3, 0x02));
        assert!(!r.is_declare());
        // Argument record: type at +16, next at +24, options at +28.
        let mut a = vec![0u8; 40];
        a[16..20].copy_from_slice(&0xFFFF_0003u32.to_le_bytes());
        a[24..28].copy_from_slice(&0x20u32.to_le_bytes());
        a[28..30].copy_from_slice(&0x0004u16.to_le_bytes());
        let r = ArgumentRecord::parse(&a, 0, &l64).unwrap();
        assert_eq!(r.type_raw, 0xFFFF_0003);
        assert_eq!(r.next, Some(0x20));
        assert!(r.is_by_val());
    }

    #[test]
    fn descriptor_index_is_scaled_by_pointer_size() {
        let mut ind = vec![0u8; 16];
        ind[2..4].copy_from_slice(&16u16.to_le_bytes());
        let mut obj = vec![0u8; 40];
        obj[26..28].copy_from_slice(&0x0123u16.to_le_bytes()); // entry 2
        obj[6..8].copy_from_slice(&0x0456u16.to_le_bytes()); // entry 0
        // x64: 16 / 8 = entry 2.
        assert_eq!(
            type_descriptor(&ind, &obj, 0, &Layout::select(0xB5, 3).unwrap()).unwrap(),
            TypeRef::Named(0x0123)
        );
        // x86: 16 / 4 = entry 4, outside the 4 entries of the table.
        assert!(type_descriptor(&ind, &obj, 0, &l()).is_err());
        // Index 0 is the first entry, not "no type".
        ind[2..4].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(
            type_descriptor(&ind, &obj, 0, &l()).unwrap(),
            TypeRef::Named(0x0456)
        );
    }

    #[test]
    fn array_descriptors_are_not_simple_types() {
        // Kind byte 0x1B (flags 0x081B for `a() As Variant`): the element
        // type is at +6; it must not be read as the type of the variable.
        let mut ind = vec![0u8; 16];
        ind[0..2].copy_from_slice(&0x081Bu16.to_le_bytes());
        ind[6] = 12;
        let l64 = Layout::select(0xB5, 3).unwrap();
        assert_eq!(
            type_descriptor(&ind, &[0u8; 20], 0, &l64).unwrap(),
            TypeRef::Array(TypeId::parse(12))
        );
        // A simple descriptor (flag 0x02, kind not 0x1B) still is simple.
        ind[0..2].copy_from_slice(&0x0002u16.to_le_bytes());
        assert_eq!(
            type_descriptor(&ind, &[0u8; 20], 0, &l64).unwrap(),
            TypeRef::Intrinsic(TypeId::parse(12))
        );
    }

    #[test]
    fn qualified_descriptor_names_a_type_reference_slot() {
        // Descriptor -> entry 1 (x64: 8 / 8), a qualified entry whose word
        // at +6 is the byte offset of slot 3.
        let mut ind = vec![0u8; 16];
        ind[2..4].copy_from_slice(&8u16.to_le_bytes());
        let mut obj = vec![0u8; 20];
        obj[12..14].copy_from_slice(&0x3C01u16.to_le_bytes());
        obj[16..18].copy_from_slice(&24u16.to_le_bytes());
        let l64 = Layout::select(0xB5, 3).unwrap();
        assert_eq!(
            type_descriptor(&ind, &obj, 0, &l64).unwrap(),
            TypeRef::Qualified(3)
        );
        // A slot word of 0xFFFF names nothing.
        obj[16..18].copy_from_slice(&0xFFFFu16.to_le_bytes());
        assert_eq!(
            type_descriptor(&ind, &obj, 0, &l64).unwrap(),
            TypeRef::Unknown(0)
        );
    }

    #[test]
    fn type_reference_list_is_located_and_read() {
        // A word, `01 01`, the byte length, then two records.
        let mut b = vec![0u8; 4];
        b.extend_from_slice(&[0, 0, 1, 1]);
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&[2, 0, 0x1E, 0, 0x34, 0x02, 0, 0]);
        b.extend_from_slice(&[8, 0, 0x40, 0, 0x28, 0x03, 0, 0]);
        let area = type_ref_area(&b, 4, b.len()).unwrap();
        assert_eq!(area, 12..28);
        let refs = &b[area];
        assert_eq!(
            type_ref_record(refs, 1).unwrap(),
            TypeRefRecord {
                kind: 8,
                library: 0x40,
                name: 0x328
            }
        );
        assert!(type_ref_record(refs, 2).is_err());
        // Wrong marker, ragged length, or a list that does not fit.
        b[6] = 0;
        assert!(type_ref_area(&b, 4, b.len()).is_none());
        b[6] = 1;
        assert!(type_ref_area(&b, 4, b.len() - 1).is_none());
        b[8] = 15;
        assert!(type_ref_area(&b, 4, b.len()).is_none());
    }

    #[test]
    fn type_ids() {
        let t = TypeId::parse(0x88);
        assert_eq!(t.intrinsic, IntrinsicType::String);
        assert_eq!(t.flags, 0x80);
        assert_eq!(IntrinsicType::from_id(13), IntrinsicType::Unknown(13));
        // `LongPtr` on x64: LongLong with the pointer flag.
        let p = TypeId::parse(0x94);
        assert_eq!(p.intrinsic, IntrinsicType::LongLong);
        assert_eq!(p.flags, 0x80);
        assert_eq!(IntrinsicType::Boolean.name(), Some("Boolean"));
    }

    #[test]
    fn descriptor_resolution() {
        // indirect: descriptor at 0 simple type 3 (Long)
        let mut ind = vec![0u8; 16];
        ind[0..2].copy_from_slice(&2u16.to_le_bytes());
        ind[6] = 3;
        let obj = vec![0u8; 20];
        assert_eq!(
            type_descriptor(&ind, &obj, 0, &l()).unwrap(),
            TypeRef::Intrinsic(TypeId::parse(3))
        );
        // named: flags 0, value 1<<2 -> entry 1 (offset 10), name at +6
        let mut ind2 = vec![0u8; 16];
        ind2[2..4].copy_from_slice(&4u16.to_le_bytes());
        let mut obj2 = vec![0u8; 20];
        obj2[16..18].copy_from_slice(&0x0123u16.to_le_bytes());
        assert_eq!(
            type_descriptor(&ind2, &obj2, 0, &l()).unwrap(),
            TypeRef::Named(0x0123)
        );
        assert!(type_descriptor(&ind2, &obj2[..12], 0, &l()).is_err());
    }
}
