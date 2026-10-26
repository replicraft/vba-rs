//! The `TextStream` object returned by `CreateTextFile`, `OpenTextFile` and
//! `OpenAsTextStream`.
//!
//! The stream decodes the whole file when it is opened and holds the text in
//! memory; a writing stream keeps the new content and writes it back when it
//! is closed (or, as a fallback, when the last reference goes away: an error
//! at that point has nowhere to go, so close streams explicitly).
//!
//! Encodings: ASCII is Windows-1252 (the system default is assumed to be
//! that code page), Unicode is UTF-16LE with a byte order mark. Error
//! numbers are **unverified** (the Learn pages say only "an error occurs"):
//! 54 for a read from a stream opened for writing, a write to a stream opened
//! for reading, or any use after `Close`; 62 for reading past the end
//! (including `ReadAll` of an empty file).

use std::cell::RefCell;
use std::rc::Rc;
use vba_automation::{Bound, DispError, ErrInfo, InvokeFlags, ObjectHandle, ObjectTable};
use vba_ffi::{Codepage, Unmappable};
use vba_value::{Variant, VbaString};

use crate::fs::{ScriptFs, parent_of};
use crate::util::{Def, dispatch_impl, getter, long_arg, method, present, string_of};

const READ: i32 = 1;
const READ_ALL: i32 = 2;
const READ_LINE: i32 = 3;
const SKIP: i32 = 4;
const SKIP_LINE: i32 = 5;
const WRITE: i32 = 6;
const WRITE_BLANK_LINES: i32 = 7;
const WRITE_LINE: i32 = 8;
const CLOSE: i32 = 9;
const AT_END_OF_LINE: i32 = 10;
const AT_END_OF_STREAM: i32 = 11;
const COLUMN: i32 = 12;
const LINE: i32 = 13;

/// How a stream was opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// `ForReading` (1).
    Read,
    /// `ForWriting` (2).
    Write,
    /// `ForAppending` (8).
    Append,
}

struct St {
    mode: Mode,
    unicode: bool,
    /// Reading: the file text. Writing: the text the file will hold.
    text: Vec<u16>,
    pos: usize,
    line: i32,
    col: i32,
    closed: bool,
    dirty: bool,
}

/// A `TextStream` object.
pub struct TextStream {
    fs: Rc<dyn ScriptFs>,
    path: String,
    st: RefCell<St>,
}

fn io_err(number: i32) -> DispError {
    DispError::from(ErrInfo::standard(number).with_source(TextStream::CLASS))
}

pub(crate) fn decode(bytes: &[u8], unicode: bool) -> Vec<u16> {
    if unicode {
        let body = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
        let (pairs, _) = body.as_chunks::<2>();
        pairs.iter().map(|c| u16::from_le_bytes(*c)).collect()
    } else {
        Codepage::Windows1252
            .decode(bytes, false)
            .unwrap_or_default()
    }
}

pub(crate) fn encode(units: &[u16], unicode: bool) -> Vec<u8> {
    if unicode {
        let mut out = vec![0xFF, 0xFE];
        out.extend(units.iter().flat_map(|u| u.to_le_bytes()));
        out
    } else {
        Codepage::Windows1252
            .encode(units, Unmappable::Replace(b'?'), false)
            .map(|e| e.bytes)
            .unwrap_or_default()
    }
}

impl TextStream {
    pub(crate) const CLASS: &'static str = "TextStream";
    pub(crate) const DEFS: &'static [Def] = &[
        method("Read", READ, &["Characters"], 1),
        method("ReadAll", READ_ALL, &[], 0),
        method("ReadLine", READ_LINE, &[], 0),
        method("Skip", SKIP, &["Characters"], 1),
        method("SkipLine", SKIP_LINE, &[], 0),
        method("Write", WRITE, &["Text"], 1),
        method("WriteBlankLines", WRITE_BLANK_LINES, &["Lines"], 1),
        method("WriteLine", WRITE_LINE, &["Text"], 0),
        method("Close", CLOSE, &[], 0),
        getter("AtEndOfLine", AT_END_OF_LINE, &[]),
        getter("AtEndOfStream", AT_END_OF_STREAM, &[]),
        getter("Column", COLUMN, &[]),
        getter("Line", LINE, &[]),
    ];

    /// Opens `abs` (a path from [`ScriptFs::resolve`]).
    ///
    /// `create` allows creating a missing file. For `Mode::Write` an
    /// existing file is emptied at once.
    pub(crate) fn open(
        fs: Rc<dyn ScriptFs>,
        abs: &str,
        mode: Mode,
        create: bool,
        unicode: bool,
    ) -> Result<TextStream, ErrInfo> {
        let stat = fs.stat(abs)?;
        if stat.as_ref().is_some_and(|s| s.is_dir) {
            // A folder is not a file: "Permission denied".
            return Err(ErrInfo::standard(70).with_source(Self::CLASS));
        }
        let text = match (mode, stat.is_some()) {
            (Mode::Read, false) => return Err(ErrInfo::standard(53).with_source(Self::CLASS)),
            (Mode::Read, true) => decode(&fs.read(abs)?, unicode),
            (Mode::Append, true) => decode(&fs.read(abs)?, unicode),
            (Mode::Write | Mode::Append, false) => {
                if !create {
                    return Err(ErrInfo::standard(53).with_source(Self::CLASS));
                }
                if let Some(p) = parent_of(abs)
                    && fs.stat(&p)?.is_none_or(|s| !s.is_dir)
                {
                    return Err(ErrInfo::standard(76).with_source(Self::CLASS));
                }
                fs.write(abs, &encode(&[], unicode))?;
                Vec::new()
            }
            (Mode::Write, true) => {
                fs.write(abs, &encode(&[], unicode))?;
                Vec::new()
            }
        };
        let (line, col) = Self::position_after(&text);
        Ok(TextStream {
            fs,
            path: abs.to_owned(),
            st: RefCell::new(St {
                mode,
                unicode,
                pos: if mode == Mode::Read { 0 } else { text.len() },
                line: if mode == Mode::Read { 1 } else { line },
                col: if mode == Mode::Read { 1 } else { col },
                text,
                closed: false,
                dirty: false,
            }),
        })
    }

    /// Line and column (1-based) after the given text.
    fn position_after(text: &[u16]) -> (i32, i32) {
        let (mut line, mut col) = (1, 1);
        for &u in text {
            if u == u16::from(b'\n') {
                line += 1;
                col = 1;
            } else if u != u16::from(b'\r') {
                col += 1;
            }
        }
        (line, col)
    }

    fn flush_locked(&self, st: &mut St) -> Result<(), ErrInfo> {
        if st.mode != Mode::Read && st.dirty {
            self.fs.write(&self.path, &encode(&st.text, st.unicode))?;
            st.dirty = false;
        }
        Ok(())
    }

    fn call(
        &self,
        _: &ObjectTable,
        id: i32,
        _: InvokeFlags,
        b: &Bound,
    ) -> Result<Variant, DispError> {
        let mut st = self.st.borrow_mut();
        if st.closed {
            return Err(io_err(54));
        }
        let readable = st.mode == Mode::Read;
        let need_read = |ok: bool| if ok { Ok(()) } else { Err(io_err(54)) };
        let nl = u16::from(b'\n');
        let cr = u16::from(b'\r');
        match id {
            READ => {
                need_read(readable)?;
                let n = long_arg(b, 0)?;
                let n = usize::try_from(n).map_err(|_| io_err(5))?;
                if st.pos >= st.text.len() {
                    return Err(io_err(62));
                }
                let end = (st.pos + n).min(st.text.len());
                let chunk = st.text[st.pos..end].to_vec();
                Self::advance(&mut st, end);
                Ok(Variant::String(VbaString::from_units(chunk)))
            }
            READ_ALL => {
                need_read(readable)?;
                if st.pos >= st.text.len() {
                    return Err(io_err(62));
                }
                let chunk = st.text[st.pos..].to_vec();
                let end = st.text.len();
                Self::advance(&mut st, end);
                Ok(Variant::String(VbaString::from_units(chunk)))
            }
            READ_LINE | SKIP_LINE => {
                need_read(readable)?;
                if st.pos >= st.text.len() {
                    return Err(io_err(62));
                }
                let rest = &st.text[st.pos..];
                let stop = rest.iter().position(|u| *u == nl);
                let (content_end, next) = match stop {
                    Some(i) => (i, i + 1),
                    None => (rest.len(), rest.len()),
                };
                let mut content = rest[..content_end].to_vec();
                if content.last() == Some(&cr) {
                    content.pop();
                }
                let to = st.pos + next;
                Self::advance(&mut st, to);
                if id == READ_LINE {
                    Ok(Variant::String(VbaString::from_units(content)))
                } else {
                    Ok(Variant::Empty)
                }
            }
            SKIP => {
                need_read(readable)?;
                let n = usize::try_from(long_arg(b, 0)?).map_err(|_| io_err(5))?;
                if st.pos >= st.text.len() {
                    return Err(io_err(62));
                }
                let to = (st.pos + n).min(st.text.len());
                Self::advance(&mut st, to);
                Ok(Variant::Empty)
            }
            AT_END_OF_STREAM => {
                need_read(readable)?;
                Ok(Variant::Boolean(st.pos >= st.text.len()))
            }
            AT_END_OF_LINE => {
                need_read(readable)?;
                Ok(Variant::Boolean(
                    st.text.get(st.pos).is_none_or(|u| *u == cr || *u == nl),
                ))
            }
            LINE => Ok(Variant::Long(st.line)),
            COLUMN => Ok(Variant::Long(st.col)),
            WRITE | WRITE_LINE | WRITE_BLANK_LINES => {
                need_read(!readable)?;
                let mut add: Vec<u16> = match id {
                    WRITE_BLANK_LINES => {
                        let n = usize::try_from(long_arg(b, 0)?).map_err(|_| io_err(5))?;
                        "\r\n".encode_utf16().cycle().take(n * 2).collect()
                    }
                    _ if present(b, 0) => string_of(&b.slots[0].value)?.into_units(),
                    _ => Vec::new(),
                };
                if id == WRITE_LINE {
                    add.extend("\r\n".encode_utf16());
                }
                for &u in &add {
                    if u == nl {
                        st.line += 1;
                        st.col = 1;
                    } else if u != cr {
                        st.col += 1;
                    }
                }
                st.text.extend(add);
                st.pos = st.text.len();
                st.dirty = true;
                Ok(Variant::Empty)
            }
            CLOSE => {
                self.flush_locked(&mut st).map_err(DispError::from)?;
                st.closed = true;
                Ok(Variant::Empty)
            }
            _ => Err(DispError::MEMBER_NOT_FOUND),
        }
    }

    /// Moves the read position to `to`, tracking line and column.
    fn advance(st: &mut St, to: usize) {
        for i in st.pos..to {
            let u = st.text[i];
            if u == u16::from(b'\n') {
                st.line += 1;
                st.col = 1;
            } else if u != u16::from(b'\r') {
                st.col += 1;
            }
        }
        st.pos = to;
    }

    fn new_enum(&self, _: &ObjectTable) -> Result<Variant, DispError> {
        Err(DispError::MEMBER_NOT_FOUND)
    }

    fn on_terminate(&self, _: &ObjectTable) {
        let mut st = self.st.borrow_mut();
        if !st.closed {
            // Nowhere to report a failure at this point.
            let _ = self.flush_locked(&mut st);
            st.closed = true;
        }
    }

    fn held(&self) -> Vec<ObjectHandle> {
        Vec::new()
    }
}

dispatch_impl!(TextStream);
