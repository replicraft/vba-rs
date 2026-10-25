//! Independent implementation of the MS-OVBA binary format.
//!
//! This crate has no dependency on any third-party VBA project library.

pub mod compression;

pub use compression::{DecompressError, compress, decompress};
