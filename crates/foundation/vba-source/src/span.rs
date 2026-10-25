/// A half-open byte range `start..end` within one source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Span {
    /// Offset of the first byte.
    pub start: u32,
    /// Offset one past the last byte.
    pub end: u32,
}

impl Span {
    /// Creates a span. `end` is clamped up to `start` if smaller.
    pub const fn new(start: u32, end: u32) -> Self {
        Span {
            start,
            end: if end < start { start } else { end },
        }
    }

    /// Creates an empty span at `offset`.
    pub const fn empty(offset: u32) -> Self {
        Span {
            start: offset,
            end: offset,
        }
    }

    /// Number of bytes covered.
    pub const fn len(&self) -> u32 {
        self.end - self.start
    }

    /// Whether the span covers no bytes.
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Whether `offset` lies inside the span (`start <= offset < end`).
    pub const fn contains(&self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Smallest span covering both `self` and `other`.
    pub const fn merge(self, other: Span) -> Span {
        Span {
            start: if self.start < other.start {
                self.start
            } else {
                other.start
            },
            end: if self.end > other.end {
                self.end
            } else {
                other.end
            },
        }
    }

    /// The bytes of `text` covered by this span, or `None` if out of range.
    pub fn slice<'a>(&self, text: &'a [u8]) -> Option<&'a [u8]> {
        text.get(self.start as usize..self.end as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let a = Span::new(2, 5);
        assert_eq!(a.len(), 3);
        assert!(a.contains(2) && a.contains(4) && !a.contains(5));
        assert_eq!(a.merge(Span::new(7, 9)), Span::new(2, 9));
        assert_eq!(Span::new(5, 2), Span::empty(5));
        assert_eq!(a.slice(b"abcdefg"), Some(&b"cde"[..]));
        assert_eq!(a.slice(b"abc"), None);
    }
}
