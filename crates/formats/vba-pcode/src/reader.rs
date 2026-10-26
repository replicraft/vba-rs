//! Bounds-checked little-endian reading.

use crate::error::{PcodeError, Result};

/// Reads a little-endian `u16` at `offset`.
pub(crate) fn u16_at(data: &[u8], offset: usize, what: &'static str) -> Result<u16> {
    let b = slice(data, offset, 2, what)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

/// Reads a little-endian `u32` at `offset`.
pub(crate) fn u32_at(data: &[u8], offset: usize, what: &'static str) -> Result<u32> {
    let b = slice(data, offset, 4, what)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Returns `data[offset..offset + len]` or a typed error.
pub(crate) fn slice<'a>(
    data: &'a [u8],
    offset: usize,
    len: usize,
    what: &'static str,
) -> Result<&'a [u8]> {
    offset
        .checked_add(len)
        .and_then(|end| data.get(offset..end))
        .ok_or(PcodeError::Truncated {
            what,
            offset,
            needed: len,
            len: data.len(),
        })
}

/// A forward cursor over a byte slice.
#[derive(Debug, Clone)]
pub(crate) struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn len(&self) -> usize {
        self.data.len()
    }

    pub(crate) fn skip(&mut self, n: usize, what: &'static str) -> Result<()> {
        slice(self.data, self.pos, n, what)?;
        self.pos += n;
        Ok(())
    }

    pub(crate) fn u8(&mut self, what: &'static str) -> Result<u8> {
        let b = slice(self.data, self.pos, 1, what)?[0];
        self.pos += 1;
        Ok(b)
    }

    pub(crate) fn u16(&mut self, what: &'static str) -> Result<u16> {
        let v = u16_at(self.data, self.pos, what)?;
        self.pos += 2;
        Ok(v)
    }

    pub(crate) fn u32(&mut self, what: &'static str) -> Result<u32> {
        let v = u32_at(self.data, self.pos, what)?;
        self.pos += 4;
        Ok(v)
    }

    pub(crate) fn bytes(&mut self, n: usize, what: &'static str) -> Result<&'a [u8]> {
        let s = slice(self.data, self.pos, n, what)?;
        self.pos += n;
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_bounds() {
        let d = [1u8, 2, 3, 4, 5];
        let mut c = Cursor::new(&d, 0);
        assert_eq!(c.u16("a").unwrap(), 0x0201);
        assert_eq!(c.u16("b").unwrap(), 0x0403);
        assert!(matches!(c.u16("c"), Err(PcodeError::Truncated { .. })));
        assert_eq!(c.u8("d").unwrap(), 5);
        assert!(slice(&d, usize::MAX, 2, "x").is_err());
        assert!(u32_at(&d, 2, "x").is_err());
    }
}
