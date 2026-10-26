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
//! `docs/formats/typelib.md`.
//!
//! With the default `sema` feature, [`sema::TypelibProvider`] adapts loaded
//! libraries to `vba_sema::TypeProvider`; with `automation`,
//! [`automation::InterfaceTable`] gives `vba-automation` objects the
//! member ids, parameter descriptions and argument conversions of an
//! interface.

#[cfg(feature = "automation")]
pub mod automation;
mod describe;
pub mod error;
pub mod model;
pub mod msft;
pub mod pe;
#[cfg(feature = "sema")]
pub mod sema;
pub mod sltg;

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

/// Parses an SLTG type library with the default [`Limits`].
///
/// # Errors
/// See [`sltg::parse`].
pub fn parse_sltg(data: &[u8]) -> Result<TypeLib> {
    sltg::parse(data, Limits::default())
}

/// Parses a raw type library of either format, chosen by its magic
/// (`MSFT` or `SLTG`), with the default [`Limits`].
///
/// # Errors
/// [`TypelibError::NotATypelib`] for any other magic, otherwise as
/// [`parse_msft`] or [`parse_sltg`].
pub fn parse_any(data: &[u8]) -> Result<TypeLib> {
    match data.get(..4) {
        Some(b"SLTG") => parse_sltg(data),
        _ => parse_msft(data),
    }
}

/// Loads a library from file contents: a raw typelib (`.tlb`, `.olb`) or a
/// PE image, in which case the first `TYPELIB` resource is read.
///
/// # Errors
/// Format errors as in [`parse_any`]; [`TypelibError::NoTypelibResource`]
/// when a PE image has no `TYPELIB` resource.
pub fn load(data: &[u8]) -> Result<TypeLib> {
    if pe::looks_like_pe(data) {
        let res = pe::typelib_resources(data)?;
        let first = res.first().ok_or(TypelibError::NoTypelibResource)?;
        parse_any(first.data)
    } else {
        parse_any(data)
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
            .map(|r| Ok((r.id.unwrap_or(0), parse_any(r.data)?)))
            .collect()
    } else {
        Ok(vec![(1, parse_any(data)?)])
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
