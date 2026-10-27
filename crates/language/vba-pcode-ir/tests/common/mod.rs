//! Helpers for the tests that use a local, uncommitted project (named by the
//! environment variable `VBA_PCODE_PROJECT`). Nothing here embeds project
//! content.

#![allow(dead_code, missing_docs)]

pub mod host;
pub mod synth;

use vba_format::{Limits, read_project};
use vba_project::ModuleKind;
use vba_sema::SourceModule;

/// Environment variable naming the local project file (never committed).
pub const PROJECT_ENV: &str = "VBA_PCODE_PROJECT";

pub struct Mod {
    pub name: String,
    pub kind: vba_project::ModuleKind,
    pub cache: Vec<u8>,
    pub source: Vec<u8>,
}

pub struct Project {
    /// True for a stream set without a `dir` stream (compiled-only).
    pub compiled_only: bool,
    pub code_page: u16,
    pub name: String,
    pub sys_kind: u32,
    pub project_stream: Vec<u8>,
    pub modules: Vec<Mod>,
}

/// Loads the project named by `VBA_PCODE_PROJECT`, or `None` when unset or
/// unreadable.
pub fn load() -> Option<Project> {
    load_path(std::path::Path::new(&std::env::var_os(PROJECT_ENV)?))
}

/// Environment variable naming a directory of local project files (never
/// committed), for the multi-layout runs.
pub const CORPUS_ENV: &str = "VBA_PCODE_CORPUS";

/// Environment variable naming a directory of per-project stream folders
/// (never committed): every folder below it that holds `VBA/_VBA_PROJECT` is
/// one project, given as raw streams.
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

/// Loads a folder of raw project streams (see the `vba-pcode` test helper).
/// Compiled-only folders (no `dir`) have no source and are not lowered.
fn load_streams(dir: &std::path::Path) -> Option<Project> {
    let vba = dir.join("VBA");
    let project_stream = std::fs::read(vba.join("_VBA_PROJECT")).ok()?;
    if !vba.join("dir").is_file() {
        return None;
    }
    let mut set = vba_format::ProjectStreams::new();
    for e in std::fs::read_dir(&vba).ok()?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        set.insert(format!("VBA/{}", unescape(&name)), std::fs::read(e.path()).ok()?);
    }
    set.insert("PROJECT", std::fs::read(dir.join("PROJECT")).ok()?);
    let p = vba_format::read_project_from_streams(
        &set,
        vba_format::ReadMode::Tolerant,
        &Limits::default(),
    )
    .ok()?;
    Some(project_of(&p, project_stream))
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
    let p = read_project(&ext.bytes, &Limits::default()).ok()?;
    let project_stream = p
        .preserved
        .get(vba_format::keys::VBA_PROJECT_STREAM)?
        .to_vec();
    Some(project_of(&p, project_stream))
}

fn project_of(p: &vba_project::VbaProject, project_stream: Vec<u8>) -> Project {
    let modules = p
        .modules
        .iter()
        .filter_map(|m| {
            Some(Mod {
                name: m.name.clone(),
                kind: m.kind,
                cache: m
                    .preserved
                    .get(vba_project::PERFORMANCE_CACHE_KEY)?
                    .to_vec(),
                source: vba_format::codepage::encode(p.code_page, &m.source, "source").ok()?,
            })
        })
        .collect();
    Project {
        compiled_only: false,
        code_page: p.code_page,
        name: p.name.clone(),
        sys_kind: p.sys_kind.value(),
        project_stream,
        modules,
    }
}

/// Splits module source into logical lines (continuations joined), skipping
/// the `Attribute` lines, which have no line in the cache. Each
/// entry is the raw text of the line.
pub fn logical_lines(source: &[u8]) -> Vec<Vec<u8>> {
    use vba_lexer::{LexOptions, TokenKind, lex};
    let out = lex(
        vba_source::FileId(0),
        source,
        LexOptions::new(vba_profile::LanguageGeneration::Vba7_0),
    );
    let mut lines: Vec<Vec<u8>> = vec![Vec::new()];
    let mut has_token: Vec<bool> = vec![false];
    for t in &out.tokens {
        if t.kind == TokenKind::Newline {
            lines.push(Vec::new());
            has_token.push(false);
            continue;
        }
        if let (Some(cur), Some(h)) = (lines.last_mut(), has_token.last_mut()) {
            cur.extend_from_slice(t.text);
            *h |= !t.kind.is_trivia();
        }
    }
    if lines.last().is_some_and(Vec::is_empty) && has_token.last() == Some(&false) {
        lines.pop();
    }
    // `Attribute` lines (leading ones and those after a procedure header)
    // have no line in the cache.
    lines.retain(|l| !l.starts_with(b"Attribute "));
    lines
}

/// The role the embedding application gives a module: forms and reports
/// are document-like (their unresolved names are members of the host).
pub fn role(m: &Mod) -> ModuleKind {
    if m.kind != ModuleKind::Standard
        && (m.name.starts_with("Form_") || m.name.starts_with("Report_"))
    {
        ModuleKind::Document
    } else {
        m.kind
    }
}

/// The names the embedding application would list as members of its
/// documents: the undeclared names that the source analysis of the
/// document-like modules reports.
pub fn host_member_names(p: &Project) -> Vec<String> {
    let mods: Vec<SourceModule> = p
        .modules
        .iter()
        .map(|m| SourceModule {
            name: m.name.clone(),
            kind: role(m),
            source: m.source.clone(),
        })
        .collect();
    let a = vba_sema::analyze_sources(
        &mods,
        vba_profile::default_profile(),
        &vba_sema::NoLibraries,
        &vba_sema::Options::default(),
    );
    let mut names = Vec::new();
    for d in &a.diagnostics {
        if let Some(rest) = d.message.strip_prefix("variable `")
            && let Some(n) = rest.strip_suffix("` is not defined (Option Explicit)")
            && !names.iter().any(|x: &String| x.eq_ignore_ascii_case(n))
        {
            names.push(n.to_owned());
        }
    }
    names
}
