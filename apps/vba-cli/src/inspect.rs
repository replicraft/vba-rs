//! The `inspect` subcommand.

use std::fmt::Write as _;
use std::path::Path;

use vba_format::Limits;
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
    /// The file is neither a compound file nor a supported source file.
    #[error(
        "{path} is not a recognized VBA file (expected a vbaProject.bin compound file or a .bas, .cls or .frm source file)"
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

/// Inspects `path` and returns the report to print.
pub fn run(path: &Path) -> Result<String, InspectError> {
    let shown = path.display().to_string();
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
        return Ok(report_source(&module));
    }
    if bytes.starts_with(&CFB_MAGIC) {
        let project = vba_format::read_project(&bytes, &Limits::default()).map_err(|source| {
            InspectError::Project {
                path: shown,
                source,
            }
        })?;
        return Ok(report_project(&project));
    }
    Err(InspectError::Unrecognized { path: shown })
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
        return "no source".to_owned();
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

fn report_project(p: &VbaProject) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Project: {}", p.name);
    if !p.description.is_empty() {
        let _ = writeln!(out, "Description: {}", p.description);
    }
    let _ = writeln!(out, "Code page: {}", p.code_page);
    let _ = writeln!(out, "Platform: {}", sys_kind_name(p.sys_kind));
    let _ = writeln!(out, "Version: {}.{}", p.version.major, p.version.minor);
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
    let stripped = p.modules.iter().filter(|m| !m.has_source).count();
    if stripped > 0 {
        let _ = writeln!(
            out,
            "Note: {stripped} module(s) have no source text (stripped or compiled-only); their code cannot be shown."
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
