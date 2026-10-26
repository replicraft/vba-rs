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
//! * [`container`] finds the project inside OOXML packages and legacy Office
//!   documents, and replaces it inside OOXML packages.
//! * [`signature`] parses (never verifies) digital signature streams.

#[cfg(test)]
extern crate self as vba_format;
#[cfg(test)]
#[path = "../tests/common/mod.rs"]
mod testsupport;

mod aux_streams;
pub mod codepage;
mod codepage_tables;
pub mod compression;
pub mod container;
pub mod crypt;
mod dir;
mod error;
mod options;
mod project_stream;
mod read;
pub mod signature;
mod streams;
mod write;

pub use aux_streams::{
    LicenseInfo, LicenseKeys, NameMapEntry, ProjectWm, license_keys, module_name_map,
};
pub use compression::{DecompressError, compress, decompress};
pub use container::{
    ExtractedProject, ProjectLocation, extract_project, locate_ooxml_project, read_document,
    replace_ooxml_project,
};
pub use error::Error;
pub use options::{Limits, ReadMode, StaleCache, WriteOptions};
pub use read::{read_project, read_project_from_streams, read_project_with};
pub use streams::ProjectStreams;
pub use write::{
    WriteOutput, WriteReport, write_project, write_project_streams, write_project_with_report,
};

/// Keys under which this crate stores uninterpreted data in the model's
/// [`Preserved`](vba_project::Preserved) slots.
pub mod keys {
    /// Project: the original compressed `dir` stream.
    pub const DIR: &str = "ovba:dir";
    /// Project: bytes following the `dir` stream's terminator record.
    pub const DIR_TRAILING: &str = "ovba:dir-trailing";
    /// Project: unrecognized `dir` records outside modules, read in
    /// [`ReadMode::Tolerant`](crate::ReadMode::Tolerant). Per record, in
    /// order: anchor (u32 LE, count of known records before it in the scope),
    /// identifier (u16 LE), payload length (u32 LE), payload.
    pub const DIR_UNKNOWN: &str = "ovba:dir-unknown";
    /// Module: unrecognized `dir` records inside the module's record group;
    /// same encoding as [`DIR_UNKNOWN`].
    pub const MODULE_DIR_UNKNOWN: &str = "ovba:module/dir-unknown";
    /// Project: the original `PROJECT` stream.
    pub const PROJECT_STREAM: &str = "ovba:project-stream";
    /// Project: the original `_VBA_PROJECT` stream, including its cache.
    pub const VBA_PROJECT_STREAM: &str = vba_project::VBA_PROJECT_STREAM_KEY;
    /// Project: the spellings, one per line, of the `dir`, `_VBA_PROJECT`,
    /// `PROJECT` and module streams when they differ from the canonical
    /// `VBA/dir`, `VBA/_VBA_PROJECT`, `PROJECT`, `VBA/<stream name>` (stream
    /// paths are located case-insensitively; this lets a write-back use the
    /// same paths it was given).
    pub const STREAM_SPELLINGS: &str = "ovba:stream-spellings";
    /// Project: compound file version number (3 or 4), one byte.
    pub const CFB_VERSION: &str = "ovba:cfb-version";
    /// Project: prefix of preserved streams, followed by the stream path.
    pub const STREAM_PREFIX: &str = "ovba:stream:";
    /// Project: prefix of preserved storages, followed by the storage path
    /// (empty for the root). The value is the 16-byte CLSID followed by the
    /// little-endian 32-bit state bits.
    pub const STORAGE_PREFIX: &str = "ovba:storage:";
    /// Module: the performance cache that precedes the compressed source.
    pub const MODULE_PERFORMANCE_CACHE: &str = vba_project::PERFORMANCE_CACHE_KEY;
    /// Module: the original compressed source container.
    pub const MODULE_COMPRESSED_SOURCE: &str = "ovba:module/compressed-source";
}
