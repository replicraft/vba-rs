//! Reading the files a command works on.

use std::path::{Path, PathBuf};

use vba_format::Limits;
use vba_project::{Module, VbaProject};
use vba_sema::SourceModule;

/// Compound file signature.
pub const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// Why a file could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// The file could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// The file is not a VBA source file or project.
    #[error(
        "{path} is not a recognized VBA file (expected a .bas, .cls or .frm source file or a vbaProject.bin compound file)"
    )]
    Unrecognized { path: String },
    /// A project file could not be parsed.
    #[error("{path}: {source}")]
    Project {
        path: String,
        #[source]
        source: vba_format::Error,
    },
    /// A source file could not be split into header and code.
    #[error("{path}: {source}")]
    Source {
        path: String,
        #[source]
        source: vba_project::StandaloneError,
    },
    /// A project has modules without source text (compiled-only or
    /// stripped); compiled state is never analysed or run.
    #[error(
        "{path}: project {project:?} is {availability}: no source for module(s) {}; \
         compiled-only projects can be inspected but not parsed, checked or run",
        modules.join(", ")
    )]
    CompiledOnly {
        path: String,
        project: String,
        availability: &'static str,
        modules: Vec<String>,
    },
    /// The text is neither UTF-8 nor Windows-1252.
    #[error("{path}: not valid UTF-8 or Windows-1252 text")]
    Encoding { path: String },
}

/// What the files named on the command line add up to.
pub struct Loaded {
    /// A project file, when one was given (then it is the only input).
    pub project: Option<VbaProject>,
    /// The path of the project file, when there is one.
    pub project_path: Option<String>,
    /// The modules to analyse.
    pub modules: Vec<SourceModule>,
    /// Where each module came from, parallel to `modules`: the file path, or
    /// `project[Module]` for a module inside a project file.
    pub origins: Vec<String>,
}

impl Loaded {
    /// The error for a project with modules that lack source, if it is one.
    pub fn compiled_only_error(&self) -> Option<LoadError> {
        let p = self.project.as_ref()?;
        let availability = p.source_availability();
        if availability == vba_project::ProjectSourceAvailability::FullSource {
            return None;
        }
        Some(LoadError::CompiledOnly {
            path: self.project_path.clone().unwrap_or_default(),
            project: p.name.clone(),
            availability: availability.name(),
            modules: p.modules_without_source().map(|m| m.name.clone()).collect(),
        })
    }
}

/// Decodes a source file: UTF-8 if valid, otherwise Windows-1252.
pub fn decode_source(bytes: &[u8]) -> Option<String> {
    match std::str::from_utf8(bytes) {
        Ok(s) => Some(s.to_owned()),
        Err(_) => vba_format::codepage::decode(1252, bytes).ok(),
    }
}

/// Reads one `.bas`, `.cls` or `.frm` file as a module.
pub fn read_module(path: &Path) -> Result<Module, LoadError> {
    let shown = path.display().to_string();
    let bytes = std::fs::read(path).map_err(|source| LoadError::Read {
        path: shown.clone(),
        source,
    })?;
    let text = decode_source(&bytes).ok_or_else(|| LoadError::Encoding {
        path: shown.clone(),
    })?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    Module::from_source_file(file_name, &text).map_err(|source| LoadError::Source {
        path: shown,
        source,
    })
}

/// Loads source files, or a single `vbaProject.bin`.
pub fn load(paths: &[PathBuf]) -> Result<Loaded, LoadError> {
    let mut modules = Vec::new();
    let mut origins = Vec::new();
    for path in paths {
        let shown = path.display().to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if matches!(ext.as_deref(), Some("bas" | "cls" | "frm")) {
            let m = read_module(path)?;
            modules.push(SourceModule {
                name: m.name.clone(),
                kind: m.kind,
                source: m.source.clone().into_bytes(),
            });
            origins.push(shown);
            continue;
        }
        let bytes = std::fs::read(path).map_err(|source| LoadError::Read {
            path: shown.clone(),
            source,
        })?;
        if bytes.starts_with(&CFB_MAGIC) {
            let project =
                vba_format::read_project(&bytes, &Limits::default()).map_err(|source| {
                    LoadError::Project {
                        path: shown.clone(),
                        source,
                    }
                })?;
            return Ok(Loaded {
                origins: project
                    .modules
                    .iter()
                    .map(|m| format!("{shown}[{}]", m.name))
                    .collect(),
                modules: project
                    .modules
                    .iter()
                    .map(|m| SourceModule {
                        name: m.name.clone(),
                        kind: m.kind,
                        source: m.source.clone().into_bytes(),
                    })
                    .collect(),
                project: Some(project),
                project_path: Some(shown),
            });
        }
        return Err(LoadError::Unrecognized { path: shown });
    }
    Ok(Loaded {
        project: None,
        project_path: None,
        modules,
        origins,
    })
}
