//! User-defined type (`Type ... End Type`) layout and byte encoding.
//!
//! Rules applied (see docs/ffi.md for provenance and confidence):
//! each member is aligned to its natural alignment (capped at 8), nested types
//! align to their largest member, the total size is rounded up to the type's
//! alignment, padding bytes are zero, multi-byte values are little-endian,
//! fixed-length strings are inline ANSI bytes padded with spaces, and
//! `LongPtr` takes the width of the vba-profile target.

use crate::{Codepage, FfiError, Unmappable};
use vba_profile::{Bitness, Target};
use vba_value::VbaString;

/// The type of one UDT member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UdtFieldType {
    /// `Byte` (1 byte).
    Byte,
    /// `Boolean` (2 bytes).
    Boolean,
    /// `Integer` (2 bytes).
    Integer,
    /// `Long` (4 bytes).
    Long,
    /// `LongLong` (8 bytes, 64-bit targets only).
    LongLong,
    /// `LongPtr` (pointer width of the target).
    LongPtr,
    /// `Single` (4 bytes).
    Single,
    /// `Double` (8 bytes).
    Double,
    /// `Currency` (8 bytes, value scaled by 10000).
    Currency,
    /// `Date` (8 bytes).
    Date,
    /// `String * n` (n ANSI bytes, alignment 1).
    FixedString(usize),
    /// A nested user-defined type.
    Udt(UdtLayout),
    /// A fixed-size array member.
    Array {
        /// Element type.
        elem: Box<UdtFieldType>,
        /// Element count (at least 1).
        count: usize,
    },
}

/// The value of one UDT member.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    /// Integer-like members (`Currency` as its scaled integer).
    Int(i64),
    /// `Single`, `Double`, `Date`.
    Float(f64),
    /// `String * n`.
    Str(VbaString),
    /// A nested UDT.
    Struct(Vec<FieldValue>),
    /// A fixed array.
    Array(Vec<FieldValue>),
}

/// A computed UDT layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdtLayout {
    target: Target,
    fields: Vec<(String, UdtFieldType)>,
    offsets: Vec<usize>,
    size: usize,
    align: usize,
}

fn size_align(t: &UdtFieldType, target: Target) -> (usize, usize) {
    let ptr = usize::from(target.pointer_size());
    match t {
        UdtFieldType::Byte => (1, 1),
        UdtFieldType::Boolean | UdtFieldType::Integer => (2, 2),
        UdtFieldType::Long | UdtFieldType::Single => (4, 4),
        UdtFieldType::LongLong
        | UdtFieldType::Double
        | UdtFieldType::Currency
        | UdtFieldType::Date => (8, 8),
        UdtFieldType::LongPtr => (ptr, ptr),
        UdtFieldType::FixedString(n) => (*n, 1),
        UdtFieldType::Udt(l) => (l.size, l.align),
        UdtFieldType::Array { elem, count } => {
            let (s, a) = size_align(elem, target);
            (s * count, a)
        }
    }
}

fn check_type(t: &UdtFieldType, target: Target) -> Result<(), FfiError> {
    let bad = |m: &str| Err(FfiError::InvalidSignature(m.to_string()));
    match t {
        UdtFieldType::LongLong if target.bitness != Bitness::B64 => {
            bad("LongLong member requires a 64-bit target")
        }
        UdtFieldType::FixedString(0) => bad("fixed-length string member must have length >= 1"),
        UdtFieldType::Array { count: 0, .. } => bad("array member must have at least one element"),
        UdtFieldType::Udt(l) if l.target != target => {
            bad("nested UDT was laid out for another target")
        }
        UdtFieldType::Array { elem, .. } => check_type(elem, target),
        _ => Ok(()),
    }
}

impl UdtLayout {
    /// Computes the layout of `fields` for `target`.
    pub fn new(target: Target, fields: Vec<(String, UdtFieldType)>) -> Result<Self, FfiError> {
        if fields.is_empty() {
            return Err(FfiError::InvalidSignature("UDT has no members".into()));
        }
        let mut offsets = Vec::with_capacity(fields.len());
        let (mut cursor, mut align) = (0usize, 1usize);
        for (_, t) in &fields {
            check_type(t, target)?;
            let (s, a) = size_align(t, target);
            cursor = cursor.next_multiple_of(a);
            offsets.push(cursor);
            cursor += s;
            align = align.max(a);
        }
        Ok(UdtLayout {
            target,
            fields,
            offsets,
            size: cursor.next_multiple_of(align),
            align,
        })
    }

    /// Total size in bytes (`LenB`).
    pub fn size(&self) -> usize {
        self.size
    }

    /// Alignment in bytes.
    pub fn align(&self) -> usize {
        self.align
    }

    /// The target this layout was computed for.
    pub fn target(&self) -> Target {
        self.target
    }

    /// Byte offset of the member called `name` (case-insensitive).
    pub fn offset_of(&self, name: &str) -> Option<usize> {
        self.fields
            .iter()
            .position(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|i| self.offsets[i])
    }

    /// Encodes member values to bytes.
    pub fn encode(&self, values: &[FieldValue], cp: Codepage) -> Result<Vec<u8>, FfiError> {
        if values.len() != self.fields.len() {
            return Err(FfiError::InvalidSignature(format!(
                "UDT has {} members, {} values given",
                self.fields.len(),
                values.len()
            )));
        }
        let mut out = vec![0u8; self.size];
        for (i, v) in values.iter().enumerate() {
            let off = self.offsets[i];
            let t = &self.fields[i].1;
            let (s, _) = size_align(t, self.target);
            let bytes = encode_value(t, v, self.target, cp)?;
            out[off..off + s].copy_from_slice(&bytes);
        }
        Ok(out)
    }

    /// Decodes bytes (exactly [`size`](Self::size) long) to member values.
    pub fn decode(&self, bytes: &[u8], cp: Codepage) -> Result<Vec<FieldValue>, FfiError> {
        if bytes.len() != self.size {
            return Err(FfiError::InvalidSignature(format!(
                "UDT is {} bytes, buffer is {}",
                self.size,
                bytes.len()
            )));
        }
        self.fields
            .iter()
            .zip(&self.offsets)
            .map(|((_, t), off)| {
                let (s, _) = size_align(t, self.target);
                decode_value(t, &bytes[*off..off + s], self.target, cp)
            })
            .collect()
    }
}

fn mismatch(what: &str) -> FfiError {
    FfiError::InvalidSignature(format!("UDT member value mismatch: {what}"))
}

fn encode_value(
    t: &UdtFieldType,
    v: &FieldValue,
    target: Target,
    cp: Codepage,
) -> Result<Vec<u8>, FfiError> {
    let int = |lo: i64, hi: i64, width: usize| -> Result<Vec<u8>, FfiError> {
        match v {
            FieldValue::Int(n) if (lo..=hi).contains(n) => Ok(n.to_le_bytes()[..width].to_vec()),
            FieldValue::Int(_) => Err(mismatch("integer out of range")),
            _ => Err(mismatch("expected integer")),
        }
    };
    match t {
        UdtFieldType::Byte => int(0, 255, 1),
        UdtFieldType::Boolean | UdtFieldType::Integer => int(i16::MIN.into(), i16::MAX.into(), 2),
        UdtFieldType::Long => int(i32::MIN.into(), i32::MAX.into(), 4),
        UdtFieldType::LongLong | UdtFieldType::Currency => int(i64::MIN, i64::MAX, 8),
        UdtFieldType::LongPtr => {
            if target.bitness == Bitness::B64 {
                int(i64::MIN, i64::MAX, 8)
            } else {
                int(i32::MIN.into(), i32::MAX.into(), 4)
            }
        }
        UdtFieldType::Single => match v {
            FieldValue::Float(f) => Ok((*f as f32).to_le_bytes().to_vec()),
            _ => Err(mismatch("expected float")),
        },
        UdtFieldType::Double | UdtFieldType::Date => match v {
            FieldValue::Float(f) => Ok(f.to_le_bytes().to_vec()),
            _ => Err(mismatch("expected float")),
        },
        UdtFieldType::FixedString(n) => match v {
            FieldValue::Str(s) => {
                let mut b = cp.encode(s.as_units(), Unmappable::Reject, true)?.bytes;
                if b.len() > *n {
                    return Err(mismatch("fixed string too long"));
                }
                b.resize(*n, b' ');
                Ok(b)
            }
            _ => Err(mismatch("expected string")),
        },
        UdtFieldType::Udt(l) => match v {
            FieldValue::Struct(vs) => l.encode(vs, cp),
            _ => Err(mismatch("expected nested struct")),
        },
        UdtFieldType::Array { elem, count } => match v {
            FieldValue::Array(vs) if vs.len() == *count => {
                let mut out = Vec::new();
                for e in vs {
                    out.extend(encode_value(elem, e, target, cp)?);
                }
                Ok(out)
            }
            _ => Err(mismatch("array length")),
        },
    }
}

fn decode_value(
    t: &UdtFieldType,
    b: &[u8],
    target: Target,
    cp: Codepage,
) -> Result<FieldValue, FfiError> {
    let signed = |b: &[u8]| -> i64 {
        let mut buf = if b.last().is_some_and(|x| x & 0x80 != 0) {
            [0xFF; 8]
        } else {
            [0; 8]
        };
        buf[..b.len()].copy_from_slice(b);
        i64::from_le_bytes(buf)
    };
    Ok(match t {
        UdtFieldType::Byte => FieldValue::Int(i64::from(b[0])),
        UdtFieldType::Boolean
        | UdtFieldType::Integer
        | UdtFieldType::Long
        | UdtFieldType::LongLong
        | UdtFieldType::Currency
        | UdtFieldType::LongPtr => FieldValue::Int(signed(b)),
        UdtFieldType::Single => {
            FieldValue::Float(f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])))
        }
        UdtFieldType::Double | UdtFieldType::Date => {
            FieldValue::Float(f64::from_le_bytes(b.try_into().expect("8 bytes")))
        }
        UdtFieldType::FixedString(_) => FieldValue::Str(VbaString::from_units(cp.decode(b, true)?)),
        UdtFieldType::Udt(l) => FieldValue::Struct(l.decode(b, cp)?),
        UdtFieldType::Array { elem, count } => {
            let (s, _) = size_align(elem, target);
            let mut vs = Vec::with_capacity(*count);
            for i in 0..*count {
                vs.push(decode_value(elem, &b[i * s..(i + 1) * s], target, cp)?);
            }
            FieldValue::Array(vs)
        }
    })
}
