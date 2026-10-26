//! The `dir` stream (MS-OVBA §2.3.4.2): project information, references and
//! module records.
//!
//! [`DirInfo`] is the decompressed `dir` stream in structured form. It keeps
//! the facts the stream states (including optional records that are present or
//! absent) but not values the specification says to ignore (reserved fields,
//! cookies, redundant size fields).

use crate::codepage;
use crate::error::Error;
use vba_project::Reference;

// Record identifiers.
const ID_SYSKIND: u16 = 0x01;
const ID_LCID: u16 = 0x02;
const ID_CODEPAGE: u16 = 0x03;
const ID_NAME: u16 = 0x04;
const ID_DOCSTRING: u16 = 0x05;
const ID_HELPFILE: u16 = 0x06;
const ID_HELPCONTEXT: u16 = 0x07;
const ID_LIBFLAGS: u16 = 0x08;
const ID_VERSION: u16 = 0x09;
const ID_CONSTANTS: u16 = 0x0C;
const ID_REF_REGISTERED: u16 = 0x0D;
const ID_REF_PROJECT: u16 = 0x0E;
const ID_MODULES: u16 = 0x0F;
const ID_TERMINATOR: u16 = 0x10;
const ID_PROJECT_COOKIE: u16 = 0x13;
const ID_LCID_INVOKE: u16 = 0x14;
const ID_REF_NAME: u16 = 0x16;
const ID_MODULE_NAME: u16 = 0x19;
const ID_MODULE_STREAM_NAME: u16 = 0x1A;
const ID_MODULE_DOCSTRING: u16 = 0x1C;
const ID_MODULE_HELPCONTEXT: u16 = 0x1E;
const ID_MODULE_TYPE_PROCEDURAL: u16 = 0x21;
const ID_MODULE_TYPE_OTHER: u16 = 0x22;
const ID_MODULE_READONLY: u16 = 0x25;
const ID_MODULE_PRIVATE: u16 = 0x28;
const ID_MODULE_TERMINATOR: u16 = 0x2B;
const ID_MODULE_COOKIE: u16 = 0x2C;
const ID_REF_CONTROL: u16 = 0x2F;
const ID_REF_CONTROL_EXT: u16 = 0x30;
const ID_MODULE_OFFSET: u16 = 0x31;
const ID_REF_ORIGINAL: u16 = 0x33;
const ID_COMPAT_VERSION: u16 = 0x4A;
const ID_MODULE_NAME_UNICODE: u16 = 0x47;

/// Reserved markers that precede the Unicode half of a string record.
const RESERVED_DOCSTRING: u16 = 0x0040;
const RESERVED_HELPFILE: u16 = 0x003D;
const RESERVED_CONSTANTS: u16 = 0x003C;
const RESERVED_REF_NAME: u16 = 0x003E;
const RESERVED_STREAM_NAME: u16 = 0x0032;
const RESERVED_MODULE_DOCSTRING: u16 = 0x0048;

/// One MODULE record (§2.3.4.2.3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirModule {
    pub name: String,
    /// Whether a MODULENAMEUNICODE record is present.
    pub has_unicode_name: bool,
    pub stream_name: String,
    pub doc_string: String,
    /// MODULEOFFSET: size of the performance cache in the module stream.
    pub text_offset: u32,
    pub help_context: u32,
    /// `true` for MODULETYPE 0x0021 (procedural), `false` for 0x0022.
    pub procedural: bool,
    pub read_only: bool,
    pub private: bool,
    /// Unrecognized records inside this module's record group (tolerant
    /// reads only).
    pub unknown: Vec<UnknownRecord>,
}

/// A `dir` record whose identifier this crate does not know, kept opaque.
///
/// MS-OVBA fixes the set of records by version; a record outside it can only
/// come from a newer or non-conforming producer. In tolerant mode it is
/// assumed to have the general shape of nearly every `dir` record: a 16-bit
/// identifier, a 32-bit size and `size` payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnknownRecord {
    /// Number of known records that precede it within its scope (the project
    /// scope, or one module's scope).
    pub anchor: u32,
    /// The record identifier.
    pub id: u16,
    /// The payload after the size field.
    pub data: Vec<u8>,
}

impl UnknownRecord {
    /// Serializes a list for a `Preserved` slot: per record `anchor` (u32 LE),
    /// `id` (u16 LE), payload length (u32 LE) and the payload.
    pub(crate) fn encode_list(list: &[UnknownRecord]) -> Vec<u8> {
        let mut out = Vec::new();
        for r in list {
            out.extend_from_slice(&r.anchor.to_le_bytes());
            out.extend_from_slice(&r.id.to_le_bytes());
            out.extend_from_slice(&(r.data.len() as u32).to_le_bytes());
            out.extend_from_slice(&r.data);
        }
        out
    }

    /// Inverse of [`encode_list`](Self::encode_list); `None` if malformed.
    pub(crate) fn decode_list(mut b: &[u8]) -> Option<Vec<UnknownRecord>> {
        let mut out = Vec::new();
        while !b.is_empty() {
            let head = b.get(..10)?;
            let anchor = u32::from_le_bytes(head[0..4].try_into().ok()?);
            let id = u16::from_le_bytes(head[4..6].try_into().ok()?);
            let len = u32::from_le_bytes(head[6..10].try_into().ok()?) as usize;
            let data = b.get(10..10usize.checked_add(len)?)?.to_vec();
            b = &b[10 + len..];
            out.push(UnknownRecord { anchor, id, data });
        }
        Some(out)
    }
}

/// Every record identifier the grammar defines.
const KNOWN_IDS: [u16; 33] = [
    ID_SYSKIND,
    ID_LCID,
    ID_CODEPAGE,
    ID_NAME,
    ID_DOCSTRING,
    ID_HELPFILE,
    ID_HELPCONTEXT,
    ID_LIBFLAGS,
    ID_VERSION,
    ID_CONSTANTS,
    ID_REF_REGISTERED,
    ID_REF_PROJECT,
    ID_MODULES,
    ID_TERMINATOR,
    ID_PROJECT_COOKIE,
    ID_LCID_INVOKE,
    ID_REF_NAME,
    ID_MODULE_NAME,
    ID_MODULE_STREAM_NAME,
    ID_MODULE_DOCSTRING,
    ID_MODULE_HELPCONTEXT,
    ID_MODULE_TYPE_PROCEDURAL,
    ID_MODULE_TYPE_OTHER,
    ID_MODULE_READONLY,
    ID_MODULE_PRIVATE,
    ID_MODULE_TERMINATOR,
    ID_MODULE_COOKIE,
    ID_REF_CONTROL,
    ID_REF_CONTROL_EXT,
    ID_MODULE_OFFSET,
    ID_REF_ORIGINAL,
    ID_COMPAT_VERSION,
    ID_MODULE_NAME_UNICODE,
];

/// The decompressed `dir` stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirInfo {
    pub sys_kind: u32,
    pub compat_version: Option<u32>,
    pub lcid: u32,
    pub lcid_invoke: u32,
    pub code_page: u16,
    pub name: String,
    pub doc_string: String,
    pub help_file: String,
    pub help_context: u32,
    pub lib_flags: u32,
    pub version_major: u32,
    pub version_minor: u16,
    /// `None` when the optional PROJECTCONSTANTS record is absent.
    pub constants: Option<String>,
    pub references: Vec<Reference>,
    pub modules: Vec<DirModule>,
    /// Bytes after the terminator record; empty in conforming streams.
    pub trailing: Vec<u8>,
    /// Unrecognized records outside the modules (tolerant reads only).
    pub unknown: Vec<UnknownRecord>,
}

// ---------------------------------------------------------------- reading

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    tolerant: bool,
    /// Known records consumed in the current scope.
    count: u32,
    /// Unknown records collected in the current scope.
    unknown: Vec<UnknownRecord>,
}

impl<'a> Reader<'a> {
    fn raw_peek(&self) -> Option<u16> {
        let b = self.data.get(self.pos..self.pos.checked_add(2)?)?;
        Some(u16::from_le_bytes([b[0], b[1]]))
    }

    /// In tolerant mode, consumes records with unknown identifiers.
    fn skip_unknown(&mut self) -> Result<(), Error> {
        while self.tolerant {
            let Some(id) = self.raw_peek() else {
                return Ok(());
            };
            if KNOWN_IDS.contains(&id) {
                return Ok(());
            }
            self.pos += 2;
            let data = self.sized_bytes("unknown record")?.to_vec();
            self.unknown.push(UnknownRecord {
                anchor: self.count,
                id,
                data,
            });
        }
        Ok(())
    }

    /// The next record id, after skipping unknown records when tolerant.
    /// A truncated unknown record surfaces when the record is consumed.
    fn peek_u16(&mut self) -> Option<u16> {
        let _ = self.skip_unknown();
        self.raw_peek()
    }

    fn take(&mut self, n: usize, detail: &'static str) -> Result<&'a [u8], Error> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.data.len())
            .ok_or(Error::DirTruncated {
                offset: self.pos,
                detail,
            })?;
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    fn u16(&mut self, detail: &'static str) -> Result<u16, Error> {
        let b = self.take(2, detail)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self, detail: &'static str) -> Result<u32, Error> {
        let b = self.take(4, detail)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Reads a record id and fails unless it is `id`.
    fn expect_id(&mut self, id: u16, expected: &'static str) -> Result<(), Error> {
        self.skip_unknown()?;
        let at = self.pos;
        let found = self.u16(expected)?;
        if found != id {
            return Err(Error::DirUnexpectedRecord {
                offset: at,
                expected,
                found,
            });
        }
        self.count += 1;
        Ok(())
    }

    /// Reads a record with a fixed-size payload: id, size, payload.
    fn fixed(&mut self, id: u16, size: u32, expected: &'static str) -> Result<&'a [u8], Error> {
        self.expect_id(id, expected)?;
        let at = self.pos;
        let got = self.u32(expected)?;
        if got != size {
            return Err(Error::DirInvalidValue {
                offset: at,
                detail: expected,
            });
        }
        self.take(size as usize, expected)
    }

    fn fixed_u32(&mut self, id: u16, expected: &'static str) -> Result<u32, Error> {
        let b = self.fixed(id, 4, expected)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Reads a length-prefixed byte array (after the id has been consumed).
    fn sized_bytes(&mut self, detail: &'static str) -> Result<&'a [u8], Error> {
        let n = self.u32(detail)? as usize;
        self.take(n, detail)
    }

    /// Reads a record made of a single MBCS string: id, size, bytes.
    fn mbcs_record(&mut self, id: u16, cp: u16, expected: &'static str) -> Result<String, Error> {
        self.expect_id(id, expected)?;
        let bytes = self.sized_bytes(expected)?;
        codepage::decode(cp, bytes)
    }

    /// Reads the Unicode half of a string record: reserved word, size, bytes.
    /// Returns `None` when the half is empty.
    fn unicode_half(&mut self, detail: &'static str) -> Result<Option<String>, Error> {
        let at = self.pos;
        self.u16(detail)?; // Reserved, MUST be ignored on read
        let bytes = self.sized_bytes(detail)?;
        if !bytes.len().is_multiple_of(2) {
            return Err(Error::DirInvalidValue { offset: at, detail });
        }
        if bytes.is_empty() {
            return Ok(None);
        }
        Ok(Some(utf16_lossy(bytes)))
    }

    /// Reads a record with an MBCS half and a Unicode half. The Unicode text
    /// wins when present because it is lossless.
    fn string_pair(&mut self, id: u16, cp: u16, expected: &'static str) -> Result<String, Error> {
        self.expect_id(id, expected)?;
        let mbcs = codepage::decode(cp, self.sized_bytes(expected)?)?;
        Ok(self.unicode_half(expected)?.unwrap_or(mbcs))
    }
}

/// Decodes UTF-16LE; a trailing odd byte is ignored.
fn utf16_lossy(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    String::from_utf16_lossy(&units)
}

fn guid_to_string(b: &[u8]) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        u16::from_le_bytes([b[4], b[5]]),
        u16::from_le_bytes([b[6], b[7]]),
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

fn guid_from_string(s: &str) -> Option<[u8; 16]> {
    let b = s.as_bytes();
    if b.len() != 38
        || b[0] != b'{'
        || b[37] != b'}'
        || ![9, 14, 19, 24].iter().all(|&i| b[i] == b'-')
    {
        return None;
    }
    let hex: String = s[1..37].chars().filter(|&c| c != '-').collect();
    if hex.len() != 32 {
        return None;
    }
    let raw = crate::crypt::hex_decode(&hex)?;
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&[raw[3], raw[2], raw[1], raw[0]]);
    out[4..6].copy_from_slice(&[raw[5], raw[4]]);
    out[6..8].copy_from_slice(&[raw[7], raw[6]]);
    out[8..].copy_from_slice(&raw[8..]);
    Some(out)
}

impl DirInfo {
    /// Parses a decompressed `dir` stream.
    #[cfg(test)]
    pub(crate) fn parse(data: &[u8]) -> Result<DirInfo, Error> {
        Self::parse_with(data, false)
    }

    /// Parses a decompressed `dir` stream. With `tolerant`, records with
    /// unrecognized identifiers are kept as [`UnknownRecord`]s instead of
    /// being an error.
    pub(crate) fn parse_with(data: &[u8], tolerant: bool) -> Result<DirInfo, Error> {
        let mut r = Reader {
            data,
            pos: 0,
            tolerant,
            count: 0,
            unknown: Vec::new(),
        };

        // PROJECTINFORMATION (§2.3.4.2.1)
        let sys_kind = r.fixed_u32(ID_SYSKIND, "PROJECTSYSKIND")?;
        let compat_version = if r.peek_u16() == Some(ID_COMPAT_VERSION) {
            Some(r.fixed_u32(ID_COMPAT_VERSION, "PROJECTCOMPATVERSION")?)
        } else {
            None
        };
        let lcid = r.fixed_u32(ID_LCID, "PROJECTLCID")?;
        let lcid_invoke = r.fixed_u32(ID_LCID_INVOKE, "PROJECTLCIDINVOKE")?;
        let cp_bytes = r.fixed(ID_CODEPAGE, 2, "PROJECTCODEPAGE")?;
        let code_page = u16::from_le_bytes([cp_bytes[0], cp_bytes[1]]);
        let name = r.mbcs_record(ID_NAME, code_page, "PROJECTNAME")?;
        let doc_string = r.string_pair(ID_DOCSTRING, code_page, "PROJECTDOCSTRING")?;
        let help_file = {
            r.expect_id(ID_HELPFILE, "PROJECTHELPFILEPATH")?;
            let first = codepage::decode(code_page, r.sized_bytes("PROJECTHELPFILEPATH")?)?;
            r.u16("PROJECTHELPFILEPATH")?; // Reserved
            // HelpFile2 MUST equal HelpFile1; only the first is used.
            r.sized_bytes("PROJECTHELPFILEPATH")?;
            first
        };
        let help_context = r.fixed_u32(ID_HELPCONTEXT, "PROJECTHELPCONTEXT")?;
        let lib_flags = r.fixed_u32(ID_LIBFLAGS, "PROJECTLIBFLAGS")?;
        r.expect_id(ID_VERSION, "PROJECTVERSION")?;
        r.u32("PROJECTVERSION")?; // Reserved
        let version_major = r.u32("PROJECTVERSION")?;
        let version_minor = r.u16("PROJECTVERSION")?;
        let constants = if r.peek_u16() == Some(ID_CONSTANTS) {
            Some(r.string_pair(ID_CONSTANTS, code_page, "PROJECTCONSTANTS")?)
        } else {
            None
        };

        // PROJECTREFERENCES (§2.3.4.2.2)
        let mut references = Vec::new();
        loop {
            let at = r.pos;
            let Some(id) = r.peek_u16() else {
                return Err(Error::DirTruncated {
                    offset: at,
                    detail: "PROJECTREFERENCES",
                });
            };
            if id == ID_MODULES {
                break;
            }
            let name = if id == ID_REF_NAME {
                Some(r.string_pair(ID_REF_NAME, code_page, "REFERENCENAME")?)
            } else {
                None
            };
            references.push(parse_reference(&mut r, code_page, name)?);
        }

        // PROJECTMODULES (§2.3.4.2.3)
        let count_bytes = r.fixed(ID_MODULES, 2, "PROJECTMODULES")?;
        let count = usize::from(u16::from_le_bytes([count_bytes[0], count_bytes[1]]));
        r.fixed(ID_PROJECT_COOKIE, 2, "PROJECTCOOKIE")?;
        let mut modules = Vec::with_capacity(count.min(r.data.len() / 40));
        for _ in 0..count {
            let saved = (r.count, std::mem::take(&mut r.unknown));
            r.count = 0;
            let mut module = parse_module(&mut r, code_page)?;
            module.unknown = std::mem::replace(&mut r.unknown, saved.1);
            r.count = saved.0;
            modules.push(module);
        }

        // Terminator and Reserved
        r.expect_id(ID_TERMINATOR, "dir terminator")?;
        r.u32("dir terminator")?;
        let trailing = r.data[r.pos..].to_vec();
        let unknown = std::mem::take(&mut r.unknown);

        Ok(DirInfo {
            sys_kind,
            compat_version,
            lcid,
            lcid_invoke,
            code_page,
            name,
            doc_string,
            help_file,
            help_context,
            lib_flags,
            version_major,
            version_minor,
            constants,
            references,
            modules,
            trailing,
            unknown,
        })
    }
}

fn parse_reference(r: &mut Reader<'_>, cp: u16, name: Option<String>) -> Result<Reference, Error> {
    let at = r.pos;
    let Some(id) = r.peek_u16() else {
        return Err(Error::DirTruncated {
            offset: at,
            detail: "REFERENCE",
        });
    };
    match id {
        ID_REF_REGISTERED => {
            r.expect_id(ID_REF_REGISTERED, "REFERENCEREGISTERED")?;
            r.u32("REFERENCEREGISTERED")?; // Size, ignored
            let libid = codepage::decode(cp, r.sized_bytes("REFERENCEREGISTERED")?)?;
            r.u32("REFERENCEREGISTERED")?; // Reserved1
            r.u16("REFERENCEREGISTERED")?; // Reserved2
            Ok(Reference::Registered { name, libid })
        }
        ID_REF_PROJECT => {
            r.expect_id(ID_REF_PROJECT, "REFERENCEPROJECT")?;
            r.u32("REFERENCEPROJECT")?; // Size, ignored
            let libid_absolute = codepage::decode(cp, r.sized_bytes("REFERENCEPROJECT")?)?;
            let libid_relative = codepage::decode(cp, r.sized_bytes("REFERENCEPROJECT")?)?;
            let major_version = r.u32("REFERENCEPROJECT")?;
            let minor_version = r.u16("REFERENCEPROJECT")?;
            Ok(Reference::Project {
                name,
                libid_absolute,
                libid_relative,
                major_version,
                minor_version,
            })
        }
        ID_REF_ORIGINAL => {
            r.expect_id(ID_REF_ORIGINAL, "REFERENCEORIGINAL")?;
            let original = codepage::decode(cp, r.sized_bytes("REFERENCEORIGINAL")?)?;
            match parse_reference_control(r, cp, name, "REFERENCECONTROL after REFERENCEORIGINAL")?
            {
                Reference::Control {
                    name,
                    libid_twiddled,
                    extended_name,
                    libid_extended,
                    original_type_lib,
                    cookie,
                    ..
                } => Ok(Reference::Control {
                    name,
                    libid_twiddled,
                    extended_name,
                    libid_extended,
                    original_type_lib,
                    cookie,
                    libid_original: Some(original),
                }),
                other => Ok(other),
            }
        }
        ID_REF_CONTROL => parse_reference_control(r, cp, name, "REFERENCECONTROL"),
        found => Err(Error::DirUnexpectedRecord {
            offset: at,
            expected: "a reference record or PROJECTMODULES",
            found,
        }),
    }
}

fn parse_reference_control(
    r: &mut Reader<'_>,
    cp: u16,
    name: Option<String>,
    expected: &'static str,
) -> Result<Reference, Error> {
    r.expect_id(ID_REF_CONTROL, expected)?;
    r.u32(expected)?; // SizeTwiddled, ignored
    let libid_twiddled = codepage::decode(cp, r.sized_bytes(expected)?)?;
    r.u32(expected)?; // Reserved1
    r.u16(expected)?; // Reserved2
    let extended_name = if r.peek_u16() == Some(ID_REF_NAME) {
        Some(r.string_pair(ID_REF_NAME, cp, "REFERENCENAME")?)
    } else {
        None
    };
    r.expect_id(ID_REF_CONTROL_EXT, "REFERENCECONTROL extended part")?;
    r.u32(expected)?; // SizeExtended, ignored
    let libid_extended = codepage::decode(cp, r.sized_bytes(expected)?)?;
    r.u32(expected)?; // Reserved4
    r.u16(expected)?; // Reserved5
    let original_type_lib = guid_to_string(r.take(16, expected)?);
    let cookie = r.u32(expected)?;
    Ok(Reference::Control {
        name,
        libid_twiddled,
        extended_name,
        libid_extended,
        original_type_lib,
        cookie,
        libid_original: None,
    })
}

fn parse_module(r: &mut Reader<'_>, cp: u16) -> Result<DirModule, Error> {
    let mut name = r.mbcs_record(ID_MODULE_NAME, cp, "MODULENAME")?;
    let has_unicode_name = r.peek_u16() == Some(ID_MODULE_NAME_UNICODE);
    if has_unicode_name {
        r.expect_id(ID_MODULE_NAME_UNICODE, "MODULENAMEUNICODE")?;
        let at = r.pos;
        let bytes = r.sized_bytes("MODULENAMEUNICODE")?;
        if !bytes.len().is_multiple_of(2) {
            return Err(Error::DirInvalidValue {
                offset: at,
                detail: "MODULENAMEUNICODE",
            });
        }
        if !bytes.is_empty() {
            name = utf16_lossy(bytes);
        }
    }
    let stream_name = r.string_pair(ID_MODULE_STREAM_NAME, cp, "MODULESTREAMNAME")?;
    let doc_string = r.string_pair(ID_MODULE_DOCSTRING, cp, "MODULEDOCSTRING")?;
    let text_offset = r.fixed_u32(ID_MODULE_OFFSET, "MODULEOFFSET")?;
    let help_context = r.fixed_u32(ID_MODULE_HELPCONTEXT, "MODULEHELPCONTEXT")?;
    r.fixed(ID_MODULE_COOKIE, 2, "MODULECOOKIE")?;
    r.skip_unknown()?;
    let at = r.pos;
    let type_id = r.u16("MODULETYPE")?;
    let procedural = match type_id {
        ID_MODULE_TYPE_PROCEDURAL => true,
        ID_MODULE_TYPE_OTHER => false,
        found => {
            return Err(Error::DirUnexpectedRecord {
                offset: at,
                expected: "MODULETYPE",
                found,
            });
        }
    };
    r.count += 1;
    r.u32("MODULETYPE")?; // Reserved
    let mut read_only = false;
    let mut private = false;
    loop {
        match r.peek_u16() {
            Some(ID_MODULE_READONLY) => {
                r.fixed_flag(ID_MODULE_READONLY, "MODULEREADONLY")?;
                read_only = true;
            }
            Some(ID_MODULE_PRIVATE) => {
                r.fixed_flag(ID_MODULE_PRIVATE, "MODULEPRIVATE")?;
                private = true;
            }
            _ => break,
        }
    }
    r.expect_id(ID_MODULE_TERMINATOR, "MODULE terminator")?;
    r.u32("MODULE terminator")?; // Reserved
    Ok(DirModule {
        name,
        has_unicode_name,
        stream_name,
        doc_string,
        text_offset,
        help_context,
        procedural,
        read_only,
        private,
        unknown: Vec::new(),
    })
}

impl Reader<'_> {
    /// Reads a record that is an id followed by a 4-byte reserved field.
    fn fixed_flag(&mut self, id: u16, expected: &'static str) -> Result<(), Error> {
        self.expect_id(id, expected)?;
        self.u32(expected)?;
        Ok(())
    }
}

// ---------------------------------------------------------------- writing

type Scope = (u32, std::collections::VecDeque<UnknownRecord>);

struct Writer {
    out: Vec<u8>,
    code_page: u16,
    /// Known records written in the current scope.
    count: u32,
    /// Unknown records of the current scope still to be written.
    pending: std::collections::VecDeque<UnknownRecord>,
}

impl Writer {
    /// Starts a known record: first writes unknown records anchored here.
    fn rec(&mut self, id: u16) {
        self.flush(false);
        self.count += 1;
        self.u16(id);
    }

    fn flush(&mut self, all: bool) {
        while let Some(front) = self.pending.front() {
            if !all && front.anchor > self.count {
                break;
            }
            let r = self.pending.pop_front().expect("front exists");
            self.u16(r.id);
            self.u32(r.data.len() as u32);
            self.out.extend_from_slice(&r.data);
        }
    }

    /// Enters a scope with its own record count and unknown records; returns
    /// the state to restore with [`leave`](Self::leave).
    fn enter(&mut self, unknown: &[UnknownRecord]) -> Scope {
        let saved = (self.count, std::mem::take(&mut self.pending));
        self.count = 0;
        self.pending = unknown.iter().cloned().collect();
        saved
    }

    fn leave(&mut self, saved: Scope) {
        self.flush(true);
        self.count = saved.0;
        self.pending = saved.1;
    }

    fn u16(&mut self, v: u16) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }

    fn u32(&mut self, v: u32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }

    fn fixed_u32(&mut self, id: u16, v: u32) {
        self.rec(id);
        self.u32(4);
        self.u32(v);
    }

    fn sized(&mut self, bytes: &[u8], what: &str) -> Result<(), Error> {
        let n = u32::try_from(bytes.len())
            .map_err(|_| Error::TooLarge(format!("{what} is too large for a dir record")))?;
        self.u32(n);
        self.out.extend_from_slice(bytes);
        Ok(())
    }

    fn mbcs(&mut self, text: &str, what: &str) -> Result<(), Error> {
        let bytes = codepage::encode(self.code_page, text, what)?;
        self.sized(&bytes, what)
    }

    fn unicode(&mut self, text: &str, what: &str) -> Result<(), Error> {
        let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        self.sized(&bytes, what)
    }

    fn string_pair(&mut self, id: u16, reserved: u16, text: &str, what: &str) -> Result<(), Error> {
        self.rec(id);
        self.mbcs(text, what)?;
        self.u16(reserved);
        self.unicode(text, what)
    }
}

impl DirInfo {
    /// Serializes to a decompressed `dir` stream. Reserved and ignored fields
    /// take the values the specification requires on write.
    pub(crate) fn serialize(&self) -> Result<Vec<u8>, Error> {
        let mut w = Writer {
            out: Vec::new(),
            code_page: self.code_page,
            count: 0,
            pending: self.unknown.iter().cloned().collect(),
        };
        w.fixed_u32(ID_SYSKIND, self.sys_kind);
        if let Some(v) = self.compat_version {
            w.fixed_u32(ID_COMPAT_VERSION, v);
        }
        w.fixed_u32(ID_LCID, self.lcid);
        w.fixed_u32(ID_LCID_INVOKE, self.lcid_invoke);
        w.rec(ID_CODEPAGE);
        w.u32(2);
        w.u16(self.code_page);
        w.rec(ID_NAME);
        w.mbcs(&self.name, "project name")?;
        w.string_pair(
            ID_DOCSTRING,
            RESERVED_DOCSTRING,
            &self.doc_string,
            "project description",
        )?;
        w.rec(ID_HELPFILE);
        w.mbcs(&self.help_file, "help file path")?;
        w.u16(RESERVED_HELPFILE);
        w.mbcs(&self.help_file, "help file path")?;
        w.fixed_u32(ID_HELPCONTEXT, self.help_context);
        w.fixed_u32(ID_LIBFLAGS, self.lib_flags);
        w.rec(ID_VERSION);
        w.u32(4);
        w.u32(self.version_major);
        w.u16(self.version_minor);
        if let Some(c) = &self.constants {
            w.string_pair(ID_CONSTANTS, RESERVED_CONSTANTS, c, "project constants")?;
        }

        for reference in &self.references {
            serialize_reference(&mut w, reference)?;
        }

        let count = u16::try_from(self.modules.len())
            .map_err(|_| Error::TooLarge("a project holds at most 65535 modules".into()))?;
        w.rec(ID_MODULES);
        w.u32(2);
        w.u16(count);
        w.rec(ID_PROJECT_COOKIE);
        w.u32(2);
        w.u16(0xFFFF);
        for m in &self.modules {
            let saved = w.enter(&m.unknown);
            w.rec(ID_MODULE_NAME);
            w.mbcs(&m.name, "module name")?;
            if m.has_unicode_name {
                w.rec(ID_MODULE_NAME_UNICODE);
                w.unicode(&m.name, "module name")?;
            }
            w.string_pair(
                ID_MODULE_STREAM_NAME,
                RESERVED_STREAM_NAME,
                &m.stream_name,
                "module stream name",
            )?;
            w.string_pair(
                ID_MODULE_DOCSTRING,
                RESERVED_MODULE_DOCSTRING,
                &m.doc_string,
                "module description",
            )?;
            w.fixed_u32(ID_MODULE_OFFSET, m.text_offset);
            w.fixed_u32(ID_MODULE_HELPCONTEXT, m.help_context);
            w.rec(ID_MODULE_COOKIE);
            w.u32(2);
            w.u16(0xFFFF);
            w.rec(if m.procedural {
                ID_MODULE_TYPE_PROCEDURAL
            } else {
                ID_MODULE_TYPE_OTHER
            });
            w.u32(0);
            if m.read_only {
                w.rec(ID_MODULE_READONLY);
                w.u32(0);
            }
            if m.private {
                w.rec(ID_MODULE_PRIVATE);
                w.u32(0);
            }
            w.rec(ID_MODULE_TERMINATOR);
            w.u32(0);
            w.leave(saved);
        }
        w.rec(ID_TERMINATOR);
        w.u32(0);
        w.flush(true);
        w.out.extend_from_slice(&self.trailing);
        Ok(w.out)
    }
}

fn serialize_reference(w: &mut Writer, reference: &Reference) -> Result<(), Error> {
    let name_record = |w: &mut Writer, name: &Option<String>| -> Result<(), Error> {
        if let Some(n) = name {
            w.string_pair(ID_REF_NAME, RESERVED_REF_NAME, n, "reference name")?;
        }
        Ok(())
    };
    match reference {
        Reference::Registered { name, libid } => {
            name_record(w, name)?;
            let bytes = codepage::encode(w.code_page, libid, "reference libid")?;
            w.rec(ID_REF_REGISTERED);
            w.u32((4 + bytes.len() + 4 + 2) as u32);
            w.sized(&bytes, "reference libid")?;
            w.u32(0);
            w.u16(0);
        }
        Reference::Project {
            name,
            libid_absolute,
            libid_relative,
            major_version,
            minor_version,
        } => {
            name_record(w, name)?;
            let abs = codepage::encode(w.code_page, libid_absolute, "reference libid")?;
            let rel = codepage::encode(w.code_page, libid_relative, "reference libid")?;
            w.rec(ID_REF_PROJECT);
            w.u32((4 + abs.len() + 4 + rel.len() + 4 + 2) as u32);
            w.sized(&abs, "reference libid")?;
            w.sized(&rel, "reference libid")?;
            w.u32(*major_version);
            w.u16(*minor_version);
        }
        Reference::Control {
            name,
            libid_twiddled,
            extended_name,
            libid_extended,
            original_type_lib,
            cookie,
            libid_original,
        } => {
            name_record(w, name)?;
            if let Some(orig) = libid_original {
                w.rec(ID_REF_ORIGINAL);
                w.mbcs(orig, "reference libid")?;
            }
            let twiddled = codepage::encode(w.code_page, libid_twiddled, "reference libid")?;
            w.rec(ID_REF_CONTROL);
            w.u32((4 + twiddled.len() + 4 + 2) as u32);
            w.sized(&twiddled, "reference libid")?;
            w.u32(0);
            w.u16(0);
            name_record(w, extended_name)?;
            let ext = codepage::encode(w.code_page, libid_extended, "reference libid")?;
            let guid = guid_from_string(original_type_lib).ok_or_else(|| {
                Error::UnsupportedEdit(format!("invalid type library GUID {original_type_lib:?}"))
            })?;
            w.rec(ID_REF_CONTROL_EXT);
            w.u32((4 + ext.len() + 4 + 2 + 16 + 4) as u32);
            w.sized(&ext, "reference libid")?;
            w.u32(0);
            w.u16(0);
            w.out.extend_from_slice(&guid);
            w.u32(*cookie);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testsupport as common;

    #[test]
    fn guid_round_trip() {
        let bytes: [u8; 16] = [
            0x30, 0x04, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x46,
        ];
        let s = guid_to_string(&bytes);
        assert_eq!(s, "{00020430-0000-0000-C000-000000000046}");
        assert_eq!(guid_from_string(&s), Some(bytes));
        assert_eq!(guid_from_string("nope"), None);
        assert_eq!(
            guid_from_string("{0002043-00000-0000-C000-000000000046}"),
            None
        );
    }

    #[test]
    fn spec_original_type_lib_guid() {
        // MS-OVBA §3.1.2.2: the bytes equal {0D452EE1-E08F-101A-852E-02608C4D0BB4}.
        assert_eq!(
            guid_to_string(&common::ORIGINAL_TYPE_LIB),
            "{0D452EE1-E08F-101A-852E-02608C4D0BB4}"
        );
    }

    #[test]
    fn parses_the_specification_example() {
        let info = DirInfo::parse(&common::spec_dir_stream()).unwrap();
        // §3.1.2.1
        assert_eq!(info.sys_kind, 1);
        assert_eq!(info.compat_version, None);
        assert_eq!((info.lcid, info.lcid_invoke), (0x409, 0x409));
        assert_eq!(info.code_page, 1252);
        assert_eq!(info.name, "VBAProject");
        assert_eq!(info.doc_string, "Example VBA Project Description");
        assert_eq!(info.help_file, "c:\\example path\\example.hlp");
        assert_eq!(info.help_context, 1);
        assert_eq!(info.lib_flags, 0);
        assert_eq!((info.version_major, info.version_minor), (0x49B5196B, 6));
        assert_eq!(info.constants.as_deref(), Some("abc = 1"));
        // §3.1.2.2
        assert_eq!(info.references.len(), 4);
        assert_eq!(info.references[0].name(), Some("stdole"));
        assert!(info.references[0].libid().contains("stdole2.tlb"));
        assert_eq!(info.references[1].name(), Some("Office"));
        match &info.references[2] {
            Reference::Project {
                name,
                libid_absolute,
                libid_relative,
                major_version,
                minor_version,
            } => {
                assert_eq!(name.as_deref(), Some("VBAProject1"));
                assert_eq!(
                    libid_absolute,
                    "*\\CC:\\Example Path\\Example-ReferencedProject.xls"
                );
                assert_eq!(libid_relative, "*\\CExample-ReferencedProject.xls");
                assert_eq!((*major_version, *minor_version), (0x49A95F46, 0x0D));
            }
            other => panic!("{other:?}"),
        }
        match &info.references[3] {
            Reference::Control {
                name,
                extended_name,
                libid_twiddled,
                original_type_lib,
                cookie,
                libid_original,
                ..
            } => {
                assert_eq!(name.as_deref(), Some("MSForms"));
                assert_eq!(extended_name.as_deref(), Some("MSForms"));
                assert_eq!(libid_twiddled, common::MSFORMS_TWIDDLED);
                assert_eq!(original_type_lib, "{0D452EE1-E08F-101A-852E-02608C4D0BB4}");
                assert_eq!(*cookie, 1);
                assert_eq!(*libid_original, None);
            }
            other => panic!("{other:?}"),
        }
        // §3.1.2.3
        assert_eq!(info.modules.len(), 3);
        assert_eq!(info.modules[0].name, "ThisWorkbook");
        assert!(info.modules[0].has_unicode_name);
        assert_eq!(info.modules[1].doc_string, "Example Module Description");
        assert_eq!(info.modules[1].help_context, 2);
        assert!(info.modules[2].private);
        assert!(!info.modules[2].procedural);
        assert!(info.trailing.is_empty());
    }

    #[test]
    fn specification_example_reserializes_byte_for_byte() {
        let bytes = common::spec_dir_stream();
        let info = DirInfo::parse(&bytes).unwrap();
        assert_eq!(info.serialize().unwrap(), bytes);
    }

    #[test]
    fn reference_original_wraps_a_control() {
        let mut refs = common::spec_references();
        refs.extend(common::reference_control(
            "Forms2",
            Some("*\\G{0D452EE1-E08F-101A-852E-02608C4D0BB4}#2.0#0#C:\\x.tlb#Orig"),
        ));
        let bytes = common::dir_stream(&refs, &common::spec_modules());
        let info = DirInfo::parse(&bytes).unwrap();
        assert_eq!(info.references.len(), 5);
        match &info.references[4] {
            Reference::Control { libid_original, .. } => assert_eq!(
                libid_original.as_deref(),
                Some("*\\G{0D452EE1-E08F-101A-852E-02608C4D0BB4}#2.0#0#C:\\x.tlb#Orig")
            ),
            other => panic!("{other:?}"),
        }
        assert_eq!(info.serialize().unwrap(), bytes);
    }

    #[test]
    fn optional_records_are_handled() {
        let mut mods = common::spec_modules();
        mods[0].read_only = true;
        mods[1].procedural = true;
        let mut bytes = common::dir_stream(&common::spec_references(), &mods);
        // Insert PROJECTCOMPATVERSION after PROJECTSYSKIND (10 bytes).
        let compat = common::rec(0x004A, &common::u32le(7));
        bytes.splice(10..10, compat);
        let info = DirInfo::parse(&bytes).unwrap();
        assert_eq!(info.compat_version, Some(7));
        assert!(info.modules[0].read_only);
        assert!(info.modules[1].procedural);
        assert_eq!(info.serialize().unwrap(), bytes);
    }

    #[test]
    fn trailing_bytes_are_kept() {
        let mut bytes = common::spec_dir_stream();
        bytes.extend([1, 2, 3]);
        let info = DirInfo::parse(&bytes).unwrap();
        assert_eq!(info.trailing, [1, 2, 3]);
        assert_eq!(info.serialize().unwrap(), bytes);
    }

    #[test]
    fn every_truncation_is_an_error_not_a_panic() {
        let bytes = common::spec_dir_stream();
        for len in 0..bytes.len() {
            assert!(DirInfo::parse(&bytes[..len]).is_err(), "prefix {len}");
        }
    }

    #[test]
    fn mutated_streams_never_panic() {
        let bytes = common::spec_dir_stream();
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        for _ in 0..5000 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mut m = bytes.clone();
            for k in 0..=(seed % 3) {
                let pos = ((seed >> (8 * k)) as usize) % m.len();
                m[pos] = (seed >> 20) as u8;
            }
            let _ = DirInfo::parse(&m);
        }
    }

    #[test]
    fn huge_size_fields_are_rejected_without_allocating() {
        let mut bytes = common::spec_dir_stream();
        // PROJECTNAME size field is at offset 0x28.
        bytes[0x28..0x2C].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            DirInfo::parse(&bytes),
            Err(Error::DirTruncated { .. })
        ));
    }

    #[test]
    fn unknown_record_in_references_is_reported() {
        let mut refs = common::spec_references();
        refs.extend(common::rec(0x0777, b"zz"));
        let bytes = common::dir_stream(&refs, &common::spec_modules());
        assert!(matches!(
            DirInfo::parse(&bytes),
            Err(Error::DirUnexpectedRecord { found: 0x0777, .. })
        ));
    }

    #[test]
    fn non_ascii_text_uses_the_code_page_and_unicode() {
        let mut info = DirInfo::parse(&common::spec_dir_stream()).unwrap();
        info.doc_string = "Größe".into();
        info.modules[1].doc_string = "Ünïcode".into();
        let bytes = info.serialize().unwrap();
        let back = DirInfo::parse(&bytes).unwrap();
        assert_eq!(back, info);
        info.doc_string = "日本語".into();
        assert!(matches!(info.serialize(), Err(Error::Unencodable { .. })));
    }
}
