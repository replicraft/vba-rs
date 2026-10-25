/// Zero-based line and byte column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineCol {
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based byte offset from the start of the line.
    pub col: u32,
}

/// Maps byte offsets to lines. A line ends with CR, LF or CRLF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    /// Offset of the first byte of each line; always starts with 0.
    starts: Vec<u32>,
    len: u32,
}

impl LineIndex {
    /// Builds the index for `text`.
    ///
    /// # Panics
    ///
    /// Panics if `text` is longer than `u32::MAX` bytes.
    pub fn new(text: &[u8]) -> Self {
        let len = u32::try_from(text.len()).expect("source larger than 4 GiB");
        let mut starts = vec![0];
        let mut i = 0usize;
        while i < text.len() {
            match text[i] {
                b'\r' => {
                    if text.get(i + 1) == Some(&b'\n') {
                        i += 1;
                    }
                    starts.push((i + 1) as u32);
                }
                b'\n' => starts.push((i + 1) as u32),
                _ => {}
            }
            i += 1;
        }
        LineIndex { starts, len }
    }

    /// Number of lines (a trailing terminator starts a final empty line).
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// Line and column of `offset` (clamped to the end of the text).
    pub fn line_col(&self, offset: u32) -> LineCol {
        let offset = offset.min(self.len);
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        LineCol {
            line: line as u32,
            col: offset - self.starts[line],
        }
    }

    /// Byte range of `line` including its terminator, or `None` if out of range.
    pub fn line_range(&self, line: u32) -> Option<(u32, u32)> {
        let start = *self.starts.get(line as usize)?;
        let end = self
            .starts
            .get(line as usize + 1)
            .copied()
            .unwrap_or(self.len);
        Some((start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_terminators() {
        let t = b"a\rbc\r\nd\ne";
        let ix = LineIndex::new(t);
        assert_eq!(ix.line_count(), 4);
        assert_eq!(ix.line_col(0), LineCol { line: 0, col: 0 });
        assert_eq!(ix.line_col(2), LineCol { line: 1, col: 0 });
        assert_eq!(ix.line_col(5), LineCol { line: 1, col: 3 }); // the LF of CRLF
        assert_eq!(ix.line_col(7), LineCol { line: 2, col: 1 });
        assert_eq!(ix.line_col(99), LineCol { line: 3, col: 1 });
        assert_eq!(ix.line_range(1), Some((2, 6)));
        assert_eq!(ix.line_range(4), None);
    }

    #[test]
    fn empty_and_trailing() {
        assert_eq!(LineIndex::new(b"").line_count(), 1);
        assert_eq!(LineIndex::new(b"x\n").line_count(), 2);
        assert_eq!(LineIndex::new(b"\r\n\r\n").line_count(), 3);
    }
}
