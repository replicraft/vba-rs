//! VBA strings: sequences of UTF-16 code units (MS-VBAL §2.1, §3.3.1).
//!
//! A VBA `String` is a BSTR: a counted sequence of 16-bit code units with no
//! validity requirement, so unpaired surrogates are legal and must survive
//! every operation. `Len` counts code units, not characters.

use crate::collation::{Collation, EN_US};
use std::cmp::Ordering;
use std::fmt;

/// `Option Compare` mode (MS-VBAL §5.2.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CompareMode {
    /// `Option Compare Binary`: compare code units numerically.
    #[default]
    Binary,
    /// `Option Compare Text`: case-, width- and kana-type-insensitive
    /// comparison through a [`Collation`]; the default is the documented en-US
    /// approximation [`crate::collation::EN_US`] (NOT LCID-exact).
    Text,
}

/// An owned VBA string (UTF-16 code units, arbitrary content).
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct VbaString(Vec<u16>);

impl VbaString {
    /// Creates an empty string.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Creates a string from raw UTF-16 code units, keeping them verbatim.
    pub fn from_units(units: Vec<u16>) -> Self {
        Self(units)
    }

    /// Creates a string from a slice of code units.
    pub fn from_slice(units: &[u16]) -> Self {
        Self(units.to_vec())
    }

    /// Returns the code units.
    pub fn as_units(&self) -> &[u16] {
        &self.0
    }

    /// Consumes the string and returns its code units.
    pub fn into_units(self) -> Vec<u16> {
        self.0
    }

    /// Returns `Len(s)`: the number of UTF-16 code units.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` for the zero-length string.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `LenB(s)`: the number of bytes of the UTF-16 representation.
    pub fn len_bytes(&self) -> usize {
        self.0.len() * 2
    }

    /// The bytes of the UTF-16LE representation (what `b = s` stores in a
    /// `Byte` array): two bytes per code unit, low byte first.
    pub fn to_bytes_le(&self) -> Vec<u8> {
        self.0.iter().flat_map(|u| u.to_le_bytes()).collect()
    }

    /// Builds a string from UTF-16LE bytes (what `s = b` does with a `Byte`
    /// array). An odd number of bytes has no representation in this type
    /// (a native `BSTR` can carry a trailing half unit) and gives `None`.
    pub fn from_bytes_le(bytes: &[u8]) -> Option<Self> {
        let (pairs, rest) = bytes.as_chunks::<2>();
        if !rest.is_empty() {
            return None;
        }
        Some(Self(pairs.iter().map(|c| u16::from_le_bytes(*c)).collect()))
    }

    /// Converts to a Rust `String`, failing if there are unpaired surrogates.
    pub fn to_string_strict(&self) -> Result<String, std::string::FromUtf16Error> {
        String::from_utf16(&self.0)
    }

    /// Converts to a Rust `String`, replacing unpaired surrogates with U+FFFD.
    pub fn to_string_lossy(&self) -> String {
        String::from_utf16_lossy(&self.0)
    }

    /// Returns the concatenation `self & other`.
    pub fn concat(&self, other: &VbaString) -> VbaString {
        let mut v = Vec::with_capacity(self.0.len() + other.0.len());
        v.extend_from_slice(&self.0);
        v.extend_from_slice(&other.0);
        VbaString(v)
    }

    /// Compares two strings under the given `Option Compare` mode.
    pub fn compare(&self, other: &VbaString, mode: CompareMode) -> Ordering {
        match mode {
            CompareMode::Binary => self.0.cmp(&other.0),
            CompareMode::Text => self.compare_with(other, &EN_US),
        }
    }

    /// Compares under `Option Compare Text` using a caller-supplied
    /// collation.
    pub fn compare_with(&self, other: &VbaString, collation: &dyn Collation) -> Ordering {
        collation.compare(&self.0, &other.0)
    }

    /// Returns `true` if both strings are equal under the given mode.
    pub fn equals(&self, other: &VbaString, mode: CompareMode) -> bool {
        self.compare(other, mode) == Ordering::Equal
    }
}

impl From<&str> for VbaString {
    fn from(s: &str) -> Self {
        Self(s.encode_utf16().collect())
    }
}

impl From<String> for VbaString {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl fmt::Debug for VbaString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VbaString({:?})", self.to_string_lossy())
    }
}

impl fmt::Display for VbaString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_counts_code_units() {
        let s = VbaString::from("a\u{1F600}");
        assert_eq!(s.len(), 3);
        assert_eq!(s.len_bytes(), 6);
    }

    #[test]
    fn unpaired_surrogates_survive() {
        let s = VbaString::from_units(vec![0xD800, 0x41, 0xDC00]);
        let t = s.concat(&VbaString::from("x"));
        assert_eq!(t.as_units(), &[0xD800, 0x41, 0xDC00, 0x78]);
        assert!(s.to_string_strict().is_err());
        assert_eq!(s.to_string_lossy(), "\u{FFFD}A\u{FFFD}");
    }

    #[test]
    fn utf16le_bytes_roundtrip() {
        let s = VbaString::from("ab");
        assert_eq!(s.to_bytes_le(), vec![97, 0, 98, 0]);
        assert_eq!(VbaString::from_bytes_le(&[97, 0, 98, 0]), Some(s));
        assert_eq!(VbaString::from_bytes_le(&[97, 0, 98]), None);
        assert_eq!(VbaString::from_bytes_le(&[]), Some(VbaString::new()));
    }

    #[test]
    fn strict_roundtrip() {
        let s = VbaString::from("héllo \u{1F600}");
        assert_eq!(s.to_string_strict().unwrap(), "héllo \u{1F600}");
    }

    #[test]
    fn binary_vs_text_compare() {
        let a = VbaString::from("abc");
        let b = VbaString::from("ABC");
        assert_eq!(a.compare(&b, CompareMode::Binary), Ordering::Greater);
        assert_eq!(a.compare(&b, CompareMode::Text), Ordering::Equal);
        assert!(VbaString::from("É").equals(&VbaString::from("é"), CompareMode::Text));
        assert!(!VbaString::from("É").equals(&VbaString::from("é"), CompareMode::Binary));
        assert_eq!(
            VbaString::from("a").compare(&VbaString::from("ab"), CompareMode::Text),
            Ordering::Less
        );
    }

    #[test]
    fn text_compare_leaves_surrogates() {
        let a = VbaString::from_units(vec![0xD800]);
        let b = VbaString::from_units(vec![0xD800]);
        assert!(a.equals(&b, CompareMode::Text));
    }
}
