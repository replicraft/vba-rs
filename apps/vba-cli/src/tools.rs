//! The language tools: `lex`, `parse`, `check`, `fmt-check` and `profiles`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use vba_diagnostics::{Diagnostic, Severity, render_plain};
use vba_lexer::{LexOptions, lex};
use vba_profile::{Profile, default_profile, find_profile, profiles};
use vba_sema::{NoLibraries, Options, SourceModule, analyze_sources};
use vba_source::{SourceEncoding, SourceMap};
use vba_syntax::ParseOptions;

use crate::load::{self, LoadError};

/// The outcome of a tool: text for standard output and whether the input is
/// clean (exit code 0) or has findings (exit code 1).
pub struct Report {
    pub text: String,
    pub clean: bool,
}

/// Why a tool could not run at all.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error(transparent)]
    Load(#[from] LoadError),
    #[error("unknown profile `{0}`; `vbars profiles` lists them")]
    UnknownProfile(String),
}

/// Resolves `--profile`.
pub fn profile_arg(id: Option<&str>) -> Result<Profile, ToolError> {
    match id {
        None => Ok(*default_profile()),
        Some(id) => find_profile(id)
            .copied()
            .ok_or_else(|| ToolError::UnknownProfile(id.to_owned())),
    }
}

fn read_source(path: &Path) -> Result<Vec<u8>, LoadError> {
    let shown = path.display().to_string();
    let bytes = std::fs::read(path).map_err(|source| LoadError::Read {
        path: shown,
        source,
    })?;
    Ok(bytes)
}

fn add_file(map: &mut SourceMap, path: &Path, bytes: Vec<u8>) -> vba_source::FileId {
    let enc = if std::str::from_utf8(&bytes).is_ok() {
        SourceEncoding::Utf8
    } else {
        SourceEncoding::Unknown
    };
    map.add(path.display().to_string(), bytes, enc)
}

fn render_all(diags: &[Diagnostic], map: &SourceMap) -> String {
    let mut out = String::new();
    for d in diags {
        out.push_str(&render_plain(d, map));
        out.push('\n');
    }
    out
}

fn count_errors(diags: &[Diagnostic]) -> usize {
    diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count()
}

/// `vbars lex`: one line per token, then the lexer diagnostics.
pub fn lex_file(path: &Path, profile: &Profile, trivia: bool) -> Result<Report, ToolError> {
    let bytes = read_source(path)?;
    let mut map = SourceMap::new();
    let file = add_file(&mut map, path, bytes.clone());
    let out = lex(file, &bytes, LexOptions::new(profile.generation));
    let src = map.get(file).expect("file was just added");
    let mut text = String::new();
    for t in &out.tokens {
        if t.kind.is_trivia() && !trivia {
            continue;
        }
        let lc = src.line_col(t.span.start);
        let _ = writeln!(
            text,
            "{}:{} {:?} {:?}",
            lc.line + 1,
            lc.col + 1,
            t.kind,
            String::from_utf8_lossy(t.text)
        );
    }
    text.push_str(&render_all(&out.diagnostics, &map));
    Ok(Report {
        clean: count_errors(&out.diagnostics) == 0,
        text,
    })
}

/// `vbars parse`: a summary (or the tree) and the parser diagnostics.
pub fn parse_file(path: &Path, profile: &Profile, tree: bool) -> Result<Report, ToolError> {
    let bytes = read_source(path)?;
    let mut map = SourceMap::new();
    let file = add_file(&mut map, path, bytes.clone());
    let parsed = vba_ast::parse_module(file, &bytes, profile, &ParseOptions::strict());
    let mut text = String::new();
    if tree {
        text.push_str(&parsed.tree.debug_dump());
    } else {
        let _ = writeln!(
            text,
            "{}: {} item(s), {} node(s), {} token(s)",
            path.display(),
            parsed.module.items.len(),
            parsed.tree.node_count(),
            parsed.tree.token_count()
        );
    }
    text.push_str(&render_all(&parsed.diagnostics, &map));
    Ok(Report {
        clean: count_errors(&parsed.diagnostics) == 0,
        text,
    })
}

/// `vbars check`: the semantic analysis of one or more files as one project.
pub fn check_files(paths: &[PathBuf], profile: &Profile) -> Result<Report, ToolError> {
    let loaded = load::load(paths)?;
    let (modules, options): (Vec<SourceModule>, Options) = match &loaded.project {
        Some(p) => (
            loaded.modules,
            Options {
                project_name: p.name.clone(),
                constants: vba_sema::parse_project_constants(&p.constants),
                ..Options::default()
            },
        ),
        None => (loaded.modules, Options::default()),
    };
    let analysis = analyze_sources(&modules, profile, &NoLibraries, &options);
    let errors = count_errors(&analysis.diagnostics);
    let warnings = analysis
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    let mut text = render_all(&analysis.diagnostics, &analysis.sources);
    let _ = writeln!(
        text,
        "{} module(s), {} procedure(s): {errors} error(s), {warnings} warning(s)",
        analysis.program.modules.len(),
        analysis.program.procs.len()
    );
    Ok(Report {
        clean: errors == 0,
        text,
    })
}

/// `vbars fmt-check`: byte-exact losslessness of the syntax tree and the
/// whitespace conventions that a formatter would normalise.
pub fn fmt_check(path: &Path, profile: &Profile) -> Result<Report, ToolError> {
    let bytes = read_source(path)?;
    let mut map = SourceMap::new();
    let file = add_file(&mut map, path, bytes.clone());
    let parsed = vba_syntax::parse(file, &bytes, profile, &ParseOptions::tolerant());
    let mut findings: Vec<String> = Vec::new();
    if parsed.tree.text() != bytes {
        findings.push("the syntax tree does not reproduce the source byte for byte".into());
    }
    let (mut crlf, mut lf, mut cr) = (0usize, 0usize, 0usize);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => {
                crlf += 1;
                i += 1;
            }
            b'\r' => cr += 1,
            b'\n' => lf += 1,
            _ => {}
        }
        i += 1;
    }
    if [crlf, lf, cr].iter().filter(|n| **n > 0).count() > 1 {
        findings.push(format!(
            "mixed line endings ({crlf} CRLF, {lf} LF, {cr} CR)"
        ));
    }
    for (n, line) in bytes.split(|b| *b == b'\n').enumerate() {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.last().is_some_and(|b| *b == b' ' || *b == b'\t') {
            findings.push(format!("line {}: trailing whitespace", n + 1));
        }
        let indent: Vec<u8> = line
            .iter()
            .copied()
            .take_while(|b| *b == b' ' || *b == b'\t')
            .collect();
        if indent.contains(&b' ') && indent.contains(&b'\t') {
            findings.push(format!(
                "line {}: mixed tabs and spaces in indentation",
                n + 1
            ));
        }
    }
    if !bytes.is_empty() && !bytes.ends_with(b"\n") && !bytes.ends_with(b"\r") {
        findings.push("the file does not end with a line break".into());
    }
    let mut text = String::new();
    for f in &findings {
        let _ = writeln!(text, "{}: {f}", path.display());
    }
    if findings.is_empty() {
        let _ = writeln!(text, "{}: ok", path.display());
    }
    Ok(Report {
        clean: findings.is_empty(),
        text,
    })
}

/// `vbars profiles`.
pub fn list_profiles() -> String {
    let mut out = String::new();
    let default = default_profile().id;
    for p in profiles() {
        let _ = writeln!(
            out,
            "{:<28} {:<10} {} {}.{} ({}) {:?} {}-bit{}",
            p.id,
            p.generation.name(),
            p.host.application.id(),
            p.host.major,
            p.host.minor,
            p.host.release_year,
            p.target.os,
            p.target.bitness.bits(),
            if p.id == default { "  [default]" } else { "" }
        );
    }
    out
}
