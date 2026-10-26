//! Shared helpers for tests that use a local, uncommitted project named by
//! the `VBA_PCODE_PROJECT` environment variable. Nothing here embeds project
//! content.

#![allow(dead_code, missing_docs)]

pub mod consistency;
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
    pub sys_kind: u32,
    pub project_stream: Vec<u8>,
    pub modules: Vec<Mod>,
}

/// Loads the local project, or `None` when it is not configured or absent.
pub fn load() -> Option<Project> {
    let path = std::env::var_os(PROJECT_ENV)?;
    let bytes = std::fs::read(path).ok()?;
    let p = read_project(&bytes, &Limits::default()).expect("local project reads");
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
                cache: m
                    .preserved
                    .get(vba_project::PERFORMANCE_CACHE_KEY)?
                    .to_vec(),
                source: vba_format::codepage::encode(p.code_page, &m.source, "source")
                    .expect("source re-encodes"),
            })
        })
        .collect();
    Some(Project {
        sys_kind: p.sys_kind.value(),
        project_stream,
        modules,
    })
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

/// Splits module source into logical lines, skipping the leading
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
    let skip = lines
        .iter()
        .take_while(|l| l.raw.starts_with(b"Attribute "))
        .count();
    lines.drain(..skip);
    lines
}
