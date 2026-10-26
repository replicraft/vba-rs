//! Extraction of `TYPELIB` resources from PE files (`.dll`, `.ocx`, `.exe`,
//! and the PE stubs some `.tlb` files are).

use object::LittleEndian as LE;
use object::read::pe::{
    ImageNtHeaders, PeFile, PeFile32, PeFile64, ResourceDirectory, ResourceDirectoryEntryData,
    ResourceNameOrId,
};

use crate::error::{Result, TypelibError};

/// One `TYPELIB` resource found in a PE file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypelibResource<'a> {
    /// Resource id (the number in `LoadTypeLib("x.dll\\N")`), if numeric.
    pub id: Option<u16>,
    /// Resource name, if it is a string name.
    pub name: Option<String>,
    /// Language id of the resource entry.
    pub language: Option<u16>,
    /// The raw typelib bytes.
    pub data: &'a [u8],
}

/// Whether `data` starts with a DOS header (`MZ`).
pub fn looks_like_pe(data: &[u8]) -> bool {
    data.starts_with(b"MZ")
}

/// Lists every `TYPELIB` resource of a PE file (32- or 64-bit).
///
/// # Errors
/// [`TypelibError::Pe`] when the file is not a parsable PE image or its
/// resource directory is damaged.
pub fn typelib_resources(data: &[u8]) -> Result<Vec<TypelibResource<'_>>> {
    // PE32+ images carry optional-header magic 0x20b at e_lfanew + 24.
    let lfanew = data
        .get(0x3c..0x40)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
        .ok_or_else(|| TypelibError::Pe("file shorter than a DOS header".into()))?;
    let magic = lfanew
        .checked_add(24)
        .and_then(|o| data.get(o..o.checked_add(2)?))
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| TypelibError::Pe("NT header out of range".into()))?;
    if magic == 0x20b {
        collect(&PeFile64::parse(data).map_err(pe_err)?, data)
    } else {
        collect(&PeFile32::parse(data).map_err(pe_err)?, data)
    }
}

fn pe_err(e: object::Error) -> TypelibError {
    TypelibError::Pe(e.to_string())
}

fn collect<'a, N: ImageNtHeaders>(
    pe: &PeFile<'a, N, &'a [u8]>,
    data: &'a [u8],
) -> Result<Vec<TypelibResource<'a>>> {
    let sections = pe.section_table();
    let Some(rsrc) = pe
        .data_directories()
        .resource_directory(data, &sections)
        .map_err(pe_err)?
    else {
        return Ok(Vec::new());
    };
    let root = rsrc.root().map_err(pe_err)?;
    let mut out = Vec::new();
    for ty in root.entries {
        let ResourceNameOrId::Name(n) = ty.name_or_id() else {
            continue;
        };
        let Ok(name) = n.to_string_lossy(rsrc) else {
            continue;
        };
        if !name.eq_ignore_ascii_case("TYPELIB") {
            continue;
        }
        let Some(ids) = ty.data(rsrc).map_err(pe_err)?.table() else {
            continue;
        };
        for id in ids.entries {
            let (rid, rname) = ident(id.name_or_id(), rsrc);
            let Some(langs) = id.data(rsrc).map_err(pe_err)?.table() else {
                continue;
            };
            for lang in langs.entries {
                let ResourceDirectoryEntryData::Data(entry) = lang.data(rsrc).map_err(pe_err)?
                else {
                    continue;
                };
                let rva = entry.offset_to_data.get(LE);
                let size = entry.size.get(LE) as usize;
                let bytes = sections
                    .pe_data_at(data, rva)
                    .and_then(|d| d.get(..size))
                    .ok_or(TypelibError::OutOfRange {
                        what: "TYPELIB resource data",
                        offset: rva as usize,
                    })?;
                out.push(TypelibResource {
                    id: rid,
                    name: rname.clone(),
                    language: match lang.name_or_id() {
                        ResourceNameOrId::Id(l) => Some(l),
                        ResourceNameOrId::Name(_) => None,
                    },
                    data: bytes,
                });
            }
        }
    }
    Ok(out)
}

fn ident(n: ResourceNameOrId, dir: ResourceDirectory<'_>) -> (Option<u16>, Option<String>) {
    match n {
        ResourceNameOrId::Id(i) => (Some(i), None),
        ResourceNameOrId::Name(n) => (None, n.to_string_lossy(dir).ok()),
    }
}
