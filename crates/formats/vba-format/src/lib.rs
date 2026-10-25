//! Independent implementation of the MS-OVBA binary format.
//!
//! This crate has no dependency on any third-party VBA project library. It
//! reads and writes the compound-file storage of a VBA project
//! (`vbaProject.bin`) and converts it to and from the
//! [`vba_project::VbaProject`] model.
//!
//! * [`read_project`] parses a storage into the model, preserving every byte it
//!   does not interpret in the model's [`Preserved`](vba_project::Preserved)
//!   slots.
//! * [`write_project`] writes the model back; an untouched project round-trips
//!   with identical stream contents.
//! * [`compress`] and [`decompress`] implement the §2.4.1 container.

#[cfg(test)]
extern crate self as vba_format;
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod testsupport;

pub mod codepage;
pub mod compression;
pub mod crypt;
mod dir;
mod error;
mod options;
mod project_stream;
mod read;
mod write;

pub use compression::{DecompressError, compress, decompress};
pub use error::Error;
pub use options::{Limits, StaleCache, WriteOptions};
pub use read::read_project;
pub use write::write_project;

/// Keys under which this crate stores uninterpreted data in the model's
/// [`Preserved`](vba_project::Preserved) slots.
pub mod keys {
    /// Project: the original compressed `dir` stream.
    pub const DIR: &str = "ovba:dir";
    /// Project: bytes following the `dir` stream's terminator record.
    pub const DIR_TRAILING: &str = "ovba:dir-trailing";
    /// Project: the original `PROJECT` stream.
    pub const PROJECT_STREAM: &str = "ovba:project-stream";
    /// Project: the original `_VBA_PROJECT` stream, including its cache.
    pub const VBA_PROJECT_STREAM: &str = "ovba:vba-project-stream";
    /// Project: compound file version number (3 or 4), one byte.
    pub const CFB_VERSION: &str = "ovba:cfb-version";
    /// Project: prefix of preserved streams, followed by the stream path.
    pub const STREAM_PREFIX: &str = "ovba:stream:";
    /// Project: prefix of preserved storages, followed by the storage path
    /// (empty for the root). The value is the 16-byte CLSID followed by the
    /// little-endian 32-bit state bits.
    pub const STORAGE_PREFIX: &str = "ovba:storage:";
    /// Module: the performance cache that precedes the compressed source.
    pub const MODULE_PERFORMANCE_CACHE: &str = "ovba:module/performance-cache";
    /// Module: the original compressed source container.
    pub const MODULE_COMPRESSED_SOURCE: &str = "ovba:module/compressed-source";
}
