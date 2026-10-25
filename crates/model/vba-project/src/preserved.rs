/// Opaque, uninterpreted data attached to a model object by a format layer.
///
/// Entries are keyed by a string and hold raw bytes. The model never looks
/// inside them. Format crates choose their own keys and should namespace them
/// with a format prefix (for example `"ovba:dir"`) so several formats can
/// coexist on one object. Entries are kept in key order, so iteration is
/// deterministic.
///
/// Cloning a model object clones its preserved data; the model gives no
/// guarantee that an entry is still meaningful after the surrounding object
/// was edited. Deciding that is the job of the format layer that wrote it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Preserved {
    items: std::collections::BTreeMap<String, Vec<u8>>,
}

impl Preserved {
    /// Creates an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores `bytes` under `key`, returning the previous value if any.
    pub fn insert(&mut self, key: impl Into<String>, bytes: Vec<u8>) -> Option<Vec<u8>> {
        self.items.insert(key.into(), bytes)
    }

    /// Returns the bytes stored under `key`.
    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.items.get(key).map(Vec::as_slice)
    }

    /// Removes and returns the bytes stored under `key`.
    pub fn remove(&mut self, key: &str) -> Option<Vec<u8>> {
        self.items.remove(key)
    }

    /// Whether `key` is present.
    pub fn contains(&self, key: &str) -> bool {
        self.items.contains_key(key)
    }

    /// Iterates over `(key, bytes)` pairs in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.items.iter().map(|(k, v)| (k.as_str(), v.as_slice()))
    }

    /// Iterates over the entries whose key starts with `prefix`, yielding the
    /// key with the prefix removed.
    pub fn iter_prefixed<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a [u8])> {
        self.items
            .range(prefix.to_owned()..)
            .take_while(move |(k, _)| k.starts_with(prefix))
            .map(move |(k, v)| (&k[prefix.len()..], v.as_slice()))
    }

    /// Removes every entry whose key starts with `prefix`.
    pub fn remove_prefixed(&mut self, prefix: &str) {
        self.items.retain(|k, _| !k.starts_with(prefix));
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are no entries.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Total number of preserved bytes across all entries.
    pub fn total_bytes(&self) -> usize {
        self.items.values().map(Vec::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_remove() {
        let mut p = Preserved::new();
        assert!(p.is_empty());
        assert_eq!(p.insert("a:x", vec![1, 2]), None);
        assert_eq!(p.insert("a:x", vec![3]), Some(vec![1, 2]));
        assert_eq!(p.get("a:x"), Some(&[3][..]));
        assert!(p.contains("a:x"));
        assert_eq!(p.total_bytes(), 1);
        assert_eq!(p.remove("a:x"), Some(vec![3]));
        assert_eq!(p.get("a:x"), None);
    }

    #[test]
    fn prefix_iteration_is_ordered_and_exact() {
        let mut p = Preserved::new();
        p.insert("f:b", vec![2]);
        p.insert("f:a", vec![1]);
        p.insert("g:a", vec![9]);
        p.insert("e:z", vec![0]);
        let got: Vec<_> = p.iter_prefixed("f:").collect();
        assert_eq!(got, vec![("a", &[1][..]), ("b", &[2][..])]);
        p.remove_prefixed("f:");
        assert_eq!(p.len(), 2);
        assert_eq!(p.iter().count(), 2);
    }
}
