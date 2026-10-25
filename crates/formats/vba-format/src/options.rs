//! Limits and options for reading and writing projects.

/// Resource limits applied while reading.
///
/// Every limit is checked before the corresponding allocation, so hostile
/// input is rejected with an [`Error`](crate::Error) instead of exhausting
/// memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// Largest accepted input file, in bytes.
    pub max_container_bytes: u64,
    /// Largest accepted size of any single stream, in bytes.
    pub max_stream_bytes: usize,
    /// Largest accepted size of a decompressed stream, in bytes.
    pub max_decompressed_bytes: usize,
    /// Largest accepted ratio of decompressed to compressed size. The
    /// compression format cannot exceed roughly 700:1, so the default of
    /// 1024 never rejects valid data.
    pub max_compression_ratio: usize,
    /// Largest accepted number of storages and streams.
    pub max_entries: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_container_bytes: 256 << 20,
            max_stream_bytes: 64 << 20,
            max_decompressed_bytes: 64 << 20,
            max_compression_ratio: 1024,
            max_entries: 65_536,
        }
    }
}

impl Limits {
    /// The decompressed-size cap for a compressed stream of `compressed_len`
    /// bytes.
    pub(crate) fn decompressed_cap(&self, compressed_len: usize) -> usize {
        self.max_decompressed_bytes
            .min(compressed_len.saturating_mul(self.max_compression_ratio))
    }
}

/// What to do with cached compiled state when a module's source changes.
///
/// The performance cache holds a compiled form of the source. A host that
/// trusts a stale cache runs the *old* code, so by default it is invalidated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StaleCache {
    /// Remove the edited module's cache, mark the `_VBA_PROJECT` stream as
    /// carrying no usable cache (version `0xFFFF`, no cache bytes) and drop
    /// `__SRP_*` streams. Unedited module streams are kept as they are.
    #[default]
    Invalidate,
    /// Keep every cache byte. The project may then contain compiled state that
    /// no longer matches its source; only use this if you know the host
    /// rebuilds it.
    Keep,
}

/// Options for [`write_project`](crate::write_project).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WriteOptions {
    /// Handling of caches when a module's source changed.
    pub stale_cache: StaleCache,
}
