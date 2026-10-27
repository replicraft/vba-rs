//! Finding the VBA project inside host documents.
//!
//! * OOXML packages (`.docm`, `.xlsm`, `.pptm`, `.dotm`, `.xlam`, `.xlsb` ...):
//!   the project is the package part reached from the main document part by
//!   a relationship of type
//!   `http://schemas.microsoft.com/office/2006/relationships/vbaProject`
//!   (MS-OFFMACRO §2.1.1.2, §2.2.1.2, §2.3.1.2). The part is found through the
//!   package relationships; no file name is assumed.
//! * Word binary documents (`.doc`): the `Macros` storage (MS-DOC §2.1.9).
//! * Excel binary workbooks (`.xls`): the `_VBA_PROJECT_CUR` storage
//!   (MS-XLS §2.1.7.18).
//! * PowerPoint binary presentations (`.ppt`): the `VbaProjectStg` persist
//!   object, located through the `Current User` stream, the user edit chain,
//!   the persist directory, the document container and its `VBAInfoAtom`
//!   (MS-PPT §2.1.2 Part 1, 2 and 11; §2.4.10, §2.4.11, §2.10.40 to §2.10.42).
//!
//! The functions here only find and copy the project. They never run it and
//! never decrypt anything: encrypted packages and encrypted binary documents
//! are reported as [`Error::UnsupportedContainer`].

use std::io::{Cursor, Read};

use vba_project::VbaProject;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::Error;
use crate::options::{Limits, ReadMode};
use crate::read::{open_and_list, read_project_with, read_stream};
use crate::write::{Layout, assemble};

/// The relationship type that links the main part to the VBA project part.
pub const VBA_PROJECT_RELATIONSHIP: &str =
    "http://schemas.microsoft.com/office/2006/relationships/vbaProject";

const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const ZIP_MAGIC: [u8; 2] = *b"PK";

/// Where a project was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectLocation {
    /// The input is itself a project storage (`vbaProject.bin`).
    RawStorage,
    /// A part of an OOXML package, by part name (no leading slash).
    OoxmlPart(String),
    /// The `Macros` storage of a Word binary document.
    WordMacros,
    /// The `_VBA_PROJECT_CUR` storage of an Excel binary workbook.
    ExcelVbaProjectCur,
    /// The `VbaProjectStg` of a PowerPoint binary presentation.
    PowerPointVbaProjectStg {
        /// Whether it was stored compressed (zlib).
        compressed: bool,
    },
}

/// A project storage copied out of its host document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedProject {
    /// Where it was found.
    pub location: ProjectLocation,
    /// The project as a standalone compound file, readable by
    /// [`read_project`](crate::read_project). For [`ProjectLocation::RawStorage`]
    /// and [`ProjectLocation::OoxmlPart`] these are the original bytes; for
    /// the legacy layouts it is a new compound file holding the same streams
    /// and storages (stream contents are identical, container layout is not).
    pub bytes: Vec<u8>,
}

fn unsupported(msg: impl Into<String>) -> Error {
    Error::UnsupportedContainer(msg.into())
}

fn package_err(msg: impl Into<String>) -> Error {
    Error::Package(msg.into())
}

fn limit_err(msg: impl Into<String>) -> Error {
    Error::ArchiveLimit(msg.into())
}

/// Extracts the VBA project from a raw storage, an OOXML package or a binary
/// Office document, detected by content (not by file name).
pub fn extract_project(bytes: &[u8], limits: &Limits) -> Result<ExtractedProject, Error> {
    if bytes.len() as u64 > limits.max_container_bytes {
        return Err(Error::InputTooLarge {
            size: bytes.len() as u64,
            limit: limits.max_container_bytes,
        });
    }
    if bytes.starts_with(&ZIP_MAGIC) {
        let (part, data) = extract_ooxml(bytes, limits)?;
        return Ok(ExtractedProject {
            location: ProjectLocation::OoxmlPart(part),
            bytes: data,
        });
    }
    if bytes.starts_with(&CFB_MAGIC) {
        return extract_from_compound_file(bytes, limits);
    }
    Err(unsupported(
        "the input is neither a ZIP package nor a compound file",
    ))
}

/// Extracts and reads the project of any supported document.
pub fn read_document(
    bytes: &[u8],
    limits: &Limits,
    mode: ReadMode,
) -> Result<(VbaProject, ProjectLocation), Error> {
    let extracted = extract_project(bytes, limits)?;
    let project = read_project_with(&extracted.bytes, limits, mode)?;
    Ok((project, extracted.location))
}

// ------------------------------------------------------------------ OOXML

type Zip<'a> = ZipArchive<Cursor<&'a [u8]>>;

fn open_zip<'a>(bytes: &'a [u8], limits: &Limits) -> Result<Zip<'a>, Error> {
    let archive = ZipArchive::new(Cursor::new(bytes)).map_err(|e| package_err(e.to_string()))?;
    if archive.len() > limits.max_archive_entries {
        return Err(limit_err(format!(
            "the archive has {} entries, over the limit of {}",
            archive.len(),
            limits.max_archive_entries
        )));
    }
    if archive.has_duplicate_names() {
        return Err(package_err("the archive has duplicate entry names"));
    }
    Ok(archive)
}

/// Index of the entry for a part name: an exact match, else an ASCII
/// case-insensitive match (part names are case-insensitive in OPC).
fn find_entry(archive: &Zip<'_>, part: &str) -> Option<usize> {
    if let Some(i) = archive.index_for_name(part) {
        return Some(i);
    }
    (0..archive.len()).find(|&i| {
        archive
            .name_for_index(i)
            .and_then(Result::ok)
            .is_some_and(|n| n.trim_start_matches('/').eq_ignore_ascii_case(part))
    })
}

/// Reads one entry fully, enforcing the size and ratio limits against what is
/// actually decompressed (header fields are not trusted).
fn read_entry(
    archive: &mut Zip<'_>,
    index: usize,
    max_len: u64,
    limits: &Limits,
) -> Result<Vec<u8>, Error> {
    let file = archive
        .by_index(index)
        .map_err(|e| package_err(e.to_string()))?;
    if file.encrypted() {
        return Err(unsupported("an encrypted ZIP entry"));
    }
    let name = file.name_raw().to_vec();
    let name = String::from_utf8_lossy(&name).into_owned();
    if file.size() > max_len {
        return Err(limit_err(format!(
            "entry {name:?} declares {} bytes, over the limit of {max_len}",
            file.size()
        )));
    }
    let ratio_cap = file
        .compressed_size()
        .max(64)
        .saturating_mul(limits.max_archive_ratio as u64);
    let cap = max_len.min(ratio_cap);
    let mut out = Vec::new();
    file.take(cap.saturating_add(1))
        .read_to_end(&mut out)
        .map_err(|e| package_err(format!("entry {name:?}: {e}")))?;
    if out.len() as u64 > cap {
        return Err(limit_err(format!(
            "entry {name:?} expands beyond the size or compression ratio limit"
        )));
    }
    Ok(out)
}

/// Percent-decodes a part name taken from a relationship target.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (
                (b[i + 1] as char).to_digit(16),
                (b[i + 2] as char).to_digit(16),
            )
        {
            out.push((h * 16 + l) as u8);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Resolves a relationship target against its source part, giving a part
/// name without a leading slash. `source` is empty for the package root.
fn resolve_target(source: &str, target: &str) -> Result<String, Error> {
    let target = percent_decode(target.split('#').next().unwrap_or(""));
    let mut segments: Vec<&str> = Vec::new();
    let path = if let Some(abs) = target.strip_prefix('/') {
        abs.to_owned()
    } else {
        let dir = source.rsplit_once('/').map_or("", |(d, _)| d);
        if dir.is_empty() {
            target.clone()
        } else {
            format!("{dir}/{target}")
        }
    };
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Err(package_err(format!(
                        "relationship target {target:?} leaves the package"
                    )));
                }
            }
            s => segments.push(s),
        }
    }
    if segments.is_empty() {
        return Err(package_err("relationship target names no part"));
    }
    Ok(segments.join("/"))
}

/// One `Relationship` element.
struct Relationship {
    ty: String,
    target: String,
    external: bool,
}

fn parse_relationships(xml: &[u8]) -> Result<Vec<Relationship>, Error> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut out = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| package_err(format!("malformed relationships part: {e}")))?
        {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == "Relationship" => {
                let (mut ty, mut target, mut external) = (None, None, false);
                for attr in e.attributes() {
                    let attr = attr
                        .map_err(|e| package_err(format!("malformed relationships part: {e}")))?;
                    let value = attr
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(|e| package_err(format!("malformed relationships part: {e}")))?
                        .into_owned();
                    match attr.key.local_name().as_ref() {
                        "Type" => ty = Some(value),
                        "Target" => target = Some(value),
                        "TargetMode" => external = value.eq_ignore_ascii_case("External"),
                        _ => {}
                    }
                }
                if let (Some(ty), Some(target)) = (ty, target) {
                    out.push(Relationship {
                        ty,
                        target,
                        external,
                    });
                }
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

/// The part name of the relationships part belonging to `source`.
fn rels_part_name(source: &str) -> String {
    match source.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None if source.is_empty() => "_rels/.rels".to_owned(),
        None => format!("_rels/{source}.rels"),
    }
}

/// Reads the relationships of `source` (empty for the package). A missing
/// relationships part means no relationships.
fn relationships(
    archive: &mut Zip<'_>,
    source: &str,
    limits: &Limits,
) -> Result<Vec<Relationship>, Error> {
    let name = rels_part_name(source);
    match find_entry(archive, &name) {
        None => Ok(Vec::new()),
        Some(i) => parse_relationships(&read_entry(
            archive,
            i,
            limits.max_stream_bytes as u64,
            limits,
        )?),
    }
}

/// Finds the VBA project part of an OOXML package through its relationships:
/// package root, then the `officeDocument` part, then the `vbaProject`
/// relationship of that part. Returns the part name and its entry index.
fn locate_vba_part(archive: &mut Zip<'_>, limits: &Limits) -> Result<(String, usize), Error> {
    let root = relationships(archive, "", limits)?;
    let main = root
        .iter()
        .find(|r| !r.external && r.ty.ends_with("/officeDocument"))
        .ok_or_else(|| package_err("the package has no officeDocument relationship"))?;
    let main_part = resolve_target("", &main.target)?;
    let rels = relationships(archive, &main_part, limits)?;
    let mut vba = rels
        .iter()
        .filter(|r| !r.external && r.ty == VBA_PROJECT_RELATIONSHIP);
    let rel = vba.next().ok_or(Error::NoVbaProject(
        "the main part has no vbaProject relationship",
    ))?;
    if vba.next().is_some() {
        return Err(package_err(
            "the main part has more than one vbaProject relationship",
        ));
    }
    let part = resolve_target(&main_part, &rel.target)?;
    let index = find_entry(archive, &part).ok_or_else(|| {
        package_err(format!(
            "the vbaProject relationship targets {part:?}, which is not in the package"
        ))
    })?;
    Ok((part, index))
}

/// Returns the part name of the VBA project in an OOXML package.
pub fn locate_ooxml_project(package: &[u8], limits: &Limits) -> Result<String, Error> {
    let mut archive = open_zip(package, limits)?;
    locate_vba_part(&mut archive, limits).map(|(part, _)| part)
}

fn extract_ooxml(package: &[u8], limits: &Limits) -> Result<(String, Vec<u8>), Error> {
    let mut archive = open_zip(package, limits)?;
    let (part, index) = locate_vba_part(&mut archive, limits)?;
    let data = read_entry(&mut archive, index, limits.max_container_bytes, limits)?;
    Ok((part, data))
}

/// Replaces the VBA project part of an OOXML package.
///
/// The package is rewritten entry by entry, in the original order. Every
/// entry except the project part is copied *raw*: its compressed bytes are
/// not touched, and its name bytes, compression method, modification time,
/// CRC, sizes, attributes, comment and extra fields are carried over, as is the
/// archive comment. The project part is written under the same name, with the
/// same compression method (deflate if the original method is unavailable),
/// time and attributes. The relationships, content types and any signature
/// part are not modified.
///
/// What is **not** preserved byte for byte: the ZIP structure around the
/// entries (offsets, central directory, any data before the first entry or
/// between entries, ZIP64 records, data-descriptor use), and the project
/// part's own compressed bytes, comment and extra fields.
///
/// `new_project` must be a compound file; it is not otherwise checked.
pub fn replace_ooxml_project(
    package: &[u8],
    new_project: &[u8],
    limits: &Limits,
) -> Result<Vec<u8>, Error> {
    if !new_project.starts_with(&CFB_MAGIC) {
        return Err(package_err("the replacement is not a compound file"));
    }
    let mut archive = open_zip(package, limits)?;
    let (_, target) = locate_vba_part(&mut archive, limits)?;
    let zerr = |e: zip::result::ZipError| package_err(e.to_string());
    let mut writer = ZipWriter::new(Cursor::new(Vec::with_capacity(package.len())));
    writer
        .set_raw_comment(archive.comment().into())
        .map_err(zerr)?;
    for i in 0..archive.len() {
        let file = archive.by_index_raw(i).map_err(zerr)?;
        if i == target {
            let method = match file.compression() {
                CompressionMethod::Stored => CompressionMethod::Stored,
                _ => CompressionMethod::Deflated,
            };
            let options = file.options().compression_method(method);
            let name = file
                .name()
                .map_err(|e| package_err(e.to_string()))?
                .into_owned();
            drop(file);
            writer.start_file(name, options).map_err(zerr)?;
            std::io::Write::write_all(&mut writer, new_project)
                .map_err(|e| package_err(e.to_string()))?;
        } else {
            writer.raw_copy_file(file).map_err(zerr)?;
        }
    }
    Ok(writer.finish().map_err(zerr)?.into_inner())
}

// ------------------------------------------------------- legacy documents

fn extract_from_compound_file(bytes: &[u8], limits: &Limits) -> Result<ExtractedProject, Error> {
    let (mut cf, nodes) = open_and_list(bytes, limits)?;
    let has_stream = |name: &str| {
        nodes
            .iter()
            .any(|n| n.is_stream && n.path.eq_ignore_ascii_case(name))
    };
    let storage = |name: &str| {
        nodes
            .iter()
            .find(|n| !n.is_stream && n.path.eq_ignore_ascii_case(name))
            .map(|n| n.path.clone())
    };

    if has_stream("EncryptedPackage") || has_stream("EncryptionInfo") {
        return Err(unsupported(
            "an encrypted Office document (EncryptedPackage); decrypt it first",
        ));
    }
    // A project storage on its own: VBA/dir and PROJECT at the root.
    if has_stream("VBA/dir") && has_stream("PROJECT") {
        return Ok(ExtractedProject {
            location: ProjectLocation::RawStorage,
            bytes: bytes.to_vec(),
        });
    }
    if let Some(prefix) = storage("Macros") {
        let bytes = copy_storage(&mut cf, &nodes, &prefix, limits)?;
        return Ok(ExtractedProject {
            location: ProjectLocation::WordMacros,
            bytes,
        });
    }
    if let Some(prefix) = storage("_VBA_PROJECT_CUR") {
        let has_vba = nodes
            .iter()
            .any(|n| !n.is_stream && n.path.eq_ignore_ascii_case(&format!("{prefix}/VBA")));
        if !has_vba
            && nodes
                .iter()
                .any(|n| n.is_stream && n.path.eq_ignore_ascii_case(&format!("{prefix}/dir")))
        {
            return Err(unsupported(
                "_VBA_PROJECT_CUR holds a dir stream directly; the layout is not one this crate can verify",
            ));
        }
        let bytes = copy_storage(&mut cf, &nodes, &prefix, limits)?;
        return Ok(ExtractedProject {
            location: ProjectLocation::ExcelVbaProjectCur,
            bytes,
        });
    }
    if has_stream("PowerPoint Document") && has_stream("Current User") {
        let node_of = |name: &str| {
            nodes
                .iter()
                .find(|n| n.is_stream && n.path.eq_ignore_ascii_case(name))
                .expect("checked above")
        };
        let current = read_stream(&mut cf, node_of("Current User"), limits)?;
        let document = read_stream(&mut cf, node_of("PowerPoint Document"), limits)?;
        let (compressed, storage) = powerpoint_vba_storage(&current, &document, limits)?;
        return Ok(ExtractedProject {
            location: ProjectLocation::PowerPointVbaProjectStg { compressed },
            bytes: storage,
        });
    }
    if has_stream("WordDocument") || has_stream("Workbook") || has_stream("Book") {
        return Err(Error::NoVbaProject(
            "the document has no Macros or _VBA_PROJECT_CUR storage",
        ));
    }
    Err(unsupported(
        "the compound file is not a recognized Office document or VBA project",
    ))
}

/// Copies the storage at `prefix` (and everything below it) into a new
/// compound file whose root is that storage.
fn copy_storage(
    cf: &mut crate::read::Cfb<'_>,
    nodes: &[crate::read::Node],
    prefix: &str,
    limits: &Limits,
) -> Result<Vec<u8>, Error> {
    let mut layout = Layout::new();
    layout.version = cf.version().number();
    let mut total: u64 = 0;
    let lead = format!("{prefix}/");
    for node in nodes {
        let path_lower = node.path.to_ascii_lowercase();
        let rel = if node.path.eq_ignore_ascii_case(prefix) {
            String::new()
        } else if path_lower.starts_with(&lead.to_ascii_lowercase()) {
            node.path[lead.len()..].to_owned()
        } else {
            continue;
        };
        if node.is_stream {
            total += node.len;
            if total > limits.max_container_bytes {
                return Err(Error::InputTooLarge {
                    size: total,
                    limit: limits.max_container_bytes,
                });
            }
            layout.streams.insert(rel, read_stream(cf, node, limits)?);
        } else {
            layout.storages.insert(rel, (node.clsid, node.state_bits));
        }
    }
    assemble(layout)
}

fn be_err(detail: &'static str) -> Error {
    Error::Package(format!("PowerPoint document: {detail}"))
}

/// A PowerPoint record header (MS-PPT §2.3.1).
struct RecordHeader {
    ver: u8,
    instance: u16,
    ty: u16,
    len: u32,
}

fn record_header(data: &[u8], offset: usize) -> Result<RecordHeader, Error> {
    let b = data
        .get(
            offset
                ..offset
                    .checked_add(8)
                    .ok_or_else(|| be_err("offset overflow"))?,
        )
        .ok_or_else(|| be_err("record header outside the stream"))?;
    let vi = u16::from_le_bytes([b[0], b[1]]);
    Ok(RecordHeader {
        ver: (vi & 0xF) as u8,
        instance: vi >> 4,
        ty: u16::from_le_bytes([b[2], b[3]]),
        len: u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
    })
}

fn record_body(data: &[u8], offset: usize, h: &RecordHeader) -> Result<(usize, usize), Error> {
    let start = offset
        .checked_add(8)
        .ok_or_else(|| be_err("offset overflow"))?;
    let end = start
        .checked_add(h.len as usize)
        .filter(|&e| e <= data.len())
        .ok_or_else(|| be_err("record extends beyond the stream"))?;
    Ok((start, end))
}

fn u32_at(data: &[u8], at: usize) -> Result<u32, Error> {
    data.get(at..at.checked_add(4).ok_or_else(|| be_err("offset overflow"))?)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| be_err("field outside the record"))
}

const RT_DOCUMENT: u16 = 0x03E8;
const RT_VBA_INFO: u16 = 0x03FF;
const RT_VBA_INFO_ATOM: u16 = 0x0400;
const RT_LIST: u16 = 0x07D0;
const RT_USER_EDIT_ATOM: u16 = 0x0FF5;
const RT_CURRENT_USER_ATOM: u16 = 0x0FF6;
const RT_EXTERNAL_OLE_OBJECT_STG: u16 = 0x1011;
const RT_PERSIST_DIRECTORY_ATOM: u16 = 0x1772;
/// Most persist directory entries replayed over all user edits.
const MAX_PERSIST_ENTRIES: usize = 1 << 21;
const ENCRYPTED_TOKEN: u32 = 0xF3D1_C4DF;

/// Finds the VBA project storage of a PowerPoint binary document and returns
/// `(was compressed, storage bytes)`.
fn powerpoint_vba_storage(
    current_user: &[u8],
    doc: &[u8],
    limits: &Limits,
) -> Result<(bool, Vec<u8>), Error> {
    use std::collections::BTreeMap;

    // CurrentUserAtom (§2.3.2)
    let h = record_header(current_user, 0)?;
    if h.ty != RT_CURRENT_USER_ATOM {
        return Err(be_err(
            "Current User stream does not start with a CurrentUserAtom",
        ));
    }
    if u32_at(current_user, 12)? == ENCRYPTED_TOKEN {
        return Err(unsupported("an encrypted PowerPoint document"));
    }
    let mut edit = u32_at(current_user, 16)? as usize;

    // Part 1: walk the user edits from the newest back, collecting each
    // persist directory.
    let mut directories: Vec<usize> = Vec::new();
    let mut doc_persist_id = None;
    let mut edits_seen = 0usize;
    let mut seen_dirs = std::collections::BTreeSet::new();
    loop {
        let h = record_header(doc, edit)?;
        if h.ty != RT_USER_EDIT_ATOM || !(h.len == 0x1C || h.len == 0x20) {
            return Err(be_err("expected a UserEditAtom"));
        }
        let last_edit = u32_at(doc, edit + 16)? as usize;
        let dir_offset = u32_at(doc, edit + 20)? as usize;
        if doc_persist_id.is_none() {
            doc_persist_id = Some(u32_at(doc, edit + 24)?);
        }
        // Edits that share a directory add nothing; replaying it per edit
        // would make a long chain cost chain length times directory size.
        if seen_dirs.insert(dir_offset) {
            directories.push(dir_offset);
        }
        edits_seen += 1;
        if edits_seen > limits.max_entries {
            return Err(be_err("the user edit chain is too long"));
        }
        if last_edit == 0 {
            break;
        }
        // MUST be less than the offset of this record: also ends any loop.
        if last_edit >= edit {
            return Err(be_err("user edit chain does not move backwards"));
        }
        edit = last_edit;
    }
    let mut persist: BTreeMap<u32, usize> = BTreeMap::new();
    // Distinct directories may still overlap one another, so the total number
    // of entries replayed is capped as well (persist identifiers are 20 bits).
    let mut entries_left: usize = MAX_PERSIST_ENTRIES;
    for &dir in directories.iter().rev() {
        let h = record_header(doc, dir)?;
        if h.ty != RT_PERSIST_DIRECTORY_ATOM {
            return Err(be_err("expected a PersistDirectoryAtom"));
        }
        let (mut p, end) = record_body(doc, dir, &h)?;
        while p < end {
            let head = u32_at(doc, p)?;
            p += 4;
            let (first, count) = (head & 0xF_FFFF, (head >> 20) as usize);
            if count == 0 || p + 4 * count > end {
                return Err(be_err("malformed persist directory entry"));
            }
            entries_left = entries_left
                .checked_sub(count)
                .ok_or_else(|| be_err("the persist directories hold too many entries"))?;
            for k in 0..count {
                persist.insert(first + k as u32, u32_at(doc, p + 4 * k)? as usize);
            }
            p += 4 * count;
        }
    }

    // Part 2: the DocumentContainer.
    let doc_id = doc_persist_id.ok_or_else(|| be_err("no user edit"))?;
    let doc_offset = *persist
        .get(&doc_id)
        .ok_or_else(|| be_err("the document persist object is not in the directory"))?;
    let dh = record_header(doc, doc_offset)?;
    if dh.ty != RT_DOCUMENT || dh.ver != 0xF {
        return Err(be_err("expected a DocumentContainer"));
    }
    let (mut p, end) = record_body(doc, doc_offset, &dh)?;

    // Part 11: DocInfoListContainer -> VBAInfoContainer -> VBAInfoAtom.
    let mut vba_atom = None;
    while p < end {
        let h = record_header(doc, p)?;
        let (body, next) = record_body(doc, p, &h)?;
        if h.ty == RT_LIST && h.ver == 0xF {
            let mut q = body;
            while q < next {
                let c = record_header(doc, q)?;
                let (cbody, cnext) = record_body(doc, q, &c)?;
                if c.ty == RT_VBA_INFO && c.ver == 0xF {
                    let a = record_header(doc, cbody)?;
                    if a.ty != RT_VBA_INFO_ATOM || a.len != 0xC {
                        return Err(be_err("malformed VBAInfoContainer"));
                    }
                    let id = u32_at(doc, cbody + 8)?;
                    let has_macros = u32_at(doc, cbody + 12)?;
                    vba_atom = Some((id, has_macros));
                }
                q = cnext;
            }
        }
        p = next;
    }
    let (id, has_macros) = vba_atom.ok_or(Error::NoVbaProject(
        "the presentation has no VBAInfoContainer",
    ))?;
    if has_macros == 0 {
        return Err(Error::NoVbaProject("the VBA storage is marked empty"));
    }
    let off = *persist
        .get(&id)
        .ok_or_else(|| be_err("the VBA persist object is not in the directory"))?;
    let h = record_header(doc, off)?;
    if h.ty != RT_EXTERNAL_OLE_OBJECT_STG || h.ver != 0 {
        return Err(be_err("expected a VbaProjectStg record"));
    }
    let (body, end) = record_body(doc, off, &h)?;
    match h.instance {
        0 => Ok((false, doc[body..end].to_vec())),
        1 => {
            let declared = u64::from(u32_at(doc, body)?);
            if declared > limits.max_container_bytes {
                return Err(Error::InputTooLarge {
                    size: declared,
                    limit: limits.max_container_bytes,
                });
            }
            let compressed = doc
                .get(body + 4..end)
                .ok_or_else(|| be_err("compressed storage outside the record"))?;
            // Never expand beyond the declared size, the global limit or the
            // ratio limit, whichever is smallest.
            let cap = declared.min(
                (compressed.len().max(64) as u64).saturating_mul(limits.max_archive_ratio as u64),
            );
            let mut out = Vec::new();
            flate2::read::ZlibDecoder::new(compressed)
                .take(cap.saturating_add(1))
                .read_to_end(&mut out)
                .map_err(|e| be_err_owned(format!("invalid zlib data: {e}")))?;
            if out.len() as u64 != declared {
                return Err(be_err("decompressed size differs from the declared size"));
            }
            Ok((true, out))
        }
        _ => Err(be_err("unknown VbaProjectStg instance")),
    }
}

fn be_err_owned(msg: String) -> Error {
    Error::Package(format!("PowerPoint document: {msg}"))
}
