//! The project stream (`_VBA_PROJECT`) and its identifier table
//! (`pcode-format.md` section 3).

use crate::error::{PcodeError, Result};
use crate::layout::Layout;
use crate::reader::{Cursor, u16_at};

/// Signature word at offset 0 of the project stream.
pub const SIGNATURE: u16 = 0x61CC;

/// Reads the version word of a project stream.
///
/// # Errors
///
/// [`PcodeError::BadSignature`] when the signature is wrong,
/// [`PcodeError::Truncated`] when the stream is shorter than four bytes.
pub fn version_word(project_stream: &[u8]) -> Result<u16> {
    let sig = u16_at(project_stream, 0, "project stream signature")?;
    if sig != SIGNATURE {
        return Err(PcodeError::BadSignature { found: sig });
    }
    u16_at(project_stream, 2, "project stream version")
}

/// Everything decoding needs from the project level: the selected layout and
/// the identifier table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCache {
    /// The layout selected from the version word and platform kind.
    pub layout: Layout,
    /// The identifier table.
    pub idents: IdentTable,
}

impl ProjectCache {
    /// Reads the version word, selects the layout for `sys_kind` (the
    /// `PROJECTSYSKIND` value of the directory stream) and walks the
    /// identifier table.
    ///
    /// # Errors
    ///
    /// Signature, layout-selection and walk errors.
    pub fn parse(project_stream: &[u8], sys_kind: u32) -> Result<Self> {
        let layout = Layout::select(version_word(project_stream)?, sys_kind)?;
        let idents = parse_identifiers(project_stream, &layout)?;
        Ok(Self { layout, idents })
    }
}

/// The identifier table recovered from the project stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentTable {
    /// Identifier spellings in table order, as raw bytes of the project code
    /// page.
    pub names: Vec<Vec<u8>>,
    /// `a - c` of the walk: the number of identifier entries announced.
    pub announced: usize,
    /// Number of dummy entries skipped before the identifier entries.
    pub dummies: usize,
    /// Offset where the walk ended.
    pub end: usize,
    /// Length of the project stream.
    pub stream_len: usize,
}

impl IdentTable {
    /// Bytes of the project stream after the walk ended (the walk is expected
    /// to end at or very near the end of the stream).
    pub fn trailing_bytes(&self) -> usize {
        self.stream_len.saturating_sub(self.end)
    }

    /// The identifier at `index`, converted with Latin-1 semantics (every byte
    /// is one character).
    pub fn get(&self, index: usize) -> Option<String> {
        self.names.get(index).map(|n| latin1(n))
    }
}

/// Converts bytes to a string one byte per character.
pub(crate) fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

fn string16(c: &mut Cursor<'_>, what: &'static str) -> Result<()> {
    let n = c.u16(what)?;
    c.skip(usize::from(n), what)
}

fn string16f(c: &mut Cursor<'_>, what: &'static str) -> Result<()> {
    let n = c.u16(what)?;
    if n != 0xFFFF {
        c.skip(usize::from(n), what)?;
    }
    Ok(())
}

/// Walks the project stream and extracts the identifier table.
///
/// The walk is a long chain of relative skips (spec 3.2); the confidence is
/// medium to low except for the verified layout. A mismatch is reported as
/// [`PcodeError::ProjectWalk`] or [`PcodeError::Truncated`].
pub fn parse_identifiers(project_stream: &[u8], layout: &Layout) -> Result<IdentTable> {
    let sig = u16_at(project_stream, 0, "project stream signature")?;
    if sig != SIGNATURE {
        return Err(PcodeError::BadSignature { found: sig });
    }
    let wide_refs = layout.wide_refs();
    let wide_names = layout.wide_names();
    let narrow_names = layout.narrow_names();
    let v = layout.version;

    let mut c = Cursor::new(project_stream, 0x1E);
    let n_refs = c.u16("reference count")?;
    c.skip(2, "after reference count")?;
    for _ in 0..n_refs {
        let l = c.u16("reference length")?;
        if l == 0 {
            c.skip(6, "empty reference")?;
        } else {
            let body = c.bytes(usize::from(l), "reference body")?;
            let (threshold, at) = if wide_refs { (5, 4) } else { (3, 2) };
            if usize::from(l) >= threshold && matches!(body[at], b'C' | b'D') {
                string16(&mut c, "reference string")?;
            }
        }
        c.skip(10, "reference fixed part")?;
        let flag = c.u16("reference flag")?;
        if flag != 0 {
            string16(&mut c, "reference name")?;
            let m = c.u16("reference tail length")?;
            if m != 0 {
                c.skip(2, "reference tail extra")?;
            }
            c.skip(usize::from(m) + 30, "reference tail")?;
        }
    }
    let n = c.u16("form table count")?;
    c.skip(usize::from(n) * 2, "form table")?;
    let n = c.u16("constant pair count")?;
    c.skip(usize::from(n) * 4, "constant pairs")?;
    c.skip(2, "after constants")?;
    string16f(&mut c, "type info id")?;
    string16f(&mut c, "project description")?;
    string16f(&mut c, "help file")?;
    c.skip(0x64, "project block")?;
    let n_modules = c.u16("module count")?;
    for _ in 0..n_modules {
        let l = c.u16("module name length")?;
        if wide_names {
            c.skip(usize::from(l), "module name")?;
        }
        if narrow_names && l != 0 {
            let l2 = c.u16("module narrow name length")?;
            c.skip(usize::from(l2), "module narrow name")?;
        }
        if !(narrow_names && l == 0) {
            string16(&mut c, "module string")?;
        }
        string16f(&mut c, "module string f1")?;
        c.skip(2, "module word")?;
        if v >= 0x6B {
            string16f(&mut c, "module string f2")?;
        }
        string16f(&mut c, "module string f3")?;
        c.skip(2, "module pad")?;
        if v != 0x51 {
            c.skip(4, "module pad2")?;
        }
        let k = c.u16("module table count")?;
        c.skip(usize::from(k) * 8, "module table")?;
        c.skip(11, "module trailer")?;
    }
    c.skip(6, "after modules")?;
    let blk = c.u32("block length")?;
    let blk = usize::try_from(blk).map_err(|_| PcodeError::ProjectWalk {
        offset: c.pos(),
        reason: "block length too large",
    })?;
    c.skip(blk, "length-prefixed block")?;
    c.skip(6, "before identifier counts")?;
    let a = usize::from(c.u16("identifier count a")?);
    let b = usize::from(c.u16("identifier count b")?);
    let cc = usize::from(c.u16("identifier count c")?);
    c.skip(4, "after identifier counts")?;
    let announced = a.checked_sub(cc).ok_or(PcodeError::ProjectWalk {
        offset: c.pos(),
        reason: "identifier count a < c",
    })?;
    let dummies = (b + cc).checked_sub(a).ok_or(PcodeError::ProjectWalk {
        offset: c.pos(),
        reason: "identifier count b + c < a",
    })?;
    for _ in 0..dummies {
        c.skip(4, "dummy entry prefix")?;
        let (len, typ) = header(&mut c)?;
        if typ > 0x7F {
            c.skip(6, "dummy entry extra")?;
        }
        c.skip(len, "dummy entry text")?;
    }
    let mut names = Vec::new();
    for _ in 0..announced {
        let (mut len, mut typ) = header(&mut c)?;
        let mut reserved = false;
        while len == 0 && typ == 0 {
            reserved = true;
            c.skip(2, "reserved entry")?;
            let h = header(&mut c)?;
            len = h.0;
            typ = h.1;
        }
        if typ & 0x80 != 0 {
            c.skip(6, "identifier extra")?;
        }
        if len > 0 {
            names.push(c.bytes(len, "identifier text")?.to_vec());
        }
        if !reserved {
            c.skip(4, "identifier trailer")?;
        }
    }
    Ok(IdentTable {
        names,
        announced,
        dummies,
        end: c.pos(),
        stream_len: c.len(),
    })
}

/// Reads an entry header, returning `(length, type)`.
fn header(c: &mut Cursor<'_>) -> Result<(usize, u8)> {
    let len = c.u8("identifier length")?;
    let typ = c.u8("identifier type")?;
    Ok((usize::from(len), typ))
}
