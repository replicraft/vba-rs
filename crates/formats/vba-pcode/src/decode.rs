//! Instruction decoding of one line of code (`pcode-opcodes.md` sections 1
//! and 5).

use crate::layout::{Bitness, Layout};
use crate::opcode::{Opcode, Shape};

/// An identifier reference word as stored (spec 5.1). Resolution to a
/// spelling needs the project identifier table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NameRef(pub u16);

/// An offset into the object table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjRef(pub u16);

/// An offset into the indirect table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecRef(pub u32);

/// Decoded operands, one variant per [`Shape`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operands {
    /// No operands.
    None,
    /// One identifier reference.
    Name(NameRef),
    /// Identifier reference and argument count.
    NameArgs(NameRef, u16),
    /// One plain word.
    Word(u16),
    /// Two plain words.
    Words2([u16; 2]),
    /// Four plain words.
    Words4([u16; 4]),
    /// An object-table offset.
    Obj(ObjRef),
    /// An indirect-table offset.
    Rec(RecRef),
    /// Indirect-table offset with the optional extra word of `VAR_DECL`.
    VarDecl(RecRef, Option<u16>),
    /// Procedure context dword(s).
    Ctx(u32, Option<u32>),
    /// Identifier reference, count and indirect-table offset.
    NameArgsRec(NameRef, u16, RecRef),
    /// Variable-length trailer bytes.
    Trailer(Vec<u8>),
    /// A word followed by trailer bytes.
    WordTrailer(u16, Vec<u8>),
}

/// One decoded instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// Byte offset of the instruction word relative to the start of the line.
    pub offset: usize,
    /// The canonical opcode.
    pub opcode: Opcode,
    /// The 6-bit modifier field.
    pub modifier: u8,
    /// The operands.
    pub operands: Operands,
}

/// Why a line stopped decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The stored opcode number is not valid for the layout.
    #[error("unknown opcode {stored} at line offset {offset}")]
    UnknownOpcode {
        /// Stored (not canonical) number.
        stored: u16,
        /// Line offset of the instruction.
        offset: usize,
    },
    /// An instruction's operands extend past the end of the line.
    #[error("instruction at line offset {offset} runs past the end of the line")]
    Truncated {
        /// Line offset of the instruction.
        offset: usize,
    },
    /// The line has an odd number of bytes left (instructions are words).
    #[error("dangling byte at line offset {offset}")]
    DanglingByte {
        /// Line offset of the byte.
        offset: usize,
    },
}

/// The result of decoding one line: the instructions read and, when the
/// stream could not be followed to its end, why it stopped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecodedLine {
    /// Instructions decoded before any error.
    pub instructions: Vec<Instruction>,
    /// The error that stopped decoding, if any.
    pub error: Option<DecodeError>,
}

impl DecodedLine {
    /// Whether the whole line decoded.
    pub fn is_complete(&self) -> bool {
        self.error.is_none()
    }
}

struct Words<'a> {
    data: &'a [u8],
    pos: usize,
    start: usize,
}

impl Words<'_> {
    fn word(&mut self) -> Result<u16, DecodeError> {
        let b = self
            .data
            .get(self.pos..self.pos + 2)
            .ok_or(DecodeError::Truncated { offset: self.start })?;
        self.pos += 2;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn dword(&mut self) -> Result<u32, DecodeError> {
        let lo = u32::from(self.word()?);
        let hi = u32::from(self.word()?);
        Ok(lo | (hi << 16))
    }

    fn trailer(&mut self) -> Result<Vec<u8>, DecodeError> {
        let n = usize::from(self.word()?);
        let b = self
            .data
            .get(self.pos..self.pos + n)
            .ok_or(DecodeError::Truncated { offset: self.start })?
            .to_vec();
        self.pos += n;
        if n % 2 == 1 {
            // Pad byte; a missing pad at the very end of the line is tolerated
            // only when nothing else follows.
            if self.pos < self.data.len() {
                self.pos += 1;
            } else {
                return Err(DecodeError::Truncated { offset: self.start });
            }
        }
        Ok(b)
    }
}

/// Decodes the instructions of one line.
///
/// `line` holds exactly the line's code bytes (its stated length).
pub fn decode_line(line: &[u8], layout: &Layout) -> DecodedLine {
    let mut out = DecodedLine::default();
    let mut w = Words {
        data: line,
        pos: 0,
        start: 0,
    };
    while w.pos < line.len() {
        w.start = w.pos;
        if w.pos + 2 > line.len() {
            out.error = Some(DecodeError::DanglingByte { offset: w.pos });
            break;
        }
        match decode_one(&mut w, layout) {
            Ok(i) => out.instructions.push(i),
            Err(e) => {
                out.error = Some(e);
                break;
            }
        }
    }
    out
}

fn decode_one(w: &mut Words<'_>, layout: &Layout) -> Result<Instruction, DecodeError> {
    let offset = w.pos;
    let word = w.word()?;
    let stored = word & 0x3FF;
    let modifier = (word >> 10) as u8;
    let opcode = layout
        .canonical_opcode(stored)
        .and_then(Opcode::from_canonical)
        .ok_or(DecodeError::UnknownOpcode { stored, offset })?;
    let operands = match opcode.shape() {
        Shape::None => Operands::None,
        Shape::Name => Operands::Name(NameRef(w.word()?)),
        Shape::NameW => {
            let n = NameRef(w.word()?);
            Operands::NameArgs(n, w.word()?)
        }
        Shape::W => Operands::Word(w.word()?),
        Shape::WW => Operands::Words2([w.word()?, w.word()?]),
        Shape::WWWW => Operands::Words4([w.word()?, w.word()?, w.word()?, w.word()?]),
        Shape::Obj => Operands::Obj(ObjRef(w.word()?)),
        Shape::Rec => Operands::Rec(RecRef(w.dword()?)),
        Shape::VarDecl => {
            let r = RecRef(w.dword()?);
            let extra = if modifier & 0x10 != 0 {
                Some(w.word()?)
            } else {
                None
            };
            Operands::VarDecl(r, extra)
        }
        Shape::Ctx => {
            let a = w.dword()?;
            let b = if layout.bitness == Bitness::X64 {
                Some(w.dword()?)
            } else {
                None
            };
            Operands::Ctx(a, b)
        }
        Shape::NameWRec => {
            let n = NameRef(w.word()?);
            let c = w.word()?;
            Operands::NameArgsRec(n, c, RecRef(w.dword()?))
        }
        Shape::V => Operands::Trailer(w.trailer()?),
        Shape::WV => {
            let a = w.word()?;
            Operands::WordTrailer(a, w.trailer()?)
        }
    };
    Ok(Instruction {
        offset,
        opcode,
        modifier,
        operands,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l() -> Layout {
        Layout::select(0x97, 1).unwrap()
    }

    fn word(stored: u16, modifier: u16) -> [u8; 2] {
        (stored | (modifier << 10)).to_le_bytes()
    }

    #[test]
    fn decodes_simple_sequence() {
        // READ_SIMPLE name, CONST_INT16_DECIMAL 5, NUM_ADD.
        let mut b = Vec::new();
        b.extend_from_slice(&word(32, 0));
        b.extend_from_slice(&0x0204u16.to_le_bytes());
        b.extend_from_slice(&word(172, 0));
        b.extend_from_slice(&5u16.to_le_bytes());
        b.extend_from_slice(&word(11, 0));
        let d = decode_line(&b, &l());
        assert!(d.is_complete());
        assert_eq!(d.instructions.len(), 3);
        assert_eq!(d.instructions[0].opcode, Opcode::ReadSimple);
        assert_eq!(d.instructions[0].operands, Operands::Name(NameRef(0x204)));
        assert_eq!(d.instructions[1].operands, Operands::Word(5));
        assert_eq!(d.instructions[2].offset, 8);
    }

    #[test]
    fn x86_numbering_is_translated() {
        // Stored 205 is canonical 208 (OPTION_STMT), MOD 3.
        let b = word(205, 3);
        let d = decode_line(&b, &l());
        assert_eq!(d.instructions[0].opcode, Opcode::OptionStmt);
        assert_eq!(d.instructions[0].modifier, 3);
    }

    #[test]
    fn trailers_and_padding() {
        // COMMENT_QUOTE (227 canonical = 224 stored): W, length 3, bytes, pad.
        let mut b = Vec::new();
        b.extend_from_slice(&word(224, 0));
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&3u16.to_le_bytes());
        b.extend_from_slice(b"abc");
        b.push(0);
        b.extend_from_slice(&word(11, 0));
        let d = decode_line(&b, &l());
        assert!(d.is_complete(), "{:?}", d.error);
        assert_eq!(
            d.instructions[0].operands,
            Operands::WordTrailer(0, b"abc".to_vec())
        );
        assert_eq!(d.instructions[1].offset, 10);
    }

    #[test]
    fn var_decl_extra_word_by_modifier() {
        // VAR_DECL canonical 245 = stored 242.
        let mut b = Vec::new();
        b.extend_from_slice(&word(242, 0x10));
        b.extend_from_slice(&0x1234_5678u32.to_le_bytes());
        b.extend_from_slice(&7u16.to_le_bytes());
        let d = decode_line(&b, &l());
        assert!(d.is_complete());
        assert_eq!(
            d.instructions[0].operands,
            Operands::VarDecl(RecRef(0x1234_5678), Some(7))
        );
    }

    #[test]
    fn errors_are_reported_not_panicked() {
        assert_eq!(
            decode_line(&word(300, 0), &l()).error,
            Some(DecodeError::UnknownOpcode {
                stored: 300,
                offset: 0
            })
        );
        assert!(matches!(
            decode_line(&word(32, 0), &l()).error,
            Some(DecodeError::Truncated { .. })
        ));
        assert!(matches!(
            decode_line(&[1], &l()).error,
            Some(DecodeError::DanglingByte { .. })
        ));
        let mut b = word(224, 0).to_vec();
        b.extend_from_slice(&[0, 0, 0xFF, 0xFF]);
        assert!(matches!(
            decode_line(&b, &l()).error,
            Some(DecodeError::Truncated { .. })
        ));
    }
}
