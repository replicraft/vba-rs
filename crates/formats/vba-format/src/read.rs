//! Reading a `vbaProject.bin` compound file into the project model.

use std::collections::BTreeSet;
use std::io::{Cursor, Read};

use vba_project::{Module, ModuleKind, SysKind, VbaProject, VersionInfo};

use crate::codepage;
use crate::compression::decompress;
use crate::dir::{DirInfo, DirModule};
use crate::error::Error;
use crate::keys;
use crate::options::Limits;
use crate::project_stream::{self, ProjectProps};

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
    for entry in cf.walk() {
        if nodes.len() >= limits.max_entries {
            return Err(Error::TooManyEntries {
                limit: limits.max_entries,
            });
        }
        let path = entry.path().to_string_lossy().replace('\\', "/");
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

fn read_stream(cf: &mut Cfb<'_>, node: &Node, limits: &Limits) -> Result<Vec<u8>, Error> {
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
    props
        .modules
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(&m.name))
        .map(|(k, _)| *k)
        .unwrap_or(if m.procedural {
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
    let (mut cf, nodes) = open_and_list(bytes, limits)?;

    let vba =
        find(&nodes, "VBA", false).ok_or_else(|| Error::MissingStream { path: "VBA".into() })?;
    let vba_prefix = vba.path.clone();
    let required = |path: String| -> Result<&Node, Error> {
        find(&nodes, &path, true).ok_or(Error::MissingStream { path })
    };
    let dir_node = required(format!("{vba_prefix}/dir"))?;
    let vp_node = required(format!("{vba_prefix}/_VBA_PROJECT"))?;
    let project_node = required("PROJECT".into())?;

    let dir_compressed = read_stream(&mut cf, dir_node, limits)?;
    let dir_bytes = decompress_stream(&dir_compressed, &dir_node.path, limits)?;
    let info = DirInfo::parse(&dir_bytes)?;

    let project_raw = read_stream(&mut cf, project_node, limits)?;
    let props = project_stream::parse(&project_raw, info.code_page)?;

    let vp_raw = read_stream(&mut cf, vp_node, limits)?;
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
    for n in [dir_node, vp_node, project_node] {
        consumed.insert(n.path.to_ascii_uppercase());
    }
    for dm in &info.modules {
        let path = format!("{vba_prefix}/{}", dm.stream_name);
        let node = find(&nodes, &path, true).ok_or(Error::MissingStream { path })?;
        consumed.insert(node.path.to_ascii_uppercase());
        let data = read_stream(&mut cf, node, limits)?;
        let offset = dm.text_offset as usize;
        if offset > data.len() {
            return Err(Error::ModuleOffsetOutOfRange {
                module: dm.name.clone(),
                offset: dm.text_offset,
                len: data.len(),
            });
        }
        let (cache, compressed) = data.split_at(offset);
        let (source, has_source) = if compressed.is_empty() {
            (String::new(), false)
        } else {
            let raw = decompress_stream(compressed, &node.path, limits)?;
            (codepage::decode(info.code_page, &raw)?, true)
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
        project.modules.push(module);
    }

    let p = &mut project.preserved;
    p.insert(keys::DIR, dir_compressed);
    p.insert(keys::PROJECT_STREAM, project_raw);
    p.insert(keys::VBA_PROJECT_STREAM, vp_raw);
    if !info.trailing.is_empty() {
        p.insert(keys::DIR_TRAILING, info.trailing.clone());
    }
    p.insert(keys::CFB_VERSION, vec![cf.version().number() as u8]);

    for node in &nodes {
        if node.is_stream {
            if consumed.contains(&node.path.to_ascii_uppercase()) {
                continue;
            }
            let data = read_stream(&mut cf, node, limits)?;
            p.insert(format!("{}{}", keys::STREAM_PREFIX, node.path), data);
        } else {
            let mut v = node.clsid.to_vec();
            v.extend_from_slice(&node.state_bits.to_le_bytes());
            p.insert(format!("{}{}", keys::STORAGE_PREFIX, node.path), v);
        }
    }
    Ok(project)
}
