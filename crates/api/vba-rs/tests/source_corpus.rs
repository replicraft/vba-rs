//! Runs the front end over a directory of real-world module files.
//!
//! Local only: set `VBA_SOURCE_CORPUS` to a directory holding `.bas`, `.cls`
//! and `.frm` files (each subdirectory is one project; an optional
//! `manifest.tsv` of `relative/path<TAB>kind` lines gives the module kind,
//! `standard`, `class`, `document` or `designer`). Without the variable the
//! test is skipped. See `docs/tooling/source-corpus.md`.
//!
//! Per module this checks that the syntax tree reproduces the source byte for
//! byte, that formatting is idempotent and never changes the token stream,
//! that lowering verifies, that language-server requests do not panic, and
//! that nothing takes unreasonably long. Semantic diagnostics are collected
//! and counted (set `VBA_SOURCE_CORPUS_VERBOSE` to list them) but are not
//! failures: real code has real errors.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use vba_lexer::{LexOptions, TokenKind, lex};
use vba_lsp_core::{Config, FILE_ID, FormatOptions, ModuleKind, Position, Workspace, apply_edits};
use vba_sema::{
    LibGlobal, LibMember, LibraryType, Options, SourceModule, Type, TypeProvider, analyze_sources,
};
use vba_syntax::{ParseOptions, parse};
use vba_value::Variant;

/// A module slower than this (in the unoptimized test build) is a failure;
/// override with `VBA_SOURCE_CORPUS_MAX_MS`.
const DEFAULT_MAX_MS: u64 = 10_000;
/// Slower than this is listed in the report.
const SLOW: Duration = Duration::from_millis(100);
/// Language-server positions tried per module.
const LSP_PROBES: usize = 120;

/// Every type name and every global name is "some host object": early bound
/// to nothing, so members are bound late. This isolates the front end from
/// the embedding application's object model.
struct AnyHost;

impl TypeProvider for AnyHost {
    fn resolve_type(&self, qualifier: Option<&str>, name: &str) -> Option<LibraryType> {
        if qualifier.is_none()
            && ["collection", "errobject"].contains(&name.to_ascii_lowercase().as_str())
        {
            return None;
        }
        Some(LibraryType {
            library: qualifier.unwrap_or("Host").to_owned(),
            name: name.to_owned(),
        })
    }

    fn has_library(&self, name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "excel" | "word" | "office" | "powerpoint" | "msforms" | "access" | "adodb"
        )
    }

    fn resolve_declared(&self, qualifier: Option<&str>, name: &str) -> Option<Type> {
        if vba_sema::builtins::is_vba_enum_type(name) {
            return None;
        }
        if is_host_constant_name(name) {
            // An enumeration of the embedding application.
            return Some(Type::Long);
        }
        self.resolve_type(qualifier, name).map(Type::Library)
    }

    fn default_member(&self, ty: &LibraryType) -> Option<LibMember> {
        if ty.library == "VBA" {
            return None;
        }
        Some(LibMember::property("Value", Type::Variant).default_member())
    }

    fn global(&self, name: &str) -> Option<LibGlobal> {
        if is_host_constant_name(name) {
            return Some(LibGlobal::Constant(Variant::Long(0), Type::Long));
        }
        const GLOBALS: &[&str] = &[
            "application",
            "activesheet",
            "activeworkbook",
            "activecell",
            "activewindow",
            "activedocument",
            "activepresentation",
            "range",
            "cells",
            "rows",
            "columns",
            "sheets",
            "worksheets",
            "workbooks",
            "names",
            "selection",
            "thisworkbook",
            "thisdocument",
            "documents",
            "windows",
            "charts",
            "addins",
            "commandbars",
            "dialogs",
            "union",
            "intersect",
            "evaluate",
            "wordbasic",
            "options",
            "tasks",
            "system",
            "assistant",
            "statusbar",
            "templates",
            "selectionchange",
        ];
        GLOBALS
            .contains(&name.to_ascii_lowercase().as_str())
            .then(|| LibGlobal::Object(Type::Variant))
    }
}

/// `xlUp`, `wdLine`, `msoTrue`, `fmAction...`: a constant or enumeration of
/// an application object model.
fn is_host_constant_name(name: &str) -> bool {
    let b = name.as_bytes();
    ["xl", "wd", "mso", "fm", "pp", "ac", "ad"].iter().any(|p| {
        b.len() > p.len()
            && name[..p.len()].eq_ignore_ascii_case(p)
            && b[p.len()].is_ascii_uppercase()
    })
}

struct Failure {
    module: String,
    what: String,
}

#[derive(Default)]
struct Totals {
    modules: usize,
    lines: usize,
    bytes: usize,
    lowered: usize,
    lowered_reduced: usize,
    refused: usize,
    probes: usize,
    codes: BTreeMap<String, usize>,
    failures: Vec<Failure>,
    slow: Vec<(Duration, String)>,
}

struct Mod {
    rel: String,
    name: String,
    kind: ModuleKind,
    text: String,
}

fn kind_of(s: &str) -> Option<ModuleKind> {
    Some(match s {
        "standard" => ModuleKind::Standard,
        "class" => ModuleKind::Class,
        "document" => ModuleKind::Document,
        "designer" => ModuleKind::Designer,
        _ => return None,
    })
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
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

fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => vba_format::codepage::decode(1252, bytes).unwrap_or_default(),
    }
}

/// Loads the modules under `root`, grouped by project (the directory of the
/// file relative to `root`).
fn load(root: &Path, totals: &mut Totals) -> BTreeMap<String, Vec<Mod>> {
    let mut kinds: BTreeMap<String, ModuleKind> = BTreeMap::new();
    if let Ok(m) = std::fs::read_to_string(root.join("manifest.tsv")) {
        for line in m.lines() {
            if let Some((p, k)) = line.split_once('\t')
                && let Some(k) = kind_of(k.trim())
            {
                kinds.insert(p.to_owned(), k);
            }
        }
    }
    let mut files = Vec::new();
    walk(root, &mut files);
    files.sort();
    let mut projects: BTreeMap<String, Vec<Mod>> = BTreeMap::new();
    for path in files {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if !matches!(ext.as_deref(), Some("bas" | "cls" | "frm")) {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let Ok(bytes) = std::fs::read(&path) else {
            totals.failures.push(Failure {
                module: rel,
                what: "unreadable file".into(),
            });
            continue;
        };
        let text = decode(&bytes);
        match vba_project::Module::from_source_file(file_name, &text) {
            Ok(m) => {
                let kind = kinds.get(&rel).copied().unwrap_or(m.kind);
                let project = rel.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
                projects.entry(project).or_default().push(Mod {
                    rel,
                    name: m.name,
                    kind,
                    text: m.source,
                });
            }
            Err(e) => totals.failures.push(Failure {
                module: rel,
                what: format!("cannot split into header and code: {e}"),
            }),
        }
    }
    projects
}

fn significant(src: &[u8], profile: &vba_profile::Profile) -> Vec<(TokenKind, Vec<u8>)> {
    lex(FILE_ID, src, LexOptions::for_profile(profile))
        .tokens
        .iter()
        .filter(|t| !t.kind.is_trivia())
        .map(|t| (t.kind, t.text.to_vec()))
        .collect()
}

/// Checks (a) and (c) for one module; returns the failures.
fn check_syntax(m: &Mod, profile: &vba_profile::Profile) -> Vec<String> {
    let mut bad = Vec::new();
    let src = m.text.as_bytes();
    let tolerant = parse(FILE_ID, src, profile, &ParseOptions::tolerant());
    if tolerant.tree.text() != src {
        bad.push("the syntax tree does not reproduce the source".into());
    }
    let strict = parse(FILE_ID, src, profile, &ParseOptions::strict());
    if strict.tree.text() != src {
        bad.push("the strict syntax tree does not reproduce the source".into());
    }
    // The tree is lossless under every language profile, not just this one.
    for other in vba_profile::profiles() {
        let t = parse(FILE_ID, src, other, &ParseOptions::tolerant());
        if t.tree.text() != src {
            bad.push(format!(
                "the syntax tree does not reproduce the source ({other:?})"
            ));
        }
    }
    let before = significant(src, profile);
    for opts in [
        FormatOptions::default(),
        FormatOptions {
            tab_size: 2,
            insert_spaces: false,
            keyword_case: true,
        },
    ] {
        let mut ws = Workspace::new();
        ws.open("mem:a.bas", &m.text, 1);
        let Ok(view) = ws.view("mem:a.bas") else {
            bad.push("document did not open".into());
            continue;
        };
        let once = apply_edits(&m.text, &view.format(opts));
        drop(view);
        let mut ws2 = Workspace::new();
        ws2.open("mem:a.bas", &once, 1);
        let Ok(view2) = ws2.view("mem:a.bas") else {
            continue;
        };
        let twice = apply_edits(&once, &view2.format(opts));
        if once != twice {
            bad.push(format!("formatting is not idempotent ({opts:?})"));
        }
        let after = significant(once.as_bytes(), profile);
        let (b, a) = if opts.keyword_case {
            // Only the case of reserved words may differ.
            let fold = |v: &[(TokenKind, Vec<u8>)]| {
                v.iter()
                    .map(|(k, t)| (*k, t.to_ascii_lowercase()))
                    .collect::<Vec<_>>()
            };
            (fold(&before), fold(&after))
        } else {
            (before.clone(), after)
        };
        if b != a {
            bad.push(format!("formatting changed the tokens ({opts:?})"));
        }
    }
    bad
}

/// Hover and completion at spread-out positions of every module of a project.
fn probe_lsp(mods: &[Mod], profile: vba_profile::Profile, totals: &mut Totals) {
    let mut ws = Workspace::new();
    ws.set_config(Config {
        profile,
        module_kinds: mods
            .iter()
            .map(|m| (format!("mem:{}", m.rel), m.kind))
            .collect(),
        ..Config::default()
    });
    ws.set_type_provider(Some(std::sync::Arc::new(AnyHostShared)));
    for m in mods {
        ws.open(&format!("mem:{}", m.rel), &m.text, 1);
    }
    for m in mods {
        let r = catch_unwind(AssertUnwindSafe(|| {
            let uri = format!("mem:{}", m.rel);
            let view = ws.view(&uri).expect("open");
            let toks = view.tokens();
            let step = (toks.len() / LSP_PROBES).max(1);
            let mut n = 0;
            for t in toks.iter().step_by(step) {
                let s = t.span();
                for off in [s.start, s.start + 1, s.end] {
                    let p = view.position(off.min(view.bytes().len() as u32));
                    let _ = view.hover(p);
                    let _ = view.completions(p);
                    let _ = view.signature_help(p);
                    let _ = view.definition(p);
                    n += 1;
                }
            }
            let _ = view.completions(Position {
                line: 0,
                character: 0,
            });
            let _ = view.document_symbols();
            let _ = view.folding_ranges();
            let _ = view.semantic_tokens();
            let _ = view.diagnostics();
            n
        }));
        match r {
            Ok(n) => totals.probes += n,
            Err(_) => totals.failures.push(Failure {
                module: m.rel.clone(),
                what: "panic in language-server requests".into(),
            }),
        }
    }
}

/// Lowers what remains of a project after the modules that have errors are
/// left out (repeatedly, as leaving one out can break its users). `None` if
/// nothing could be lowered; otherwise the verification result.
fn lower_without_erroneous_modules(
    sources: &[SourceModule],
    profile: &vba_profile::Profile,
    opts: &Options,
    first: &vba_sema::Analysis,
) -> Option<Result<(), String>> {
    let mut analysis = None;
    let mut kept: Vec<SourceModule> = sources.to_vec();
    for round in 0..8 {
        let current = analysis.as_ref().unwrap_or(first);
        let bad: Vec<String> = current
            .diagnostics
            .iter()
            .filter(|d| d.severity == vba_diagnostics::Severity::Error)
            .filter_map(|d| current.sources.get(d.file).map(|f| f.name().to_owned()))
            .collect();
        if bad.is_empty() {
            return None;
        }
        kept.retain(|m| !bad.iter().any(|b| b.eq_ignore_ascii_case(&m.name)));
        if kept.is_empty() {
            return None;
        }
        let next = analyze_sources(&kept, profile, &AnyHost, opts);
        match vba_ir::lower(&next) {
            Ok(p) => {
                return Some(vba_ir::verify(&p).map_err(|e| {
                    format!(
                        "IR verification failed after {round} rounds: {:?}",
                        e.first()
                    )
                }));
            }
            Err(_) => analysis = Some(next),
        }
    }
    None
}

struct AnyHostShared;

impl TypeProvider for AnyHostShared {
    fn resolve_type(&self, q: Option<&str>, n: &str) -> Option<LibraryType> {
        AnyHost.resolve_type(q, n)
    }
    fn has_library(&self, n: &str) -> bool {
        AnyHost.has_library(n)
    }
    fn global(&self, n: &str) -> Option<LibGlobal> {
        AnyHost.global(n)
    }
    fn resolve_declared(&self, q: Option<&str>, n: &str) -> Option<Type> {
        AnyHost.resolve_declared(q, n)
    }
    fn default_member(&self, t: &LibraryType) -> Option<LibMember> {
        AnyHost.default_member(t)
    }
}

#[test]
fn real_world_modules() {
    let Some(root) = std::env::var_os("VBA_SOURCE_CORPUS") else {
        eprintln!("VBA_SOURCE_CORPUS is not set; skipping");
        return;
    };
    let root = PathBuf::from(root);
    let max = Duration::from_millis(
        std::env::var("VBA_SOURCE_CORPUS_MAX_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_MS),
    );
    let verbose = std::env::var_os("VBA_SOURCE_CORPUS_VERBOSE").is_some();
    let profile = *vba_profile::default_profile();
    let mut totals = Totals::default();
    let projects = load(&root, &mut totals);

    for (project, mods) in &projects {
        // (a) and (c): syntax tree and formatter, per module.
        for m in mods {
            totals.modules += 1;
            totals.lines += m.text.lines().count();
            totals.bytes += m.text.len();
            let start = Instant::now();
            match catch_unwind(AssertUnwindSafe(|| check_syntax(m, &profile))) {
                Ok(bad) => {
                    for what in bad {
                        totals.failures.push(Failure {
                            module: m.rel.clone(),
                            what,
                        });
                    }
                }
                Err(_) => totals.failures.push(Failure {
                    module: m.rel.clone(),
                    what: "panic in lexer, parser or formatter".into(),
                }),
            }
            let took = start.elapsed();
            totals.slow.push((took, m.rel.clone()));
            if took > max {
                totals.failures.push(Failure {
                    module: m.rel.clone(),
                    what: format!("took {took:?}"),
                });
            }
        }

        // (b) and (d): the project as a whole.
        let sources: Vec<SourceModule> = mods
            .iter()
            .map(|m| SourceModule {
                name: m.name.clone(),
                kind: m.kind,
                source: m.text.clone().into_bytes(),
            })
            .collect();
        // Optional `project.txt`: the project name, then the `#Const` line.
        let info =
            std::fs::read_to_string(root.join(project).join("project.txt")).unwrap_or_default();
        let mut info = info.lines();
        let opts = Options {
            project_name: info
                .next()
                .filter(|n| !n.is_empty())
                .unwrap_or("Corpus")
                .to_owned(),
            constants: vba_sema::parse_project_constants(info.next().unwrap_or("")),
            strict_spec: std::env::var_os("VBA_SOURCE_CORPUS_STRICT_SPEC").is_some(),
            ..Options::default()
        };
        let start = Instant::now();
        let r = catch_unwind(AssertUnwindSafe(|| {
            let analysis = analyze_sources(&sources, &profile, &AnyHost, &opts);
            let lowered = vba_ir::lower(&analysis);
            let verified = match &lowered {
                Ok(p) => Some(vba_ir::verify(p)),
                Err(_) => None,
            };
            let reduced = if lowered.is_err() {
                lower_without_erroneous_modules(&sources, &profile, &opts, &analysis)
            } else {
                None
            };
            (analysis, verified, reduced)
        }));
        let took = start.elapsed();
        totals
            .slow
            .push((took, format!("{project} (whole project analysis)")));
        match r {
            Ok((analysis, verified, reduced)) => {
                for d in &analysis.diagnostics {
                    *totals.codes.entry(d.code.0.to_owned()).or_default() += 1;
                    if verbose {
                        let name = analysis
                            .sources
                            .get(d.file)
                            .map(|f| f.name().to_owned())
                            .unwrap_or_default();
                        let line = analysis
                            .sources
                            .get(d.file)
                            .map(|f| f.line_col(d.span.start).line + 1)
                            .unwrap_or(0);
                        eprintln!(
                            "DIAG {} {:?} {project}/{name}:{line} {}",
                            d.code.0, d.severity, d.message
                        );
                    }
                }
                match verified {
                    Some(Ok(())) => totals.lowered += 1,
                    Some(Err(errs)) => totals.failures.push(Failure {
                        module: project.clone(),
                        what: format!("IR verification failed: {:?}", errs.first()),
                    }),
                    None => totals.refused += 1,
                }
                match reduced {
                    Some(Ok(())) => totals.lowered_reduced += 1,
                    Some(Err(what)) => totals.failures.push(Failure {
                        module: project.clone(),
                        what,
                    }),
                    None => {}
                }
            }
            Err(_) => totals.failures.push(Failure {
                module: project.clone(),
                what: "panic in analysis or lowering".into(),
            }),
        }

        // (e): language-server requests.
        probe_lsp(mods, profile, &mut totals);
    }

    totals.slow.sort_by_key(|(d, _)| std::cmp::Reverse(*d));
    eprintln!(
        "source corpus: {} projects, {} modules, {} lines, {} bytes; \
         {} projects lowered and verified, {} more after dropping modules with errors, \
         {} not lowered (errors); {} language-server probes",
        projects.len(),
        totals.modules,
        totals.lines,
        totals.bytes,
        totals.lowered,
        totals.lowered_reduced,
        totals.refused,
        totals.probes
    );
    eprintln!("diagnostics by code: {:?}", totals.codes);
    eprintln!(
        "{} timings of {SLOW:?} or more (the test build is unoptimized)",
        totals.slow.iter().filter(|(d, _)| *d >= SLOW).count()
    );
    for (d, m) in totals.slow.iter().take(5) {
        eprintln!("slowest: {d:?} {m}");
    }
    for f in &totals.failures {
        eprintln!("FAIL {}: {}", f.module, f.what);
    }
    assert!(
        totals.failures.is_empty(),
        "{} failures",
        totals.failures.len()
    );
}
