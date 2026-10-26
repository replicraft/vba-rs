//! Errors of the type library reader.

/// Why a type library could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TypelibError {
    /// The input is neither a typelib nor a PE file with a TYPELIB resource.
    #[error("not a type library: {0}")]
    NotATypelib(&'static str),
    /// A recognised but unsupported typelib format (for example SLTG).
    #[error("unsupported type library format: {0}")]
    UnsupportedFormat(&'static str),
    /// The PE file could not be parsed.
    #[error("invalid PE file: {0}")]
    Pe(String),
    /// The PE file has no TYPELIB resource (or not the requested one).
    #[error("no TYPELIB resource found")]
    NoTypelibResource,
    /// A structure extends past the end of the data or points outside its segment.
    #[error("truncated or out-of-range data in {what} at offset {offset:#x}")]
    OutOfRange {
        /// Which structure was being read.
        what: &'static str,
        /// The offending offset.
        offset: usize,
    },
    /// A structure is internally inconsistent.
    #[error("malformed {what}: {detail}")]
    Malformed {
        /// Which structure.
        what: &'static str,
        /// Explanation.
        detail: &'static str,
    },
    /// A file could not be read.
    #[error("i/o error: {0}")]
    Io(String),
    /// A configured size/count limit was exceeded.
    #[error("limit exceeded: {0}")]
    LimitExceeded(&'static str),
}

/// Result alias.
pub type Result<T> = std::result::Result<T, TypelibError>;
