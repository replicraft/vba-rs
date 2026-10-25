use crate::{LineCol, LineIndex, Span};

/// Identifier of a file registered in a [`SourceMap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

/// How the stored bytes of a source file are encoded.
///
/// This records what the container (for example a VBA project's `dir`
/// stream) declared; the bytes themselves are never converted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceEncoding {
    /// A Windows code page identified by number (1252, 932, ...).
    CodePage(u16),
    /// UTF-8 text (typically exported `.bas`/`.cls` files from tools).
    Utf8,
    /// Encoding not known; treated as a single-byte superset of ASCII.
    Unknown,
}

/// One source file: original bytes, encoding and line index.
#[derive(Debug, Clone)]
pub struct SourceFile {
    name: String,
    bytes: Vec<u8>,
    encoding: SourceEncoding,
    lines: LineIndex,
}

impl SourceFile {
    /// Creates a source file, keeping `bytes` exactly as given.
    pub fn new(name: impl Into<String>, bytes: Vec<u8>, encoding: SourceEncoding) -> Self {
        let lines = LineIndex::new(&bytes);
        SourceFile {
            name: name.into(),
            bytes,
            encoding,
            lines,
        }
    }

    /// Display name (usually a path or module name).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The original bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The declared encoding.
    pub fn encoding(&self) -> SourceEncoding {
        self.encoding
    }

    /// The line index.
    pub fn lines(&self) -> &LineIndex {
        &self.lines
    }

    /// Line and column of a byte offset.
    pub fn line_col(&self, offset: u32) -> LineCol {
        self.lines.line_col(offset)
    }

    /// Text of a line without its terminator, for display only.
    ///
    /// Bytes are decoded as UTF-8 for [`SourceEncoding::Utf8`] and as
    /// Latin-1 otherwise, so non-ASCII text in other code pages may appear
    /// incorrectly. Never use this to interpret source.
    pub fn line_text_lossy(&self, line: u32) -> Option<String> {
        let (s, e) = self.lines.line_range(line)?;
        let mut b = &self.bytes[s as usize..e as usize];
        while let [rest @ .., b'\r' | b'\n'] = b {
            b = rest;
        }
        Some(match self.encoding {
            SourceEncoding::Utf8 => String::from_utf8_lossy(b).into_owned(),
            _ => b.iter().map(|&c| c as char).collect(),
        })
    }

    /// The bytes covered by `span`.
    pub fn slice(&self, span: Span) -> Option<&[u8]> {
        span.slice(&self.bytes)
    }
}

/// A collection of source files addressed by [`FileId`].
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    /// Creates an empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file and returns its id.
    pub fn add(
        &mut self,
        name: impl Into<String>,
        bytes: Vec<u8>,
        encoding: SourceEncoding,
    ) -> FileId {
        let id = FileId(u32::try_from(self.files.len()).expect("too many files"));
        self.files.push(SourceFile::new(name, bytes, encoding));
        id
    }

    /// Looks up a file.
    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }

    /// Number of files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether the map has no files.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_original_bytes() {
        let raw = vec![b'a', 0xE9, 0xFF, b'\r', b'\n', b'x'];
        let mut map = SourceMap::new();
        let id = map.add("m.bas", raw.clone(), SourceEncoding::CodePage(1252));
        let f = map.get(id).unwrap();
        assert_eq!(f.bytes(), &raw[..]);
        assert_eq!(f.encoding(), SourceEncoding::CodePage(1252));
        assert_eq!(f.line_text_lossy(0).as_deref(), Some("a\u{e9}\u{ff}"));
        assert_eq!(f.line_text_lossy(1).as_deref(), Some("x"));
        assert_eq!(f.line_col(5), LineCol { line: 1, col: 0 });
        assert!(map.get(FileId(3)).is_none());
    }
}
