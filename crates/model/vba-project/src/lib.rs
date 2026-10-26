//! In-memory domain model of a VBA project.
//!
//! This crate describes *what* a VBA project is: its metadata, references and
//! modules. It knows nothing about any binary serialization (compound files,
//! MS-OVBA records, compression). Format crates convert to and from this model
//! and use [`Preserved`] to attach data they cannot or do not interpret, so a
//! project can be written back without losing it.
//!
//! The model can also represent standalone `.bas`, `.cls` and `.frm` sources;
//! see [`Module::from_source_file`].

mod availability;
mod module;
mod preserved;
mod project;
mod reference;
mod standalone;

pub use availability::{PERFORMANCE_CACHE_KEY, ProjectSourceAvailability, SourceAvailability};
pub use module::{Module, ModuleKind};
pub use preserved::Preserved;
pub use project::{
    HostExtender, ModelError, Protection, SysKind, VbaProject, VersionInfo, WindowRecord,
};
pub use reference::Reference;
pub use standalone::StandaloneError;
