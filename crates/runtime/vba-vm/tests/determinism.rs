//! Determinism over the conformance corpus: every program under `tests/vba`
//! runs twice in fresh VMs and must produce identical output, an identical
//! result and an identical step count, and must never panic or report an
//! internal error.
//!
//! Set `VBA_STEP_DIGEST=<file>` to also write one line per program (steps,
//! output checksum, result class). Comparing two digests shows whether a
//! change altered step accounting or behaviour anywhere in the corpus.

mod common;

use std::path::{Path, PathBuf};
use std::rc::Rc;

use common::module;
use vba_host::{HeadlessHost, Host, ResourceLimits};
use vba_project::ModuleKind;
use vba_sema::SourceModule;
use vba_vm::{BufferSink, NoServices, Vm, VmConfig, VmError};

const DEFAULT_MAX_STEPS: u64 = 3_000_000;

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e == "bas") {
            out.push(p);
        }
    }
}

/// Splits a conformance program into its modules and reads `ENTRY` and
/// `MAX_STEPS`.
fn parse(text: &str) -> (Vec<SourceModule>, String, u64) {
    let mut entry = "Main".to_owned();
    let mut max_steps = DEFAULT_MAX_STEPS;
    let mut mods: Vec<(String, ModuleKind, String)> = Vec::new();
    let mut current = ("Module1".to_owned(), ModuleKind::Standard, String::new());
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("'#MODULE ") {
            let mut it = rest.split_whitespace();
            let name = it.next().unwrap_or("Module1").to_owned();
            let kind = match it.next() {
                Some("class") => ModuleKind::Class,
                Some("document") => ModuleKind::Document,
                _ => ModuleKind::Standard,
            };
            let prev = std::mem::replace(&mut current, (name, kind, String::new()));
            let blank = prev
                .2
                .lines()
                .all(|l| l.trim().is_empty() || l.trim_start().starts_with('\''));
            if !(blank && mods.is_empty() && prev.0 == "Module1") {
                mods.push(prev);
            }
            continue;
        }
        if let Some(v) = line.strip_prefix("' ENTRY: ") {
            entry = v.trim().to_owned();
        }
        if let Some(v) = line.strip_prefix("' MAX_STEPS: ") {
            max_steps = v.trim().parse().unwrap_or(max_steps);
        }
        current.2.push_str(line);
        current.2.push('\n');
    }
    mods.push(current);
    let mods = mods
        .into_iter()
        .map(|(n, k, s)| module(&n, k, &s))
        .collect();
    (mods, entry, max_steps)
}

struct Run {
    out: String,
    result: String,
    steps: u64,
}

/// `None` when the program does not compile (several corpus programs are
/// about diagnostics).
fn execute(mods: &[SourceModule], entry: &str, max_steps: u64) -> Option<Run> {
    let a = vba_sema::analyze_sources(
        mods,
        vba_profile::default_profile(),
        &vba_sema::NoLibraries,
        &vba_sema::Options::default(),
    );
    if a.has_errors() {
        return None;
    }
    let ir = vba_ir::lower(&a).ok()?;
    vba_ir::verify(&ir).expect("lowered programs verify");
    let host = Rc::new(HeadlessHost::new().with_limits(ResourceLimits {
        max_steps: Some(max_steps),
        max_memory_bytes: Some(64 << 20),
        max_call_depth: 200,
    }));
    let sink = BufferSink::new();
    let mut vm = Vm::new(
        Rc::new(ir),
        host.clone(),
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig::default(),
    )
    .ok()?;
    let result = match vm.find_procedure(entry) {
        None => "no-entry".to_owned(),
        Some(p) => match vm.run_procedure(p, vec![]) {
            Ok(o) => format!("ok {:?}", o.value),
            Err(e @ VmError::Internal(_)) => panic!("internal error: {e}"),
            Err(e) => format!("err {e}"),
        },
    };
    Some(Run {
        out: sink.contents(),
        result,
        steps: host.budget().steps_used(),
    })
}

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn every_corpus_program_is_deterministic() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/vba");
    let mut files = Vec::new();
    collect(&base, &mut files);
    assert!(files.len() > 1000, "corpus not found");
    let mut digest = String::new();
    let mut failures = Vec::new();
    for f in &files {
        let id = f.strip_prefix(&base).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(f).unwrap();
        let (mods, entry, max_steps) = parse(&text);
        let r = std::panic::catch_unwind(|| {
            let first = execute(&mods, &entry, max_steps)?;
            let second = execute(&mods, &entry, max_steps).expect("compiles again");
            Some((first, second))
        });
        match r {
            Err(p) => {
                let msg = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                    .unwrap_or_default();
                failures.push(format!("{id}: panic: {msg}"));
            }
            Ok(None) => digest.push_str(&format!("{id} skipped\n")),
            Ok(Some((a, b))) => {
                if (&a.out, &a.result, a.steps) != (&b.out, &b.result, b.steps) {
                    failures.push(format!(
                        "{id}: runs differ: steps {} vs {}, result {:?} vs {:?}",
                        a.steps, b.steps, a.result, b.result
                    ));
                }
                digest.push_str(&format!(
                    "{id} {} {:016x} {}\n",
                    a.steps,
                    fnv(&a.out),
                    fnv(&a.result)
                ));
            }
        }
    }
    if let Some(path) = std::env::var_os("VBA_STEP_DIGEST") {
        std::fs::write(path, digest).unwrap();
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
