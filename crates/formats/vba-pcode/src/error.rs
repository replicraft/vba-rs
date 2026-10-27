//! Error type of the p-code decoder.

/// Everything that can go wrong while interpreting a compiled-code cache.
///
/// All parsing is bounds-checked; hostile or truncated input is reported
/// through this type and never panics.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PcodeError {
    /// A read ran past the end of the data.
    #[error("{what}: need {needed} byte(s) at offset {offset}, data has {len}")]
    Truncated {
        /// What was being read.
        what: &'static str,
        /// Offset of the read.
        offset: usize,
        /// Bytes required.
        needed: usize,
        /// Length of the data.
        len: usize,
    },
    /// An offset or range points outside the area it must lie in.
    #[error("{what}: offset {offset} (+{size}) outside {limit} bytes")]
    OutOfBounds {
        /// What was being located.
        what: &'static str,
        /// The offending offset.
        offset: u64,
        /// Size of the item at that offset.
        size: u64,
        /// Size of the containing area.
        limit: u64,
    },
    /// The project stream does not start with the 0x61CC signature.
    #[error("project stream signature is {found:#06x}, expected 0x61cc")]
    BadSignature {
        /// The word found.
        found: u16,
    },
    /// The project stream marks the cache as unusable (version 0xFFFF).
    #[error("the project stream declares the compiled cache unusable")]
    CacheNotUsable,
    /// The module has no `0xCAFE` marker at the line table position.
    #[error("line table marker is {found:#06x}, expected 0xcafe")]
    BadLineMarker {
        /// The word found.
        found: u16,
    },
    /// The layout selected by the inputs is recognised but not implemented.
    #[error("unsupported layout: {0}")]
    UnsupportedLayout(UnsupportedLayout),
    /// The identifier walk of the project stream failed.
    #[error("project stream identifier walk failed at offset {offset}: {reason}")]
    ProjectWalk {
        /// Offset at which the walk stopped.
        offset: usize,
        /// Why.
        reason: &'static str,
    },
}

/// A layout that is identified but deliberately not decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UnsupportedLayout {
    /// Big-endian (Macintosh) streams.
    #[error("big-endian (Macintosh) streams")]
    BigEndian,
    /// The platform kind is not 32-bit or 64-bit Windows.
    #[error("platform kind {0} (neither 32-bit nor 64-bit Windows)")]
    PlatformKind(u32),
    /// The module stream says it was compiled for the other bitness than the
    /// platform kind of the project (the word at offset 2 of a VBA 6 or 7
    /// module stream is 1 for 32-bit and 3 for 64-bit Windows).
    #[error("module stream is marked for platform kind {found}, the project says {expected}")]
    PlatformMismatch {
        /// The word found in the module stream.
        found: u16,
        /// The platform kind the selected layout stands for.
        expected: u16,
    },
    /// The version word belongs to a family without a known layout.
    #[error("version word {0:#06x} has no known layout")]
    Version(u16),
}

/// Result alias of this crate.
pub type Result<T> = std::result::Result<T, PcodeError>;
