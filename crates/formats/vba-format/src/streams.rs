//! A project as a set of named streams, independent of any container.
//!
//! An embedding application that reads a database (or any other store) itself
//! can pass the VBA storage in as a [`ProjectStreams`]: stream paths such as
//! `VBA/dir`, `VBA/_VBA_PROJECT`, `VBA/<module>`, `PROJECT`, `PROJECTwm` and
//! `VBA/__SRP_n` mapped to their bytes. Nothing that exists only in a compound
//! file (storage CLSIDs, state bits, sector layout, timestamps) is needed.

use std::collections::BTreeMap;

/// An ordered map from stream path to stream bytes.
///
/// Paths use `/` as the separator; `\` is accepted and converted and a leading
/// `/` is dropped. Lookup is ASCII case-insensitive, as when locating streams
/// in a compound file; the spelling given at insertion is kept. Iteration
/// order is deterministic: ascending by upper-cased path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectStreams {
    // Key: normalized, upper-cased path.
    items: BTreeMap<String, (String, Vec<u8>)>,
}

fn normalize(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches('/').to_owned()
}

impl ProjectStreams {
    /// An empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a stream, returning the bytes of a stream previously stored
    /// under a case-insensitively equal path (which it replaces).
    pub fn insert(&mut self, path: impl AsRef<str>, bytes: Vec<u8>) -> Option<Vec<u8>> {
        let path = normalize(path.as_ref());
        let key = path.to_ascii_uppercase();
        self.items.insert(key, (path, bytes)).map(|(_, b)| b)
    }

    /// The bytes of the stream at `path` (case-insensitive).
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.entry(path).map(|(_, b)| b)
    }

    /// The stored spelling and bytes of the stream at `path`
    /// (case-insensitive).
    pub fn entry(&self, path: &str) -> Option<(&str, &[u8])> {
        self.items
            .get(&normalize(path).to_ascii_uppercase())
            .map(|(p, b)| (p.as_str(), b.as_slice()))
    }

    /// Changes the stored spelling of an existing stream to `path`, which
    /// must equal the current path case-insensitively. Returns whether a
    /// stream was found.
    pub(crate) fn respell(&mut self, path: &str) -> bool {
        let path = normalize(path);
        match self.items.get_mut(&path.to_ascii_uppercase()) {
            Some(entry) => {
                entry.0 = path;
                true
            }
            None => false,
        }
    }

    /// Whether a stream exists at `path` (case-insensitive).
    pub fn contains(&self, path: &str) -> bool {
        self.entry(path).is_some()
    }

    /// Removes the stream at `path` (case-insensitive).
    pub fn remove(&mut self, path: &str) -> Option<Vec<u8>> {
        self.items
            .remove(&normalize(path).to_ascii_uppercase())
            .map(|(_, b)| b)
    }

    /// Number of streams.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterates over `(path, bytes)` in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.items.values().map(|(p, b)| (p.as_str(), b.as_slice()))
    }

    /// Iterates over the stream paths in deterministic order.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.items.values().map(|(p, _)| p.as_str())
    }
}

impl<P: AsRef<str>> FromIterator<(P, Vec<u8>)> for ProjectStreams {
    fn from_iter<T: IntoIterator<Item = (P, Vec<u8>)>>(iter: T) -> Self {
        let mut s = Self::new();
        for (p, b) in iter {
            s.insert(p, b);
        }
        s
    }
}

impl IntoIterator for ProjectStreams {
    type Item = (String, Vec<u8>);
    type IntoIter = std::vec::IntoIter<(String, Vec<u8>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_values().collect::<Vec<_>>().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_case_insensitive_and_keeps_spelling() {
        let mut s = ProjectStreams::new();
        s.insert("Vba/Dir", vec![1]);
        assert_eq!(s.get("VBA/DIR"), Some(&[1][..]));
        assert_eq!(s.entry("vba/dir").unwrap().0, "Vba/Dir");
        assert_eq!(s.insert("VBA/dir", vec![2]), Some(vec![1]));
        assert_eq!(s.len(), 1);
        assert_eq!(s.entry("vba/dir").unwrap().0, "VBA/dir");
    }

    #[test]
    fn separators_are_normalized() {
        let mut s = ProjectStreams::new();
        s.insert("\\VBA\\dir", vec![1]);
        assert!(s.contains("VBA/dir"));
        assert!(s.contains("/vba/DIR"));
        assert!(s.remove("vba/dir").is_some());
        assert!(s.is_empty());
    }

    #[test]
    fn order_is_deterministic() {
        let a: ProjectStreams = [("b", vec![]), ("A", vec![]), ("c/x", vec![])]
            .into_iter()
            .collect();
        let b: ProjectStreams = [("c/x", vec![]), ("b", vec![]), ("A", vec![])]
            .into_iter()
            .collect();
        assert_eq!(a, b);
        assert_eq!(a.paths().collect::<Vec<_>>(), ["A", "b", "c/x"]);
    }
}
