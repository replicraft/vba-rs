//! The `inspect` subcommand.

use std::fmt::Write as _;
use std::path::Path;

use vba_format::{Limits, ProjectStreams};
use vba_project::{Module, SysKind, VbaProject};

/// Compound file signature.
const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// Why inspection failed.
#[derive(Debug, thiserror::Error)]
pub enum InspectError {
    /// The file could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// A directory of exported streams could not be read.
    #[error("{path}: {message}")]
    Directory { path: String, message: String },
    /// The file is neither a compound file nor a supported source file.
    #[error(
        "{path} is not a recognized VBA file (expected a vbaProject.bin compound file, a directory of exported streams, or a .bas, .cls or .frm source file)"
    )]
    Unrecognized { path: String },
    /// A project file could not be parsed.
    #[error("{path}: {source}")]
    Project {
        path: String,
        #[source]
        source: vba_format::Error,
    },
    /// A source file could not be parsed.
    #[error("{path}: {source}")]
    Source {
        path: String,
        #[source]
        source: vba_project::StandaloneError,
    },
    /// A source file is not text in a supported encoding.
    #[error("{path}: not valid UTF-8 or Windows-1252 text")]
    Encoding { path: String },
}

/// Version of the `--json` output layout (see `docs/formats/vba-format.md`).
pub const JSON_SCHEMA_VERSION: u32 = 1;

/// Inspects `path` and returns the report to print: text, or JSON when
/// `json` is set.
pub fn run(path: &Path, json: bool) -> Result<String, InspectError> {
    let shown = path.display().to_string();
    if path.is_dir() {
        let streams = read_stream_dir(path, &shown)?;
        let project = vba_format::read_project_from_streams(
            &streams,
            vba_format::ReadMode::Strict,
            &Limits::default(),
        )
        .map_err(|source| InspectError::Project {
            path: shown,
            source,
        })?;
        return Ok(render_project(&project, "directory", json));
    }
    let bytes = std::fs::read(path).map_err(|source| InspectError::Read {
        path: shown.clone(),
        source,
    })?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if matches!(ext.as_deref(), Some("bas" | "cls" | "frm")) {
        let text = decode_source(&bytes).ok_or_else(|| InspectError::Encoding {
            path: shown.clone(),
        })?;
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let module =
            Module::from_source_file(file_name, &text).map_err(|source| InspectError::Source {
                path: shown.clone(),
                source,
            })?;
        return Ok(if json {
            json_source(&module)
        } else {
            report_source(&module)
        });
    }
    if bytes.starts_with(&CFB_MAGIC) {
        let project = vba_format::read_project(&bytes, &Limits::default()).map_err(|source| {
            InspectError::Project {
                path: shown,
                source,
            }
        })?;
        return Ok(render_project(&project, "compound-file", json));
    }
    Err(InspectError::Unrecognized { path: shown })
}

/// Reads a directory of exported streams: every regular file below `dir` is a
/// stream whose path is its path relative to `dir`, with `/` between
/// components. Symbolic links are not followed.
fn read_stream_dir(dir: &Path, shown: &str) -> Result<ProjectStreams, InspectError> {
    let limits = Limits::default();
    let fail = |message: String| InspectError::Directory {
        path: shown.to_owned(),
        message,
    };
    let mut streams = ProjectStreams::new();
    let mut pending = vec![(dir.to_path_buf(), String::new())];
    let mut files = 0usize;
    while let Some((current, prefix)) = pending.pop() {
        let entries = std::fs::read_dir(&current)
            .map_err(|e| fail(format!("cannot read directory {}: {e}", current.display())))?;
        for entry in entries {
            let entry = entry.map_err(|e| fail(e.to_string()))?;
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or_else(|| fail(format!("file name {name:?} is not valid UTF-8")))?;
            let rel = format!("{prefix}{name}");
            let kind = entry.file_type().map_err(|e| fail(e.to_string()))?;
            if kind.is_dir() {
                pending.push((entry.path(), format!("{rel}/")));
            } else if kind.is_file() {
                files += 1;
                if files > limits.max_entries {
                    return Err(fail(format!("more than {} files", limits.max_entries)));
                }
                let len = entry.metadata().map_err(|e| fail(e.to_string()))?.len();
                if len > limits.max_stream_bytes as u64 {
                    return Err(fail(format!(
                        "{rel}: {len} bytes exceeds the stream limit of {}",
                        limits.max_stream_bytes
                    )));
                }
                let bytes = std::fs::read(entry.path())
                    .map_err(|e| fail(format!("cannot read {rel}: {e}")))?;
                streams.insert(rel, bytes);
            }
        }
    }
    Ok(streams)
}

/// Decodes a source file: UTF-8 if valid, otherwise Windows-1252.
fn decode_source(bytes: &[u8]) -> Option<String> {
    match std::str::from_utf8(bytes) {
        Ok(s) => Some(s.to_owned()),
        Err(_) => vba_format::codepage::decode(1252, bytes).ok(),
    }
}

fn describe_size(m: &Module) -> String {
    if !m.has_source {
        return match m.performance_cache_len() {
            0 => "no source, no cache [absent, no cache]".to_owned(),
            n => format!("no source, {n}-byte compiled cache [compiled-only]"),
        };
    }
    if m.source_is_ambiguous() {
        return "0 chars, 0 lines [empty source, with compiled cache; ambiguous]".to_owned();
    }
    let lines = m.source.lines().count();
    format!("{} chars, {} lines", m.source.chars().count(), lines)
}

fn report_source(m: &Module) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Standalone source file");
    let _ = writeln!(out, "Module: {}", m.name);
    let _ = writeln!(out, "Kind: {}", m.kind.name());
    let _ = writeln!(out, "Size: {}", describe_size(m));
    let _ = writeln!(
        out,
        "Export header: {}",
        match &m.file_header {
            Some(h) => format!("{} lines", h.lines().count()),
            None => "none".to_owned(),
        }
    );
    out
}

fn sys_kind_name(k: SysKind) -> String {
    match k {
        SysKind::Win16 => "16-bit Windows".to_owned(),
        SysKind::Win32 => "32-bit Windows".to_owned(),
        SysKind::Macintosh => "Macintosh".to_owned(),
        SysKind::Win64 => "64-bit Windows".to_owned(),
        SysKind::Other(v) => format!("unknown ({v})"),
    }
}

fn render_project(p: &VbaProject, input: &str, json: bool) -> String {
    if json {
        json_project(p, input)
    } else {
        report_project(p)
    }
}

fn hex_word(w: u16) -> String {
    format!("{w:#06X}")
}

fn version_pair(v: Option<(u32, u32)>) -> Option<String> {
    v.map(|(a, b)| format!("{a:X}.{b:X}"))
}

fn json_source(m: &Module) -> String {
    let value = serde_json::json!({
        "schema_version": JSON_SCHEMA_VERSION,
        "kind": "source",
        "module": {
            "name": m.name,
            "kind": m.kind.name(),
            "has_source": m.has_source,
            "chars": m.source.chars().count(),
            "lines": m.source.lines().count(),
            "export_header_lines": m.file_header.as_ref().map(|h| h.lines().count()),
        },
    });
    format!("{value:#}\n")
}

fn json_project(p: &VbaProject, input: &str) -> String {
    let v = p.project_version();
    let pr = &p.protection;
    let modules: Vec<_> = p
        .modules
        .iter()
        .map(|m| {
            serde_json::json!({
                "name": m.name,
                "kind": m.kind.name(),
                "has_source": m.has_source,
                "source_availability": m.source_availability().name(),
                "chars": m.source.chars().count(),
                "lines": m.source.lines().count(),
                "performance_cache_bytes": m.performance_cache_len(),
            })
        })
        .collect();
    let references: Vec<_> = p
        .references
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name(),
                "kind": r.kind_name(),
                "libid": r.libid(),
            })
        })
        .collect();
    let hints: Vec<_> = v
        .reference_hints
        .iter()
        .map(|h| {
            serde_json::json!({
                "reference": h.reference,
                "name": h.name,
                "libid": h.libid,
                "guid": h.guid,
                "type_lib_version": version_pair(h.type_lib_version),
                "lcid": h.lcid,
                "path": h.path,
                "office_dir": h.office_dir,
            })
        })
        .collect();
    let value = serde_json::json!({
        "schema_version": JSON_SCHEMA_VERSION,
        "kind": "project",
        "input": input,
        "name": p.name,
        "description": p.description,
        "code_page": v.code_page,
        "lcid": v.lcid,
        "platform": {
            "name": sys_kind_name(v.sys_kind),
            "value": v.sys_kind.value(),
        },
        "project_version": { "major": p.version.major, "minor": p.version.minor },
        "version_word": v.version_word,
        "version_word_hex": v.version_word.map(hex_word),
        "profile_hint": v.profile_hint(),
        "reference_hints": hints,
        "compiled_only": v.compiled_only,
        "source_availability": p.source_availability().name(),
        "protection": {
            "user_protected": pr.user_protected,
            "host_protected": pr.host_protected,
            "vbe_protected": pr.vbe_protected,
            "has_password": pr.has_password,
            "visible": pr.visible,
        },
        "references": references,
        "modules": modules,
    });
    format!("{value:#}\n")
}

fn report_project(p: &VbaProject) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Project: {}", p.name);
    if !p.description.is_empty() {
        let _ = writeln!(out, "Description: {}", p.description);
    }
    let _ = writeln!(out, "Code page: {}", p.code_page);
    let _ = writeln!(out, "Platform: {}", sys_kind_name(p.sys_kind));
    let _ = writeln!(out, "Version: {}.{}", p.version.major, p.version.minor);
    let ver = p.project_version();
    let _ = writeln!(
        out,
        "Version word: {}",
        ver.version_word.map_or_else(|| "none".to_owned(), hex_word)
    );
    let _ = writeln!(
        out,
        "Profile hint: {}",
        ver.profile_hint()
            .map_or_else(|| "unknown".to_owned(), |h| format!("{h} (hypothesis)"))
    );
    if let Some(h) = &p.help_file {
        let _ = writeln!(out, "Help file: {h}");
    }
    if !p.constants.is_empty() {
        let _ = writeln!(out, "Constants: {}", p.constants);
    }

    let pr = &p.protection;
    if pr.is_locked() || !pr.visible {
        let mut parts = Vec::new();
        if pr.user_protected {
            parts.push("protected by user");
        }
        if pr.host_protected {
            parts.push("protected by host");
        }
        if pr.vbe_protected {
            parts.push("locked for viewing");
        }
        if pr.has_password {
            parts.push("password set");
        }
        if !pr.visible {
            parts.push("not visible");
        }
        let _ = writeln!(out, "Protection: {}", parts.join(", "));
        let _ = writeln!(
            out,
            "Note: protection is a declared flag; module source is stored unencrypted and is shown below."
        );
    } else {
        let _ = writeln!(out, "Protection: none");
    }

    let _ = writeln!(out, "References ({}):", p.references.len());
    for r in &p.references {
        let _ = writeln!(
            out,
            "  {} [{}] {}",
            r.name().unwrap_or("(unnamed)"),
            r.kind_name(),
            r.libid()
        );
    }

    if !ver.reference_hints.is_empty() {
        let _ = writeln!(out, "Reference version hints (raw facts, not conclusions):");
        for h in &ver.reference_hints {
            let _ = writeln!(
                out,
                "  {} guid={} typelib={} office-dir={} path={}",
                h.name.as_deref().unwrap_or("(unnamed)"),
                h.guid.as_deref().unwrap_or("-"),
                version_pair(h.type_lib_version).unwrap_or_else(|| "-".to_owned()),
                h.office_dir
                    .map_or_else(|| "-".to_owned(), |n| n.to_string()),
                h.path.as_deref().unwrap_or("-"),
            );
        }
    }

    let _ = writeln!(out, "Modules ({}):", p.modules.len());
    let width = p
        .modules
        .iter()
        .map(|m| m.name.chars().count())
        .max()
        .unwrap_or(0);
    for m in &p.modules {
        let _ = writeln!(
            out,
            "  {:<width$}  {:<8}  {}",
            m.name,
            m.kind.name(),
            describe_size(m)
        );
    }
    let availability = p.source_availability();
    let _ = writeln!(out, "Source availability: {}", availability.name());
    let lacking: Vec<&str> = p.modules_without_source().map(|m| &*m.name).collect();
    if !lacking.is_empty() {
        let _ = writeln!(
            out,
            "Note: no source text for {} module(s): {}. Their code exists only as \
             compiled state, which is preserved byte for byte but never decompiled \
             or run; such a project cannot be parsed, checked or run.",
            lacking.len(),
            lacking.join(", ")
        );
    }
    let ambiguous: Vec<&str> = p
        .modules
        .iter()
        .filter(|m| m.source_is_ambiguous())
        .map(|m| &*m.name)
        .collect();
    if !ambiguous.is_empty() {
        let _ = writeln!(
            out,
            "Note: the source state of {} is ambiguous (see docs/runtime/compiled-projects.md).",
            ambiguous.join(", ")
        );
    }
    let _ = writeln!(
        out,
        "Preserved: {} entries, {} bytes",
        p.preserved.len() + p.modules.iter().map(|m| m.preserved.len()).sum::<usize>(),
        p.preserved.total_bytes()
            + p.modules
                .iter()
                .map(|m| m.preserved.total_bytes())
                .sum::<usize>()
    );
    out
}
