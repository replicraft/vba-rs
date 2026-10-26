//! Writing the project model back to a `vbaProject.bin` compound file.
//!
//! # Preservation
//!
//! A project read by [`read_project`](crate::read_project) carries the
//! original bytes of everything it interpreted (see [`keys`]). On writing:
//!
//! * a stream whose modelled content did not change is emitted with its
//!   original bytes (exact preservation);
//! * a stream that must change is rebuilt: module streams keep their
//!   performance cache (or drop it, see [`StaleCache`]) and get freshly
//!   compressed source; the `dir` stream is re-serialized canonically; single
//!   mirrored lines of the `PROJECT` stream are patched in place;
//! * every other stream and storage (designers, `PROJECTwm`, `PROJECTlk`,
//!   unknown data) is emitted unchanged, with storage CLSIDs and state bits.
//!
//! Compound-file *container* bytes (sector layout, timestamps) are not
//! preserved; stream contents are.

use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use vba_project::{Module, ModuleKind, VbaProject};

use crate::codepage;
use crate::compression::compress;
use crate::dir::{DirInfo, DirModule, UnknownRecord};
use crate::error::Error;
use crate::keys;
use crate::options::{Limits, StaleCache, WriteOptions};
use crate::project_stream::{self, LineEdit};
use crate::read::{decompress_stream, module_kind};
use crate::signature::SignatureKind;

/// `_VBA_PROJECT` header with no performance cache: Reserved1 `0x61CC`,
/// Version `0xFFFF`, Reserved2 `0x00`, Reserved3 `0x0001` (§2.3.4.1, §3.1.1).
const EMPTY_VBA_PROJECT: [u8; 7] = [0xCC, 0x61, 0xFF, 0xFF, 0x00, 0x01, 0x00];

/// Timestamp stored on every entry: 2000-01-01T00:00:00Z.
fn fixed_time() -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(946_684_800)
}

/// Streams and storages to emit.
pub(crate) struct Layout {
    pub(crate) streams: BTreeMap<String, Vec<u8>>,
    pub(crate) storages: BTreeMap<String, ([u8; 16], u32)>,
    pub(crate) version: u16,
}

impl Layout {
    pub(crate) fn new() -> Self {
        Self {
            streams: BTreeMap::new(),
            storages: BTreeMap::new(),
            version: 3,
        }
    }
}

/// Writes a project to `vbaProject.bin` bytes.
///
/// If `project` was produced by [`read_project`](crate::read_project) the
/// original storage is reproduced with the model's edits applied (see the
/// module documentation for exactly what is preserved). Otherwise a new
/// project is generated from the model; designer modules cannot be generated.
///
/// Supported edits to a project read from a file: module source, module
/// metadata (description, help context, read-only, private) and project
/// metadata held in the `dir` stream, including references. Adding, removing,
/// renaming, reordering or re-typing modules, and changing the project id,
/// protection, host extenders or workspace, return
/// [`Error::UnsupportedEdit`].
pub fn write_project(project: &VbaProject, options: &WriteOptions) -> Result<Vec<u8>, Error> {
    write_project_with_report(project, options).map(|o| o.bytes)
}

/// What a write changed, for callers that must tell users about
/// consequences such as invalidated digital signatures.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WriteReport {
    /// Paths of the streams whose bytes differ from the project as read
    /// (including streams that were dropped). For a project not read from a
    /// file this lists every stream.
    pub changed_streams: Vec<String>,
    /// Paths of the digital signature streams carried by the project. They
    /// are written unchanged.
    pub signature_streams: Vec<String>,
    /// `true` if the project is signed and at least one stream changed. The
    /// signature streams are still written, but a signature over the old
    /// content no longer matches and an application will reject or ignore
    /// it. Deliberately conservative: it does not work out whether the
    /// changed streams lie inside the hashed subset of MS-OVBA §2.4.2.
    pub signatures_invalidated: bool,
}

/// The result of [`write_project_with_report`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOutput {
    /// The `vbaProject.bin` bytes.
    pub bytes: Vec<u8>,
    /// What changed.
    pub report: WriteReport,
}

/// Like [`write_project`], also reporting which streams changed and whether
/// that invalidates digital signatures the project carries.
pub fn write_project_with_report(
    project: &VbaProject,
    options: &WriteOptions,
) -> Result<WriteOutput, Error> {
    let (layout, changed_streams) = if project.preserved.contains(keys::DIR) {
        edit_layout(project, options)?
    } else {
        let layout = fresh_layout(project)?;
        let all = layout.streams.keys().cloned().collect();
        (layout, all)
    };
    let signature_streams: Vec<String> = layout
        .streams
        .keys()
        .filter(|p| SignatureKind::from_stream_name(p).is_some())
        .cloned()
        .collect();
    let signatures_invalidated = !signature_streams.is_empty() && !changed_streams.is_empty();
    Ok(WriteOutput {
        bytes: assemble(layout)?,
        report: WriteReport {
            changed_streams,
            signature_streams,
            signatures_invalidated,
        },
    })
}

fn is_srp(path: &str) -> bool {
    let mut parts = path.splitn(2, '/');
    let (storage, name) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    storage.eq_ignore_ascii_case("VBA") && name.to_ascii_uppercase().starts_with("__SRP_")
}

/// Copies the preserved, uninterpreted storages and streams into `layout`.
fn copy_preserved(project: &VbaProject, layout: &mut Layout, drop_srp: bool) -> Result<(), Error> {
    for (path, bytes) in project.preserved.iter_prefixed(keys::STORAGE_PREFIX) {
        if bytes.len() != 20 {
            return Err(Error::UnsupportedEdit(format!(
                "preserved storage entry {path:?} is malformed"
            )));
        }
        let mut clsid = [0u8; 16];
        clsid.copy_from_slice(&bytes[..16]);
        let bits = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        layout.storages.insert(path.to_owned(), (clsid, bits));
    }
    for (path, bytes) in project.preserved.iter_prefixed(keys::STREAM_PREFIX) {
        if drop_srp && is_srp(path) {
            continue;
        }
        layout.streams.insert(path.to_owned(), bytes.to_vec());
    }
    if let Some(v) = project
        .preserved
        .get(keys::CFB_VERSION)
        .and_then(|b| b.first())
    {
        layout.version = u16::from(*v);
    }
    Ok(())
}

fn edit_layout(
    project: &VbaProject,
    options: &WriteOptions,
) -> Result<(Layout, Vec<String>), Error> {
    let limits = Limits::default();
    let missing = |k: &str| Error::UnsupportedEdit(format!("preserved data {k:?} is missing"));
    let orig_dir_compressed = project
        .preserved
        .get(keys::DIR)
        .ok_or_else(|| missing(keys::DIR))?;
    let orig_dir_bytes = decompress_stream(orig_dir_compressed, "dir", &limits)?;
    let mut orig = DirInfo::parse_with(&orig_dir_bytes, true)?;
    if let Some(t) = project.preserved.get(keys::DIR_TRAILING) {
        orig.trailing = t.to_vec();
    }
    let project_raw = project
        .preserved
        .get(keys::PROJECT_STREAM)
        .ok_or_else(|| missing(keys::PROJECT_STREAM))?;
    let orig_props = project_stream::parse(project_raw, orig.code_page)?;

    // Structural edits are not supported.
    let same_shape = project.modules.len() == orig.modules.len()
        && project
            .modules
            .iter()
            .zip(&orig.modules)
            .all(|(m, d)| m.name == d.name && m.kind == module_kind(&orig_props, d));
    if !same_shape {
        return Err(Error::UnsupportedEdit(
            "adding, removing, renaming, reordering or re-typing modules is not supported".into(),
        ));
    }

    // Which modules have new source?
    let mut changed = vec![false; project.modules.len()];
    for (i, m) in project.modules.iter().enumerate() {
        let comp = m
            .preserved
            .get(keys::MODULE_COMPRESSED_SOURCE)
            .ok_or_else(|| missing(keys::MODULE_COMPRESSED_SOURCE))?;
        let (orig_has, orig_text) = if comp.is_empty() {
            (false, String::new())
        } else {
            let raw = decompress_stream(comp, &m.name, &limits)?;
            (true, codepage::decode(orig.code_page, &raw)?)
        };
        if !m.has_source && orig_has {
            return Err(Error::UnsupportedEdit(format!(
                "removing the source of module {:?} is not supported",
                m.name
            )));
        }
        if !orig_has && (m.has_source || !m.source.is_empty()) {
            // The stored module has no source container. Creating one would
            // produce text that does not correspond to the performance
            // cache (the only representation of the module's code).
            return Err(Error::UnsupportedEdit(format!(
                "module {:?} has no source in the file (compiled-only or stripped); \
                 supplying source would not match its performance cache",
                m.name
            )));
        }
        changed[i] = m.has_source != orig_has || (m.has_source && m.source != orig_text);
    }
    let any_changed = changed.iter().any(|&c| c);
    if any_changed && options.stale_cache == StaleCache::Invalidate {
        // Invalidation tells the host to recompile from source, which a
        // module without source cannot survive.
        if let Some(m) = project.modules.iter().find(|m| {
            let cached = m
                .preserved
                .get(keys::MODULE_PERFORMANCE_CACHE)
                .is_some_and(|c| !c.is_empty());
            let stripped = m
                .preserved
                .get(keys::MODULE_COMPRESSED_SOURCE)
                .is_some_and(<[u8]>::is_empty);
            cached && stripped
        }) {
            return Err(Error::UnsupportedEdit(format!(
                "editing source with StaleCache::Invalidate would force a recompilation, \
                 but module {:?} is compiled-only and has no source to recompile",
                m.name
            )));
        }
    }
    let invalidate = any_changed && options.stale_cache == StaleCache::Invalidate;

    // Apply the model to the dir records.
    let mut info = orig.clone();
    apply_model(&mut info, project);
    let decode_unknown = |bytes: Option<&[u8]>| -> Result<Vec<UnknownRecord>, Error> {
        bytes.map_or(Ok(Vec::new()), |b| {
            UnknownRecord::decode_list(b).ok_or_else(|| {
                Error::UnsupportedEdit("preserved unknown dir records are malformed".into())
            })
        })
    };
    info.unknown = decode_unknown(project.preserved.get(keys::DIR_UNKNOWN))?;
    for (dm, m) in info.modules.iter_mut().zip(&project.modules) {
        dm.unknown = decode_unknown(m.preserved.get(keys::MODULE_DIR_UNKNOWN))?;
    }
    for (i, dm) in info.modules.iter_mut().enumerate() {
        if changed[i] && invalidate {
            dm.text_offset = 0;
        }
    }

    let mut layout = Layout {
        streams: BTreeMap::new(),
        storages: BTreeMap::new(),
        version: 3,
    };
    copy_preserved(project, &mut layout, invalidate)?;

    let vba = vba_storage_name(project);
    let dir_bytes = if info == orig {
        orig_dir_compressed.to_vec()
    } else {
        compress(&info.serialize()?)
    };
    layout.streams.insert(format!("{vba}/dir"), dir_bytes);

    let vp = project
        .preserved
        .get(keys::VBA_PROJECT_STREAM)
        .ok_or_else(|| missing(keys::VBA_PROJECT_STREAM))?;
    layout.streams.insert(
        format!("{vba}/_VBA_PROJECT"),
        if invalidate {
            EMPTY_VBA_PROJECT.to_vec()
        } else {
            vp.to_vec()
        },
    );

    layout.streams.insert(
        "PROJECT".into(),
        patched_project_stream(project, &orig, &orig_props, project_raw)?,
    );

    for (i, m) in project.modules.iter().enumerate() {
        let dm = &info.modules[i];
        let cache = m
            .preserved
            .get(keys::MODULE_PERFORMANCE_CACHE)
            .unwrap_or_default();
        let mut stream = Vec::new();
        if changed[i] {
            if !(changed[i] && invalidate) {
                stream.extend_from_slice(cache);
            }
            let encoded = codepage::encode(
                info.code_page,
                &m.source,
                &format!("source of module {:?}", m.name),
            )?;
            stream.extend(compress(&encoded));
        } else {
            stream.extend_from_slice(cache);
            stream.extend_from_slice(
                m.preserved
                    .get(keys::MODULE_COMPRESSED_SOURCE)
                    .unwrap_or_default(),
            );
        }
        debug_assert!(stream.len() >= dm.text_offset as usize);
        layout
            .streams
            .insert(format!("{vba}/{}", dm.stream_name), stream);
    }

    // Which streams differ from the project as read?
    let mut original: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    original.insert(format!("{vba}/dir"), orig_dir_compressed.to_vec());
    original.insert(format!("{vba}/_VBA_PROJECT"), vp.to_vec());
    original.insert("PROJECT".into(), project_raw.to_vec());
    for (m, dm) in project.modules.iter().zip(&orig.modules) {
        let mut bytes = m
            .preserved
            .get(keys::MODULE_PERFORMANCE_CACHE)
            .unwrap_or_default()
            .to_vec();
        bytes.extend_from_slice(
            m.preserved
                .get(keys::MODULE_COMPRESSED_SOURCE)
                .unwrap_or_default(),
        );
        original.insert(format!("{vba}/{}", dm.stream_name), bytes);
    }
    for (path, bytes) in project.preserved.iter_prefixed(keys::STREAM_PREFIX) {
        original.insert(path.to_owned(), bytes.to_vec());
    }
    let mut changed: Vec<String> = original
        .iter()
        .filter(|(path, bytes)| layout.streams.get(*path) != Some(*bytes))
        .map(|(path, _)| path.clone())
        .collect();
    changed.extend(
        layout
            .streams
            .keys()
            .filter(|p| !original.contains_key(*p))
            .cloned(),
    );
    changed.sort();
    Ok((layout, changed))
}

/// The name under which the VBA storage appears in the preserved data.
fn vba_storage_name(project: &VbaProject) -> String {
    project
        .preserved
        .iter_prefixed(keys::STORAGE_PREFIX)
        .map(|(p, _)| p)
        .find(|p| p.eq_ignore_ascii_case("VBA"))
        .unwrap_or("VBA")
        .to_owned()
}

/// Copies the model's dir-level fields over `info`.
fn apply_model(info: &mut DirInfo, p: &VbaProject) {
    info.sys_kind = p.sys_kind.value();
    info.compat_version = p.compat_version;
    info.lcid = p.lcid;
    info.lcid_invoke = p.lcid_invoke;
    info.name = p.name.clone();
    info.doc_string = p.description.clone();
    info.help_file = p.help_file.clone().unwrap_or_default();
    info.help_context = p.help_context;
    info.lib_flags = p.lib_flags;
    info.version_major = p.version.major;
    info.version_minor = p.version.minor;
    info.constants = if p.constants.is_empty() {
        info.constants.as_ref().map(|_| String::new())
    } else {
        Some(p.constants.clone())
    };
    info.references = p.references.clone();
    for (dm, m) in info.modules.iter_mut().zip(&p.modules) {
        dm.doc_string = m.doc_string.clone();
        dm.help_context = m.help_context;
        dm.read_only = m.read_only;
        dm.private = m.private;
    }
}

/// Returns the PROJECT stream bytes: the original, with mirrored properties
/// patched when the model's values differ from the original `dir` values.
fn patched_project_stream(
    project: &VbaProject,
    orig: &DirInfo,
    props: &project_stream::ProjectProps,
    raw: &[u8],
) -> Result<Vec<u8>, Error> {
    let unsupported = |what: &str| {
        Err(Error::UnsupportedEdit(format!(
            "changing the {what} is not supported for a project read from a file"
        )))
    };
    if project.project_id != props.id {
        return unsupported("project id");
    }
    if project.protection != props.protection {
        return unsupported("protection state");
    }
    if project.host_extenders != props.host_extenders {
        return unsupported("host extenders");
    }
    if project.workspace != props.workspace {
        return unsupported("workspace");
    }
    let help_file = project.help_file.clone().unwrap_or_default();
    let help_context = project.help_context.to_string();
    let mut edits = Vec::new();
    if project.name != orig.name {
        edits.push(LineEdit {
            key: "Name",
            value: &project.name,
        });
    }
    if project.description != orig.doc_string {
        edits.push(LineEdit {
            key: "Description",
            value: &project.description,
        });
    }
    if help_file != orig.help_file {
        edits.push(LineEdit {
            key: "HelpFile",
            value: &help_file,
        });
    }
    if project.help_context != orig.help_context {
        edits.push(LineEdit {
            key: "HelpContextID",
            value: &help_context,
        });
    }
    if edits.is_empty() {
        Ok(raw.to_vec())
    } else {
        project_stream::patch(raw, orig.code_page, &edits)
    }
}

/// Generates a layout for a project that was not read from a file.
fn fresh_layout(project: &VbaProject) -> Result<Layout, Error> {
    let mut seen = std::collections::BTreeSet::new();
    for m in &project.modules {
        if !seen.insert(m.name.to_ascii_uppercase()) {
            return Err(Error::UnsupportedEdit(format!(
                "duplicate module name {:?}",
                m.name
            )));
        }
        if m.name.is_empty() || m.name.contains(['/', '\\', ':', '!']) {
            return Err(Error::UnsupportedEdit(format!(
                "module name {:?} cannot be used as a stream name",
                m.name
            )));
        }
    }
    let info = DirInfo {
        sys_kind: project.sys_kind.value(),
        compat_version: project.compat_version,
        lcid: project.lcid,
        lcid_invoke: project.lcid_invoke,
        code_page: project.code_page,
        name: project.name.clone(),
        doc_string: project.description.clone(),
        help_file: project.help_file.clone().unwrap_or_default(),
        help_context: project.help_context,
        lib_flags: project.lib_flags,
        version_major: project.version.major,
        version_minor: project.version.minor,
        constants: (!project.constants.is_empty()).then(|| project.constants.clone()),
        references: project.references.clone(),
        modules: project.modules.iter().map(fresh_dir_module).collect(),
        trailing: Vec::new(),
        unknown: Vec::new(),
    };
    let mut layout = Layout {
        streams: BTreeMap::new(),
        storages: BTreeMap::new(),
        version: 3,
    };
    copy_preserved(project, &mut layout, false)?;
    layout
        .streams
        .insert("VBA/dir".into(), compress(&info.serialize()?));
    layout
        .streams
        .insert("VBA/_VBA_PROJECT".into(), EMPTY_VBA_PROJECT.to_vec());
    layout
        .streams
        .insert("PROJECT".into(), project_stream::generate(project)?);
    for m in &project.modules {
        let stream = if m.has_source {
            let encoded = codepage::encode(
                project.code_page,
                &m.source,
                &format!("source of module {:?}", m.name),
            )?;
            compress(&encoded)
        } else {
            Vec::new()
        };
        layout.streams.insert(format!("VBA/{}", m.name), stream);
    }
    Ok(layout)
}

fn fresh_dir_module(m: &Module) -> DirModule {
    DirModule {
        name: m.name.clone(),
        has_unicode_name: true,
        stream_name: m.name.clone(),
        doc_string: m.doc_string.clone(),
        text_offset: 0,
        help_context: m.help_context,
        procedural: m.kind == ModuleKind::Standard,
        read_only: m.read_only,
        private: m.private,
        unknown: Vec::new(),
    }
}

/// Builds the compound file.
pub(crate) fn assemble(layout: Layout) -> Result<Vec<u8>, Error> {
    let err = |e: std::io::Error| Error::ContainerWrite(e.to_string());
    let version = cfb::Version::from_number(layout.version).unwrap_or(cfb::Version::V3);
    let mut cf =
        cfb::CompoundFile::create_with_version(version, Cursor::new(Vec::new())).map_err(err)?;

    // Parents before children.
    let mut storages: Vec<_> = layout.storages.iter().collect();
    storages.sort_by_key(|(p, _)| (p.matches('/').count(), p.to_ascii_uppercase()));
    for (path, (clsid, bits)) in storages {
        let full = format!("/{path}");
        if !path.is_empty() {
            cf.create_storage_all(&full).map_err(err)?;
        }
        cf.set_storage_clsid(&full, uuid::Uuid::from_bytes(*clsid))
            .map_err(err)?;
        cf.set_state_bits(&full, *bits).map_err(err)?;
    }
    for (path, data) in &layout.streams {
        if let Some((parent, _)) = path.rsplit_once('/') {
            let parent = format!("/{parent}");
            if !cf.is_storage(&parent) {
                cf.create_storage_all(&parent).map_err(err)?;
            }
        }
        let mut s = cf.create_stream(format!("/{path}")).map_err(err)?;
        s.write_all(data).map_err(err)?;
    }
    // Fixed timestamps keep the output byte-for-byte reproducible.
    let paths: Vec<String> = cf
        .walk()
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect();
    for path in paths {
        cf.set_created_time(&path, fixed_time()).map_err(err)?;
        cf.set_modified_time(&path, fixed_time()).map_err(err)?;
    }
    cf.flush().map_err(err)?;
    Ok(cf.into_inner().into_inner())
}
