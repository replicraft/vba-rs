//! Reader for Windows type libraries.
//!
//! A type library is the metadata VBA binds early-bound calls to: libraries,
//! coclasses, interfaces, dispinterfaces, enums, records, aliases and
//! modules with their members (DISPIDs, invoke kinds, parameters, flags,
//! defaults, constants, GUIDs, help strings). This crate reads the
//! MSFT-format files (`.tlb`, `.olb`) and the `TYPELIB` resources embedded
//! in PE images (`.dll`, `.ocx`, `.exe`) into the [`TypeLib`] model.
//!
//! The older SLTG format is recognised and reported as
//! [`TypelibError::UnsupportedFormat`]. Nothing here loads code or calls
//! COM; the reader works on bytes only and is bounds checked (see
//! [`Limits`]). The layout and its verification status are described in
//! `docs/typelib.md`.

mod describe;
pub mod error;
pub mod model;
pub mod msft;
pub mod pe;

use std::path::Path;

pub use describe::vt_name;
pub use error::{Result, TypelibError};
pub use model::*;
pub use msft::Limits;

/// Parses an MSFT type library with the default [`Limits`].
///
/// # Errors
/// See [`msft::parse`].
pub fn parse_msft(data: &[u8]) -> Result<TypeLib> {
    msft::parse(data, Limits::default())
}

/// Loads a library from file contents: a raw typelib (`.tlb`, `.olb`) or a
/// PE image, in which case the first `TYPELIB` resource is read.
///
/// # Errors
/// Format errors as in [`parse_msft`]; [`TypelibError::NoTypelibResource`]
/// when a PE image has no `TYPELIB` resource.
pub fn load(data: &[u8]) -> Result<TypeLib> {
    if pe::looks_like_pe(data) {
        let res = pe::typelib_resources(data)?;
        let first = res.first().ok_or(TypelibError::NoTypelibResource)?;
        parse_msft(first.data)
    } else {
        parse_msft(data)
    }
}

/// Loads every library of the file: all `TYPELIB` resources of a PE image
/// (each with its resource id), or the single raw typelib (id `1`).
///
/// # Errors
/// As [`load`]; the first failing library aborts the call.
pub fn load_all(data: &[u8]) -> Result<Vec<(u16, TypeLib)>> {
    if pe::looks_like_pe(data) {
        let res = pe::typelib_resources(data)?;
        if res.is_empty() {
            return Err(TypelibError::NoTypelibResource);
        }
        res.iter()
            .map(|r| Ok((r.id.unwrap_or(0), parse_msft(r.data)?)))
            .collect()
    } else {
        Ok(vec![(1, parse_msft(data)?)])
    }
}

/// Reads and loads a file with [`load`].
///
/// # Errors
/// [`TypelibError::Io`] when the file cannot be read, otherwise as [`load`].
pub fn load_file(path: impl AsRef<Path>) -> Result<TypeLib> {
    let data = std::fs::read(path).map_err(|e| TypelibError::Io(e.to_string()))?;
    load(&data)
}
