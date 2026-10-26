//! Positions and text encoding.
//!
//! Editors address text by line and UTF-16 code unit; the document is a byte
//! buffer in a declared encoding (UTF-8 for editor buffers by default, a
//! Windows code page for documents that came from a VBA project). This
//! module converts between the two and between Unicode strings and the
//! buffer encoding.

use vba_source::{LineIndex, SourceEncoding, Span};

/// A zero-based line and UTF-16 column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Position {
    /// Zero-based line.
    pub line: u32,
    /// Zero-based column in UTF-16 code units.
    pub character: u32,
}

impl Position {
    /// Creates a position.
    pub const fn new(line: u32, character: u32) -> Self {
        Position { line, character }
    }
}

/// A half-open range between two positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Range {
    /// First position.
    pub start: Position,
    /// Position one past the last character.
    pub end: Position,
}

impl Range {
    /// Creates a range.
    pub const fn new(start: Position, end: Position) -> Self {
        Range { start, end }
    }

    /// Whether `p` lies in `start..=end` (the end counts so a cursor
    /// directly after a word still hits it).
    pub fn contains_inclusive(&self, p: Position) -> bool {
        self.start <= p && p <= self.end
    }
}

/// Maps the declared encoding to an `encoding_rs` codec, if it needs one.
fn codec(enc: SourceEncoding) -> Option<&'static encoding_rs::Encoding> {
    let SourceEncoding::CodePage(cp) = enc else {
        return None;
    };
    let label = match cp {
        65001 => "utf-8",
        932 => "shift_jis",
        936 => "gbk",
        949 => "euc-kr",
        950 => "big5",
        874 => "windows-874",
        10000 => "macintosh",
        20866 => "koi8-r",
        1250..=1258 => return encoding_rs::Encoding::for_label(format!("windows-{cp}").as_bytes()),
        _ => return Some(encoding_rs::WINDOWS_1252),
    };
    encoding_rs::Encoding::for_label(label.as_bytes())
}

/// Whether the encoding is UTF-8 (or a code page that maps to it).
fn is_utf8(enc: SourceEncoding) -> bool {
    match enc {
        SourceEncoding::Utf8 => true,
        SourceEncoding::CodePage(_) => codec(enc) == Some(encoding_rs::UTF_8),
        SourceEncoding::Unknown => false,
    }
}

/// Whether `b` is the lead byte of a two-byte character in the DBCS code
/// pages 932, 936, 949 and 950.
fn is_dbcs_lead(cp: u16, b: u8) -> bool {
    match cp {
        932 => matches!(b, 0x81..=0x9F | 0xE0..=0xFC),
        936 | 949 | 950 => matches!(b, 0x81..=0xFE),
        _ => false,
    }
}

/// Length in bytes and UTF-16 units of the character at the start of `s`
/// (`s` is never empty and never contains a line terminator). Invalid
/// sequences count as one byte and one unit (the replacement character), so
/// every offset can be converted whatever the bytes are.
fn char_at(s: &[u8], enc: SourceEncoding) -> (usize, u32) {
    let b = s[0];
    if b < 0x80 {
        return (1, 1);
    }
    if is_utf8(enc) {
        let n = match b {
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => return (1, 1),
        };
        if s.len() >= n && std::str::from_utf8(&s[..n]).is_ok() {
            return (n, if n == 4 { 2 } else { 1 });
        }
        return (1, 1);
    }
    if let SourceEncoding::CodePage(cp) = enc
        && is_dbcs_lead(cp, b)
        && s.len() >= 2
        && s[1] >= 0x40
    {
        return (2, 1);
    }
    (1, 1)
}

/// Byte offsets of the lines of a buffer plus the buffer encoding: the unit
/// that converts between byte offsets and LSP positions.
#[derive(Debug, Clone)]
pub struct TextMap {
    lines: LineIndex,
    encoding: SourceEncoding,
    len: u32,
}

impl TextMap {
    /// Builds the map for `bytes`.
    pub fn new(bytes: &[u8], encoding: SourceEncoding) -> Self {
        TextMap {
            lines: LineIndex::new(bytes),
            encoding,
            len: bytes.len() as u32,
        }
    }

    /// Number of lines (a trailing terminator starts an empty last line).
    pub fn line_count(&self) -> u32 {
        self.lines.line_count() as u32
    }

    /// The encoding of the buffer.
    pub fn encoding(&self) -> SourceEncoding {
        self.encoding
    }

    /// Byte range of a line excluding its terminator.
    pub fn line_content(&self, bytes: &[u8], line: u32) -> Option<(u32, u32)> {
        let (s, mut e) = self.lines.line_range(line)?;
        // A line holds at most one terminator (CRLF, CR or LF).
        if e > s && bytes[e as usize - 1] == b'\n' {
            e -= 1;
            if e > s && bytes[e as usize - 1] == b'\r' {
                e -= 1;
            }
        } else if e > s && bytes[e as usize - 1] == b'\r' {
            e -= 1;
        }
        Some((s, e))
    }

    /// Byte offset to position. Offsets past the end clamp to the end;
    /// offsets inside a character or a terminator map to the position of
    /// the character start.
    pub fn position(&self, bytes: &[u8], offset: u32) -> Position {
        let offset = offset.min(self.len);
        let lc = self.lines.line_col(offset);
        let (s, e) = self.line_content(bytes, lc.line).unwrap_or((0, 0));
        let target = offset.min(e);
        let mut i = s as usize;
        let mut units = 0;
        while (i as u32) < target {
            let (n, u) = char_at(&bytes[i..e as usize], self.encoding);
            if i + n > target as usize {
                break;
            }
            i += n;
            units += u;
        }
        Position::new(lc.line, units)
    }

    /// Position to byte offset. A line past the end clamps to the end of
    /// the buffer, a column past the end of the line to the line end, and a
    /// column inside a surrogate pair to the start of the character.
    pub fn offset(&self, bytes: &[u8], pos: Position) -> u32 {
        let Some((s, e)) = self.line_content(bytes, pos.line) else {
            return self.len;
        };
        let mut i = s as usize;
        let mut units = 0;
        while i < e as usize {
            let (n, u) = char_at(&bytes[i..e as usize], self.encoding);
            if units + u > pos.character {
                break;
            }
            i += n;
            units += u;
        }
        i as u32
    }

    /// Converts a byte span to a range.
    pub fn range(&self, bytes: &[u8], span: Span) -> Range {
        Range::new(
            self.position(bytes, span.start),
            self.position(bytes, span.end),
        )
    }

    /// Converts a range to a byte span (ordered).
    pub fn span(&self, bytes: &[u8], range: Range) -> Span {
        let a = self.offset(bytes, range.start);
        let b = self.offset(bytes, range.end);
        Span::new(a.min(b), a.max(b))
    }
}

/// Decodes buffer bytes to a string (invalid bytes become U+FFFD).
pub fn decode(bytes: &[u8], enc: SourceEncoding) -> String {
    match enc {
        SourceEncoding::Unknown => bytes.iter().map(|&b| b as char).collect(),
        _ => match codec(enc) {
            Some(c) => c.decode_without_bom_handling(bytes).0.into_owned(),
            None => String::from_utf8_lossy(bytes).into_owned(),
        },
    }
}

/// Encodes a string to buffer bytes. Characters the code page cannot
/// represent become `?`.
pub fn encode(text: &str, enc: SourceEncoding) -> Vec<u8> {
    match enc {
        SourceEncoding::Utf8 => text.as_bytes().to_vec(),
        SourceEncoding::Unknown => text
            .chars()
            .map(|c| {
                if (c as u32) < 256 {
                    c as u32 as u8
                } else {
                    b'?'
                }
            })
            .collect(),
        SourceEncoding::CodePage(_) => {
            let Some(c) = codec(enc) else {
                return text.as_bytes().to_vec();
            };
            if c == encoding_rs::UTF_8 {
                return text.as_bytes().to_vec();
            }
            let (out, _, bad) = c.encode(text);
            if !bad {
                return out.into_owned();
            }
            let mut v = Vec::new();
            let mut buf = [0u8; 4];
            for ch in text.chars() {
                let (o, _, bad) = c.encode(ch.encode_utf8(&mut buf));
                if bad {
                    v.push(b'?');
                } else {
                    v.extend_from_slice(&o);
                }
            }
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(s: &[u8], e: SourceEncoding) -> TextMap {
        TextMap::new(s, e)
    }

    #[test]
    fn ascii_positions() {
        let b = b"ab\ncd\r\nef\rgh";
        let m = map(b, SourceEncoding::Utf8);
        assert_eq!(m.position(b, 0), Position::new(0, 0));
        assert_eq!(m.position(b, 4), Position::new(1, 1));
        assert_eq!(m.position(b, 7), Position::new(2, 0));
        assert_eq!(m.position(b, 9), Position::new(2, 2));
        assert_eq!(m.position(b, 10), Position::new(3, 0));
        assert_eq!(m.position(b, 12), Position::new(3, 2));
        assert_eq!(m.offset(b, Position::new(1, 1)), 4);
        assert_eq!(m.offset(b, Position::new(1, 99)), 5);
        assert_eq!(m.offset(b, Position::new(99, 0)), b.len() as u32);
        assert_eq!(m.line_count(), 4);
    }

    #[test]
    fn utf8_utf16_columns() {
        // 'é' = 2 bytes/1 unit, '€' = 3 bytes/1 unit, U+1F600 = 4 bytes/2 units
        let s = "é€😀x";
        let b = s.as_bytes();
        let m = map(b, SourceEncoding::Utf8);
        assert_eq!(m.position(b, 2), Position::new(0, 1));
        assert_eq!(m.position(b, 5), Position::new(0, 2));
        assert_eq!(m.position(b, 9), Position::new(0, 4));
        assert_eq!(m.offset(b, Position::new(0, 4)), 9);
        // inside the surrogate pair: rounds down to the character start
        assert_eq!(m.offset(b, Position::new(0, 3)), 5);
    }

    #[test]
    fn windows_1252_is_one_unit_per_byte() {
        let b = b"caf\xE9 \x80";
        let m = map(b, SourceEncoding::CodePage(1252));
        assert_eq!(m.position(b, 6), Position::new(0, 6));
        assert_eq!(decode(b, SourceEncoding::CodePage(1252)), "café €");
        assert_eq!(encode("café €", SourceEncoding::CodePage(1252)), b);
    }

    #[test]
    fn shift_jis_double_byte() {
        // "日本" = 93 FA 96 7B, then 'a'
        let b = b"\x93\xFA\x96\x7Ba";
        let m = map(b, SourceEncoding::CodePage(932));
        assert_eq!(m.position(b, 2), Position::new(0, 1));
        assert_eq!(m.position(b, 4), Position::new(0, 2));
        assert_eq!(m.position(b, 5), Position::new(0, 3));
        assert_eq!(m.offset(b, Position::new(0, 2)), 4);
        assert_eq!(decode(b, SourceEncoding::CodePage(932)), "日本a");
        assert_eq!(encode("日本a", SourceEncoding::CodePage(932)), b);
        // offset inside a double-byte character rounds down
        assert_eq!(m.position(b, 1), Position::new(0, 0));
    }

    #[test]
    fn unmappable_becomes_question_mark() {
        assert_eq!(encode("a😀b", SourceEncoding::CodePage(1252)), b"a?b");
    }

    #[test]
    fn invalid_utf8_counts_one_unit_per_byte() {
        let b = b"a\xFF\xFEb";
        let m = map(b, SourceEncoding::Utf8);
        assert_eq!(m.position(b, 3), Position::new(0, 3));
        assert_eq!(m.offset(b, Position::new(0, 3)), 3);
    }

    #[test]
    fn offset_inside_crlf_maps_to_line_end() {
        let b = b"ab\r\ncd";
        let m = map(b, SourceEncoding::Utf8);
        assert_eq!(m.position(b, 3), Position::new(0, 2));
        assert_eq!(m.position(b, 4), Position::new(1, 0));
    }
}
