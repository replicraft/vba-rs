//! Error type for reading and writing VBA project storages.

use crate::compression::DecompressError;

/// Everything that can go wrong while reading or writing a project.
///
/// Malformed, truncated and hostile input is always reported through this
/// type; the reader never panics on bad data.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The input is not a readable compound file.
    #[error("not a valid compound file: {0}")]
    Container(String),
    /// A compound file operation failed while writing.
    #[error("compound file write failed: {0}")]
    ContainerWrite(String),
    /// The input is larger than the configured limit.
    #[error("input of {size} bytes exceeds the limit of {limit} bytes")]
    InputTooLarge {
        /// Size of the input.
        size: u64,
        /// The configured limit.
        limit: u64,
    },
    /// The compound file holds more entries than allowed.
    #[error("compound file has more than {limit} entries")]
    TooManyEntries {
        /// The configured limit.
        limit: usize,
    },
    /// A stream is larger than allowed.
    #[error("stream {path:?} has {size} bytes, over the limit of {limit}")]
    StreamTooLarge {
        /// Path of the stream.
        path: String,
        /// Size of the stream.
        size: u64,
        /// The configured limit.
        limit: usize,
    },
    /// A required storage or stream is missing.
    #[error("required stream or storage {path:?} is missing")]
    MissingStream {
        /// Path of the missing object.
        path: String,
    },
    /// A stream could not be read from the compound file.
    #[error("cannot read stream {path:?}: {message}")]
    StreamRead {
        /// Path of the stream.
        path: String,
        /// Underlying message.
        message: String,
    },
    /// A compressed container is invalid.
    #[error("invalid compressed data in {stream:?}: {source}")]
    Decompress {
        /// Stream the data came from.
        stream: String,
        /// Underlying decompression error.
        #[source]
        source: DecompressError,
    },
    /// The `dir` stream ended before a record was complete.
    #[error("dir stream truncated at offset {offset}: {detail}")]
    DirTruncated {
        /// Offset in the decompressed stream.
        offset: usize,
        /// What was being read.
        detail: &'static str,
    },
    /// The `dir` stream contains a record that is not allowed at this point.
    #[error("dir stream offset {offset}: expected {expected}, found record id {found:#06x}")]
    DirUnexpectedRecord {
        /// Offset in the decompressed stream.
        offset: usize,
        /// What the grammar required.
        expected: &'static str,
        /// The record identifier found.
        found: u16,
    },
    /// The `dir` stream contains an invalid value.
    #[error("dir stream offset {offset}: invalid {detail}")]
    DirInvalidValue {
        /// Offset in the decompressed stream.
        offset: usize,
        /// What is invalid.
        detail: &'static str,
    },
    /// The `_VBA_PROJECT` stream is too short to hold its header.
    #[error("_VBA_PROJECT stream has {len} bytes; its header needs 7")]
    VbaProjectTooShort {
        /// Length found.
        len: usize,
    },
    /// A module's text offset points past the end of its stream.
    #[error("module {module:?}: text offset {offset} is beyond its stream of {len} bytes")]
    ModuleOffsetOutOfRange {
        /// Module name.
        module: String,
        /// The MODULEOFFSET value.
        offset: u32,
        /// Length of the module stream.
        len: usize,
    },
    /// The project's code page has no supported converter.
    #[error("code page {0} is not supported")]
    UnsupportedCodePage(u16),
    /// Text cannot be represented in the project's code page.
    #[error("{what} contains characters that cannot be encoded in code page {code_page}")]
    Unencodable {
        /// What was being encoded.
        what: String,
        /// The project's code page.
        code_page: u16,
    },
    /// The requested change cannot be written.
    #[error("unsupported edit: {0}")]
    UnsupportedEdit(String),
    /// A value does not fit its on-disk field.
    #[error("{0}")]
    TooLarge(String),
}
