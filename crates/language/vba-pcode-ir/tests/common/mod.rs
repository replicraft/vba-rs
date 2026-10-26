//! Helpers for the tests that use a local, uncommitted project (named by the
//! environment variable `VBA_PCODE_PROJECT`). Nothing here embeds project
//! content.

#![allow(dead_code, missing_docs)]

pub mod host;
pub mod synth;

use vba_format::{Limits, read_project};
use vba_project::ModuleKind;
use vba_sema::SourceModule;

pub struct Mod {
    pub name: String,
    pub kind: vba_project::ModuleKind,
    pub cache: Vec<u8>,
    pub source: Vec<u8>,
}

pub struct Project {
    pub sys_kind: u32,
    pub project_stream: Vec<u8>,
    pub modules: Vec<Mod>,
}

/// Loads the project named by `VBA_PCODE_PROJECT`, or `None` when unset or
/// unreadable.
pub fn load() -> Option<Project> {
    let path = std::env::var_os("VBA_PCODE_PROJECT")?;
    let bytes = std::fs::read(path).ok()?;
    let p = read_project(&bytes, &Limits::default()).ok()?;
    let project_stream = p
        .preserved
        .get(vba_format::keys::VBA_PROJECT_STREAM)?
        .to_vec();
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
    Some(Project {
        sys_kind: p.sys_kind.value(),
        project_stream,
        modules,
    })
}

/// Splits module source into logical lines (continuations joined), skipping
/// the leading `Attribute` lines, which have no code in the cache. Each
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
    let skip = lines
        .iter()
        .take_while(|l| l.starts_with(b"Attribute "))
        .count();
    lines.drain(..skip);
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
