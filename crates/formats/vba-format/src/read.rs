//! Reading a `vbaProject.bin` compound file into the project model.

use std::collections::BTreeSet;
use std::io::{Cursor, Read};

use vba_project::{Module, ModuleKind, SysKind, VbaProject, VersionInfo};

use crate::codepage;
use crate::compression::decompress;
use crate::dir::{DirInfo, DirModule, UnknownRecord};
use crate::error::Error;
use crate::keys;
use crate::options::{Limits, ReadMode};
use crate::project_stream::{self, ProjectProps};
use crate::streams::ProjectStreams;

/// Size of the version-independent header of the `_VBA_PROJECT` stream
/// (§2.3.4.1).
pub(crate) const VBA_PROJECT_HEADER_LEN: usize = 7;

pub(crate) type Cfb<'a> = cfb::CompoundFile<Cursor<&'a [u8]>>;

/// A storage or stream found in the compound file.
pub(crate) struct Node {
    /// Path without a leading separator, using `/`.
    pub path: String,
    pub is_stream: bool,
    pub len: u64,
    pub clsid: [u8; 16],
    pub state_bits: u32,
}

/// Longest accepted path of a storage or stream, in bytes.
const MAX_PATH_BYTES: usize = 4096;
/// Most bytes of path text accepted over all objects.
const MAX_TOTAL_PATH_BYTES: usize = 16 << 20;

/// Opens the compound file and lists every object in it.
pub(crate) fn open_and_list<'a>(
    bytes: &'a [u8],
    limits: &Limits,
) -> Result<(Cfb<'a>, Vec<Node>), Error> {
    if bytes.len() as u64 > limits.max_container_bytes {
        return Err(Error::InputTooLarge {
            size: bytes.len() as u64,
            limit: limits.max_container_bytes,
        });
    }
    let cf =
        cfb::CompoundFile::open(Cursor::new(bytes)).map_err(|e| Error::Container(e.to_string()))?;
    let mut nodes = Vec::new();
    let mut path_bytes = 0usize;
    for entry in cf.walk() {
        if nodes.len() >= limits.max_entries {
            return Err(Error::TooManyEntries {
                limit: limits.max_entries,
            });
        }
        let path = entry.path().to_string_lossy().replace('\\', "/");
        // A chain of nested storages makes every path as long as the nesting
        // is deep, so the paths alone would take the square of the depth in
        // time and memory. Names are at most 31 characters; real documents
        // nest a handful of levels.
        path_bytes += path.len();
        if path.len() > MAX_PATH_BYTES || path_bytes > MAX_TOTAL_PATH_BYTES {
            return Err(Error::Container(
                "storages are nested too deeply".to_owned(),
            ));
        }
        nodes.push(Node {
            path: path.trim_start_matches('/').to_owned(),
            is_stream: entry.is_stream(),
            len: entry.len(),
            clsid: *entry.clsid().as_bytes(),
            state_bits: entry.state_bits(),
        });
    }
    Ok((cf, nodes))
}

fn find<'n>(nodes: &'n [Node], path: &str, stream: bool) -> Option<&'n Node> {
    nodes
        .iter()
        .find(|n| n.is_stream == stream && n.path.eq_ignore_ascii_case(path))
}

pub(crate) fn read_stream(
    cf: &mut Cfb<'_>,
    node: &Node,
    limits: &Limits,
) -> Result<Vec<u8>, Error> {
    if node.len > limits.max_stream_bytes as u64 {
        return Err(Error::StreamTooLarge {
            path: node.path.clone(),
            size: node.len,
            limit: limits.max_stream_bytes,
        });
    }
    let read_err = |e: std::io::Error| Error::StreamRead {
        path: node.path.clone(),
        message: e.to_string(),
    };
    let mut stream = cf
        .open_stream(format!("/{}", node.path))
        .map_err(read_err)?;
    let mut buf = Vec::new();
    // The directory entry's length is untrusted; cap the read regardless.
    (&mut stream)
        .take(limits.max_stream_bytes as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(read_err)?;
    if buf.len() > limits.max_stream_bytes {
        return Err(Error::StreamTooLarge {
            path: node.path.clone(),
            size: buf.len() as u64,
            limit: limits.max_stream_bytes,
        });
    }
    Ok(buf)
}

pub(crate) fn decompress_stream(
    data: &[u8],
    stream: &str,
    limits: &Limits,
) -> Result<Vec<u8>, Error> {
    decompress(data, limits.decompressed_cap(data.len())).map_err(|source| Error::Decompress {
        stream: stream.to_owned(),
        source,
    })
}

/// Determines each module's kind from the PROJECT stream, falling back to the
/// `dir` stream's MODULETYPE (which cannot tell class from document/designer).
pub(crate) fn module_kind(props: &ProjectProps, m: &DirModule) -> ModuleKind {
    props.kind_of(&m.name).unwrap_or(if m.procedural {
        ModuleKind::Standard
    } else {
        ModuleKind::Class
    })
}

/// Reads a `vbaProject.bin` compound file into a [`VbaProject`].
///
/// All streams and storages the reader does not interpret, and the original
/// bytes of the ones it does, are attached to the returned model as
/// [`Preserved`](vba_project::Preserved) data (see [`keys`]), so
/// [`write_project`](crate::write_project) can reproduce them.
///
/// Malformed input yields an [`Error`]; this function does not panic on bad
/// data. Projects whose module streams hold no source (stripped or
/// compiled-only) are read with `has_source == false` on those modules.
pub fn read_project(bytes: &[u8], limits: &Limits) -> Result<VbaProject, Error> {
    read_project_with(bytes, limits, ReadMode::Strict)
}

/// Like [`read_project`], with a choice of [`ReadMode`].
///
/// In [`ReadMode::Tolerant`], `dir` records with identifiers outside the
/// MS-OVBA grammar are kept as opaque blobs in the model's `Preserved` slots
/// (see [`keys::DIR_UNKNOWN`] and [`keys::MODULE_DIR_UNKNOWN`]) and are
/// written back unchanged. Everything else is as strict as in
/// [`ReadMode::Strict`].
pub fn read_project_with(
    bytes: &[u8],
    limits: &Limits,
    mode: ReadMode,
) -> Result<VbaProject, Error> {
    let (mut cf, nodes) = open_and_list(bytes, limits)?;
    find(&nodes, "VBA", false).ok_or_else(|| Error::MissingStream { path: "VBA".into() })?;
    // Directory entries may share sectors, so the sum of the declared lengths
    // can exceed the file. Refuse before reading anything rather than after
    // holding the multiplied data.
    let declared: u64 = nodes
        .iter()
        .filter(|n| n.is_stream)
        .fold(0u64, |a, n| a.saturating_add(n.len));
    if declared > limits.max_container_bytes {
        return Err(Error::InputTooLarge {
            size: declared,
            limit: limits.max_container_bytes,
        });
    }
    let mut streams = ProjectStreams::new();
    for node in nodes.iter().filter(|n| n.is_stream) {
        streams.insert(&node.path, read_stream(&mut cf, node, limits)?);
    }
    let mut project = read_project_from_streams(&streams, mode, limits)?;
    let p = &mut project.preserved;
    p.insert(keys::CFB_VERSION, vec![cf.version().number() as u8]);
    for node in nodes.iter().filter(|n| !n.is_stream) {
        let mut v = node.clsid.to_vec();
        v.extend_from_slice(&node.state_bits.to_le_bytes());
        p.insert(format!("{}{}", keys::STORAGE_PREFIX, node.path), v);
    }
    Ok(project)
}

/// Reads a project from a set of named streams (`VBA/dir`,
/// `VBA/_VBA_PROJECT`, `VBA/<module>`, `PROJECT`, and any others such as
/// `PROJECTwm` or `VBA/__SRP_n`), without any container.
///
/// This is the code path [`read_project_with`] uses once it has collected the
/// streams of a compound file, so the result is identical to reading a
/// compound file holding the same streams, except that it carries no storage
/// entries (CLSIDs, state bits) or compound-file version, because a stream
/// set has none. Paths are located case-insensitively. Limits apply to the
/// stream count, each stream, their total, and decompression.
///
/// Missing required streams, oversized streams and malformed content are
/// reported as [`Error`]s.
pub fn read_project_from_streams(
    streams: &ProjectStreams,
    mode: ReadMode,
    limits: &Limits,
) -> Result<VbaProject, Error> {
    if streams.len() > limits.max_entries {
        return Err(Error::TooManyEntries {
            limit: limits.max_entries,
        });
    }
    let mut total: u64 = 0;
    for (path, data) in streams.iter() {
        if data.len() > limits.max_stream_bytes {
            return Err(Error::StreamTooLarge {
                path: path.to_owned(),
                size: data.len() as u64,
                limit: limits.max_stream_bytes,
            });
        }
        total += data.len() as u64;
        if total > limits.max_container_bytes {
            return Err(Error::InputTooLarge {
                size: total,
                limit: limits.max_container_bytes,
            });
        }
    }
    let required = |path: &str| -> Result<(&str, &[u8]), Error> {
        streams.entry(path).ok_or_else(|| Error::MissingStream {
            path: path.to_owned(),
        })
    };
    let (dir_path, dir_compressed) = required("VBA/dir")?;
    let vba_prefix = dir_path.split('/').next().unwrap_or("VBA").to_owned();
    let (vp_path, vp_raw) = required("VBA/_VBA_PROJECT")?;
    let (project_path, project_raw) = required("PROJECT")?;

    let dir_bytes = decompress_stream(dir_compressed, dir_path, limits)?;
    let info = DirInfo::parse_with(&dir_bytes, mode == ReadMode::Tolerant)?;
    // Module records may name one stream many times. Everything the model
    // is about to hold on their behalf (stream copies, decompressed text,
    // decoded text) is charged here, so the memory kept is bounded by the
    // container limit however many records there are.
    let mut held = dir_bytes.len() as u64;
    let mut charge = |n: usize| -> Result<(), Error> {
        held = held.saturating_add(n as u64);
        if held > limits.max_container_bytes {
            return Err(Error::InputTooLarge {
                size: held,
                limit: limits.max_container_bytes,
            });
        }
        Ok(())
    };

    let props = project_stream::parse(project_raw, info.code_page)?;

    if vp_raw.len() < VBA_PROJECT_HEADER_LEN {
        return Err(Error::VbaProjectTooShort { len: vp_raw.len() });
    }

    let mut project = VbaProject::new(info.name.clone());
    project.description = info.doc_string.clone();
    project.code_page = info.code_page;
    project.sys_kind = SysKind::from_value(info.sys_kind);
    project.lcid = info.lcid;
    project.lcid_invoke = info.lcid_invoke;
    project.compat_version = info.compat_version;
    project.help_file = (!info.help_file.is_empty()).then(|| info.help_file.clone());
    project.help_context = info.help_context;
    project.lib_flags = info.lib_flags;
    project.version = VersionInfo {
        major: info.version_major,
        minor: info.version_minor,
    };
    project.constants = info.constants.clone().unwrap_or_default();
    project.project_id = props.id.clone();
    project.protection = props.protection;
    project.host_extenders = props.host_extenders.clone();
    project.workspace = props.workspace.clone();
    project.references = info.references.clone();

    let mut consumed: BTreeSet<String> = BTreeSet::new();
    let mut spellings = vec![
        dir_path.to_owned(),
        vp_path.to_owned(),
        project_path.to_owned(),
    ];
    let mut canonical = vec![
        "VBA/dir".to_owned(),
        "VBA/_VBA_PROJECT".to_owned(),
        "PROJECT".to_owned(),
    ];
    for path in [dir_path, vp_path, project_path] {
        consumed.insert(path.to_ascii_uppercase());
    }
    for dm in &info.modules {
        let path = format!("{vba_prefix}/{}", dm.stream_name);
        let (node_path, data) = required(&path)?;
        consumed.insert(node_path.to_ascii_uppercase());
        spellings.push(node_path.to_owned());
        canonical.push(format!("VBA/{}", dm.stream_name));
        let offset = dm.text_offset as usize;
        if offset > data.len() {
            return Err(Error::ModuleOffsetOutOfRange {
                module: dm.name.clone(),
                offset: dm.text_offset,
                len: data.len(),
            });
        }
        charge(data.len())?;
        let (cache, compressed) = data.split_at(offset);
        let (source, has_source) = if compressed.is_empty() {
            (String::new(), false)
        } else {
            let raw = decompress_stream(compressed, node_path, limits)?;
            charge(raw.len())?;
            let text = codepage::decode(info.code_page, &raw)?;
            charge(text.len())?;
            (text, true)
        };
        let mut module = Module::new(dm.name.clone(), module_kind(&props, dm), source);
        module.has_source = has_source;
        module.doc_string = dm.doc_string.clone();
        module.help_context = dm.help_context;
        module.read_only = dm.read_only;
        module.private = dm.private;
        module
            .preserved
            .insert(keys::MODULE_PERFORMANCE_CACHE, cache.to_vec());
        module
            .preserved
            .insert(keys::MODULE_COMPRESSED_SOURCE, compressed.to_vec());
        if !dm.unknown.is_empty() {
            module.preserved.insert(
                keys::MODULE_DIR_UNKNOWN,
                UnknownRecord::encode_list(&dm.unknown),
            );
        }
        project.modules.push(module);
    }

    let p = &mut project.preserved;
    p.insert(keys::DIR, dir_compressed.to_vec());
    p.insert(keys::PROJECT_STREAM, project_raw.to_vec());
    p.insert(keys::VBA_PROJECT_STREAM, vp_raw.to_vec());
    if !info.trailing.is_empty() {
        p.insert(keys::DIR_TRAILING, info.trailing.clone());
    }
    if !info.unknown.is_empty() {
        p.insert(keys::DIR_UNKNOWN, UnknownRecord::encode_list(&info.unknown));
    }
    if spellings.iter().zip(&canonical).any(|(a, b)| a != b) {
        p.insert(keys::STREAM_SPELLINGS, spellings.join("\n").into_bytes());
    }
    for (path, data) in streams.iter() {
        if !consumed.contains(&path.to_ascii_uppercase()) {
            p.insert(format!("{}{}", keys::STREAM_PREFIX, path), data.to_vec());
        }
    }
    Ok(project)
}
