//! Shared helpers for tests that use a local, uncommitted project named by
//! the `VBA_PCODE_PROJECT` environment variable. Nothing here embeds project
//! content.

#![allow(dead_code, missing_docs)]

pub mod consistency;
pub mod expr;
pub mod synth;

use vba_format::{Limits, read_project};
use vba_lexer::{LexOptions, TokenKind, lex};
use vba_profile::LanguageGeneration;
use vba_source::FileId;

/// Environment variable naming the local project file (never committed).
pub const PROJECT_ENV: &str = "VBA_PCODE_PROJECT";

pub struct Mod {
    pub name: String,
    pub cache: Vec<u8>,
    pub source: Vec<u8>,
}

pub struct Project {
    /// True for a stream set without a `dir` stream (compiled-only): the
    /// modules carry a cache and no source.
    pub compiled_only: bool,
    pub sys_kind: u32,
    pub project_stream: Vec<u8>,
    pub modules: Vec<Mod>,
}

/// Loads the local project, or `None` when it is not configured or absent.
pub fn load() -> Option<Project> {
    load_path(std::path::Path::new(&std::env::var_os(PROJECT_ENV)?))
}

/// Environment variable naming a directory of local project files (never
/// committed), for the multi-layout runs.
pub const CORPUS_ENV: &str = "VBA_PCODE_CORPUS";

/// Environment variable naming a directory of per-project stream folders
/// (never committed): every folder below it that holds `VBA/_VBA_PROJECT` is
/// one project, given as raw streams (`VBA/dir`, `VBA/<module>`, ...).
pub const STREAMS_ENV: &str = "VBA_PCODE_STREAMS";

/// The files below the directory named by `VBA_PCODE_CORPUS` and the stream
/// folders below `VBA_PCODE_STREAMS`, sorted; empty when both are unset.
pub fn corpus_files() -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    if let Some(dir) = std::env::var_os(CORPUS_ENV) {
        walk(std::path::Path::new(&dir), &mut v);
    }
    if let Some(dir) = std::env::var_os(STREAMS_ENV) {
        fn folders(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            if dir.join("VBA").join("_VBA_PROJECT").is_file() {
                out.push(dir.to_path_buf());
                return;
            }
            let Ok(rd) = std::fs::read_dir(dir) else {
                return;
            };
            for e in rd.flatten() {
                if e.path().is_dir() {
                    folders(&e.path(), out);
                }
            }
        }
        folders(std::path::Path::new(&dir), &mut v);
    }
    v.sort();
    v
}

/// Loads a folder of raw project streams. With a `dir` stream the project is
/// read as usual; without one (compiled-only) the module streams are taken
/// as caches and the platform kind comes from the word at offset 2 of the
/// first module.
fn load_streams(dir: &std::path::Path) -> Option<Project> {
    let vba = dir.join("VBA");
    let project_stream = std::fs::read(vba.join("_VBA_PROJECT")).ok()?;
    if vba.join("dir").is_file() {
        let mut set = vba_format::ProjectStreams::new();
        for e in std::fs::read_dir(&vba).ok()?.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            set.insert(
                format!("VBA/{}", unescape(&name)),
                std::fs::read(e.path()).ok()?,
            );
        }
        set.insert("PROJECT", std::fs::read(dir.join("PROJECT")).ok()?);
        let p = vba_format::read_project_from_streams(
            &set,
            vba_format::ReadMode::Tolerant,
            &Limits::default(),
        )
        .ok()?;
        return Some(project_of(&p, project_stream, false));
    }
    let mut modules = Vec::new();
    let mut names: Vec<_> = std::fs::read_dir(&vba).ok()?.flatten().collect();
    names.sort_by_key(|e| e.file_name());
    for e in names {
        let name = e.file_name().to_string_lossy().into_owned();
        if name == "_VBA_PROJECT" {
            continue;
        }
        modules.push(Mod {
            name,
            cache: std::fs::read(e.path()).ok()?,
            source: Vec::new(),
        });
    }
    let sys_kind = u32::from(modules.first()?.cache.get(2).copied()?);
    Some(Project {
        compiled_only: true,
        sys_kind,
        project_stream,
        modules,
    })
}

/// Undoes the `%XX` escapes of stream folder file names.
fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Some(v) = std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Loads one project file or stream folder, or `None` when it cannot be read.
pub fn load_path(path: &std::path::Path) -> Option<Project> {
    if path.is_dir() {
        return load_streams(path);
    }
    let bytes = std::fs::read(path).ok()?;
    let ext = vba_format::extract_project(&bytes, &Limits::default()).ok()?;
    let p = match read_project(&ext.bytes, &Limits::default()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("read_project: {e}");
            return None;
        }
    };
    let project_stream = p
        .preserved
        .get(vba_format::keys::VBA_PROJECT_STREAM)?
        .to_vec();
    Some(project_of(&p, project_stream, false))
}

fn project_of(
    p: &vba_project::VbaProject,
    project_stream: Vec<u8>,
    compiled_only: bool,
) -> Project {
    let modules = p
        .modules
        .iter()
        .filter_map(|m| {
            Some(Mod {
                name: m.name.clone(),
                cache: m
                    .preserved
                    .get(vba_project::PERFORMANCE_CACHE_KEY)?
                    .to_vec(),
                source: vba_format::codepage::encode(p.code_page, &m.source, "source").ok()?,
            })
        })
        .collect();
    Project {
        compiled_only,
        sys_kind: p.sys_kind.value(),
        project_stream,
        modules,
    }
}

/// A significant (non-trivia) source token.
#[derive(Debug, Clone)]
pub struct Tok {
    pub kind: TokenKind,
    pub text: Vec<u8>,
}

/// One logical source line (continuations joined).
#[derive(Debug, Clone, Default)]
pub struct SrcLine {
    pub toks: Vec<Tok>,
    /// Comment text without the leading apostrophe, if any.
    pub comment: Option<Vec<u8>>,
    /// Raw text of the logical line.
    pub raw: Vec<u8>,
}

/// Splits module source into logical lines, skipping the
/// `Attribute` lines (they have no code in the cache).
pub fn logical_lines(source: &[u8]) -> Vec<SrcLine> {
    let out = lex(
        FileId(0),
        source,
        LexOptions::new(LanguageGeneration::Vba7_0),
    );
    let mut lines: Vec<SrcLine> = vec![SrcLine::default()];
    for t in &out.tokens {
        let cur = lines.last_mut().unwrap();
        match t.kind {
            TokenKind::Newline => {
                lines.push(SrcLine::default());
                continue;
            }
            TokenKind::Comment => {
                cur.raw.extend_from_slice(t.text);
                cur.comment = Some(t.text[1..].to_vec());
            }
            TokenKind::RemComment => {
                cur.raw.extend_from_slice(t.text);
                cur.comment = Some(t.text.get(3..).unwrap_or(&[]).to_vec());
            }
            k if k.is_trivia() => cur.raw.extend_from_slice(t.text),
            k => {
                cur.raw.extend_from_slice(t.text);
                cur.toks.push(Tok {
                    kind: k,
                    text: t.text.to_vec(),
                });
            }
        }
    }
    // Drop a trailing empty line produced by the final newline.
    if lines
        .last()
        .is_some_and(|l| l.raw.is_empty() && l.toks.is_empty())
    {
        lines.pop();
    }
    // `Attribute` lines (leading ones and those after a procedure header)
    // have no line in the cache.
    lines.retain(|l| !l.raw.starts_with(b"Attribute "));
    lines
}
