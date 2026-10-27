//! The conformance harness.
//!
//! Runs every program under `tests/vba/` through the public facade and
//! compares its output with the expectation embedded in the program, then
//! checks that `tests/inventory.csv` is well-formed, agrees with the programs
//! and that `docs/tooling/conformance.md` is the summary of that inventory.
//!
//! The program format and the runner live in the `vba-conformance` crate
//! (`tooling/vba-conformance`); this test runs the corpus with the default
//! engine and owns the inventory and the generated summary.
//!
//! Regenerate the derived inventory columns and `docs/tooling/conformance.md` with
//! `VBA_BLESS=1 cargo test -p vba-rs --test conformance`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use vba::Engine;
use vba_conformance::{Options, Program, Provenance, Xfail, load_programs, run_programs};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn bless() -> bool {
    std::env::var_os("VBA_BLESS").is_some()
}

fn programs() -> Vec<Program> {
    match load_programs(&root().join("tests/vba")) {
        Ok(p) => p,
        Err(errors) => panic!("malformed programs:\n{}", errors.join("\n")),
    }
}

#[test]
fn every_program_matches_its_expectation() {
    let progs = programs();
    let report = run_programs(&progs, Engine::new, &Options::all());
    let failures = report.failures();
    assert!(
        failures.is_empty(),
        "{} of {} programs failed:\n{}",
        failures.len(),
        progs.len(),
        failures.join("\n\n")
    );
}

// --------------------------------------------------------------- inventory

const STATUSES: [&str; 4] = ["implemented", "partial", "unsupported", "not-started"];
const COLUMNS: [&str; 8] = [
    "id",
    "category",
    "name",
    "spec_ref",
    "status",
    "tests",
    "provenance",
    "notes",
];

#[derive(Debug, Clone)]
struct Entry {
    fields: Vec<String>,
}

impl Entry {
    fn get(&self, col: &str) -> &str {
        &self.fields[COLUMNS.iter().position(|c| *c == col).unwrap()]
    }
    fn set(&mut self, col: &str, v: String) {
        let i = COLUMNS.iter().position(|c| *c == col).unwrap();
        self.fields[i] = v;
    }
}

fn parse_csv(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => in_quotes = false,
                _ => field.push(c),
            }
        } else {
            match c {
                '"' => in_quotes = true,
                ',' => row.push(std::mem::take(&mut field)),
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                }
                '\r' => {}
                _ => field.push(c),
            }
        }
    }
    if in_quotes {
        return Err("unterminated quote".into());
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

fn read_inventory() -> Vec<Entry> {
    let text =
        std::fs::read_to_string(root().join("tests/inventory.csv")).expect("tests/inventory.csv");
    let rows = parse_csv(&text).expect("inventory.csv is valid CSV");
    assert_eq!(rows[0], COLUMNS, "inventory header");
    rows[1..]
        .iter()
        .map(|r| {
            assert_eq!(r.len(), COLUMNS.len(), "wrong column count in row {r:?}");
            Entry { fields: r.clone() }
        })
        .collect()
}

fn write_inventory(entries: &[Entry]) {
    let mut s = COLUMNS.join(",");
    s.push('\n');
    for e in entries {
        let line: Vec<String> = e.fields.iter().map(|f| csv_field(f)).collect();
        s.push_str(&line.join(","));
        s.push('\n');
    }
    std::fs::write(root().join("tests/inventory.csv"), s).unwrap();
}

/// The statuses, test ids and provenance the programs imply for an entry.
fn derive(entry_id: &str, progs: &[Program]) -> (String, String, String) {
    let mine: Vec<&Program> = progs
        .iter()
        .filter(|p| p.inv.iter().any(|i| i == entry_id))
        .collect();
    if mine.is_empty() {
        return ("not-started".into(), String::new(), String::new());
    }
    let failing: Vec<&&Program> = mine.iter().filter(|p| p.xfail.is_some()).collect();
    let status = if failing.is_empty() {
        "implemented"
    } else if failing.len() == mine.len()
        && failing
            .iter()
            .all(|p| matches!(p.xfail, Some((Xfail::Unsupported, _))))
    {
        "unsupported"
    } else {
        "partial"
    };
    let tests = mine
        .iter()
        .map(|p| p.id.clone())
        .collect::<Vec<_>>()
        .join(";");
    // The weakest provenance among the programs that pass: that is what the
    // implemented behaviour is measured against.
    let basis: Vec<&&Program> = mine.iter().filter(|p| p.xfail.is_none()).collect();
    let basis: Vec<&&Program> = if basis.is_empty() {
        mine.iter().collect()
    } else {
        basis
    };
    let prov = basis.iter().map(|p| p.provenance).max().unwrap();
    (status.into(), tests, prov.name().into())
}

fn blessed_inventory() -> Vec<Entry> {
    let progs = programs();
    let mut entries = read_inventory();
    for e in &mut entries {
        let (status, tests, prov) = derive(e.get("id"), &progs);
        e.set("status", status);
        e.set("tests", tests);
        e.set("provenance", prov);
    }
    entries
}

#[test]
fn inventory_is_well_formed() {
    let entries = read_inventory();
    assert!(!entries.is_empty());
    let mut ids = BTreeSet::new();
    for e in &entries {
        let id = e.get("id");
        assert!(
            !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || ".-_".contains(c)),
            "bad id `{id}`"
        );
        assert!(ids.insert(id.to_owned()), "duplicate id `{id}`");
        for col in ["category", "name", "spec_ref"] {
            assert!(!e.get(col).is_empty(), "{id}: empty {col}");
        }
        assert!(
            e.get("spec_ref")
                .starts_with("https://learn.microsoft.com/")
                || e.get("spec_ref").starts_with("ms-vbal"),
            "{id}: spec_ref must be a learn.microsoft.com URL or an ms-vbal section"
        );
        assert!(
            STATUSES.contains(&e.get("status")),
            "{id}: bad status `{}`",
            e.get("status")
        );
        let prov = e.get("provenance");
        assert!(
            prov.is_empty() || Provenance::ALL.iter().any(|p| p.name() == prov),
            "{id}: bad provenance `{prov}`"
        );
        assert_eq!(
            e.get("tests").is_empty(),
            prov.is_empty(),
            "{id}: tests/provenance mismatch"
        );
    }
}

#[test]
fn inventory_references_existing_programs_and_has_tests_for_implemented() {
    let entries = read_inventory();
    let progs: BTreeMap<String, Program> =
        programs().into_iter().map(|p| (p.id.clone(), p)).collect();
    let ids: BTreeSet<&str> = entries.iter().map(|e| e.get("id")).collect();
    for e in &entries {
        let tests: Vec<&str> = e
            .get("tests")
            .split(';')
            .filter(|s| !s.is_empty())
            .collect();
        for t in &tests {
            assert!(
                progs.contains_key(*t),
                "{}: test `{t}` does not exist",
                e.get("id")
            );
        }
        if e.get("status") == "implemented" {
            assert!(
                !tests.is_empty(),
                "{}: implemented without a test",
                e.get("id")
            );
            for t in &tests {
                assert!(
                    progs[*t].xfail.is_none(),
                    "{}: implemented but `{t}` is a known failure",
                    e.get("id")
                );
            }
        }
    }
    for p in progs.values() {
        for i in &p.inv {
            assert!(
                ids.contains(i.as_str()),
                "{}: unknown inventory id `{i}`",
                p.id
            );
        }
    }
}

#[test]
fn inventory_agrees_with_the_programs() {
    let want = blessed_inventory();
    if bless() {
        write_inventory(&want);
        return;
    }
    let have = read_inventory();
    let mut diffs = Vec::new();
    for (h, w) in have.iter().zip(&want) {
        for col in ["status", "tests", "provenance"] {
            if h.get(col) != w.get(col) {
                diffs.push(format!(
                    "{} {col}: have `{}`, programs say `{}`",
                    h.get("id"),
                    h.get(col),
                    w.get(col)
                ));
            }
        }
    }
    assert!(
        diffs.is_empty(),
        "tests/inventory.csv is stale (run with VBA_BLESS=1):\n{}",
        diffs.join("\n")
    );
}

// ----------------------------------------------------------------- summary

fn summary(entries: &[Entry], progs: &[Program]) -> String {
    let mut s = String::new();
    let total = entries.len();
    let count = |st: &str| entries.iter().filter(|e| e.get("status") == st).count();
    let _ = writeln!(s, "# Conformance");
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "This file is generated from `tests/inventory.csv` and the programs under `tests/vba/` by\n\
         `VBA_BLESS=1 cargo test -p vba-rs --test conformance`; a test fails when it is stale.\n"
    );
    let _ = writeln!(
        s,
        "The inventory lists the statements, declarations, operators, functions, objects and data types\n\
         that the Microsoft VBA language reference (and MS-VBAL) enumerates. Each entry's status is derived from\n\
         the programs that exercise it: `implemented` means every program for the entry passes, `partial` that some\n\
         pass and some are recorded known deviations, `unsupported` that the construct is rejected or not run,\n\
         `not-started` that no program exists yet. **A status measures only what the listed programs check; it\n\
         is not a claim of full compatibility.** An entry with a handful of programs is far from fully\n\
         verified, and `unverified` provenance means the expected value is the implementation's own behavior.\n"
    );
    let _ = writeln!(
        s,
        "Provenance of the expected values (the weakest among an entry's passing programs):\n"
    );
    let _ = writeln!(
        s,
        "* `oracle`: confirmed by running the program in real VBA (environment pinned in `tests/oracle/ENVIRONMENTS.md`)"
    );
    let _ = writeln!(s, "* `spec`: stated by MS-VBAL (a section is cited)");
    let _ = writeln!(
        s,
        "* `doc`: stated by the cited Microsoft language-reference page (example or remark)"
    );
    let _ = writeln!(
        s,
        "* `derived`: derived by reasoning from documented semantics, not stated verbatim"
    );
    let _ = writeln!(
        s,
        "* `unverified`: only the current behavior; no document backs it\n"
    );
    let _ = writeln!(s, "## Totals\n");
    let _ = writeln!(s, "| Measure | Count |\n| --- | ---: |");
    let _ = writeln!(s, "| Inventory entries | {total} |");
    for st in STATUSES {
        let _ = writeln!(s, "| {st} | {} |", count(st));
    }
    let _ = writeln!(s, "| Conformance programs | {} |", progs.len());
    let _ = writeln!(
        s,
        "| Programs that are known deviations (XFAIL) | {} |\n",
        progs.iter().filter(|p| p.xfail.is_some()).count()
    );

    let mut cats: Vec<String> = Vec::new();
    for e in entries {
        if !cats.iter().any(|c| c == e.get("category")) {
            cats.push(e.get("category").to_owned());
        }
    }
    let _ = writeln!(s, "## Entries per category and status\n");
    let _ = writeln!(
        s,
        "| Category | Total | implemented | partial | unsupported | not-started |"
    );
    let _ = writeln!(s, "| --- | ---: | ---: | ---: | ---: | ---: |");
    for c in &cats {
        let of: Vec<&Entry> = entries.iter().filter(|e| e.get("category") == c).collect();
        let n = |st: &str| of.iter().filter(|e| e.get("status") == st).count();
        let _ = writeln!(
            s,
            "| {c} | {} | {} | {} | {} | {} |",
            of.len(),
            n("implemented"),
            n("partial"),
            n("unsupported"),
            n("not-started")
        );
    }
    let _ = writeln!(s, "\n## Entries per provenance\n");
    let _ = writeln!(s, "| Provenance | Entries |\n| --- | ---: |");
    for p in Provenance::ALL {
        let n = entries
            .iter()
            .filter(|e| e.get("provenance") == p.name())
            .count();
        let _ = writeln!(s, "| {} | {n} |", p.name());
    }
    let none = entries
        .iter()
        .filter(|e| e.get("provenance").is_empty())
        .count();
    let _ = writeln!(s, "| (no program) | {none} |");

    let _ = writeln!(s, "\n## Programs per area and provenance\n");
    let _ = writeln!(
        s,
        "| Area | Programs | oracle | spec | doc | derived | unverified |"
    );
    let _ = writeln!(s, "| --- | ---: | ---: | ---: | ---: | ---: | ---: |");
    let mut areas: BTreeMap<String, Vec<&Program>> = BTreeMap::new();
    for p in progs {
        areas
            .entry(p.id.split('/').next().unwrap().to_owned())
            .or_default()
            .push(p);
    }
    for (a, ps) in &areas {
        let n = |pr: Provenance| ps.iter().filter(|p| p.provenance == pr).count();
        let _ = writeln!(
            s,
            "| {a} | {} | {} | {} | {} | {} | {} |",
            ps.len(),
            n(Provenance::Oracle),
            n(Provenance::Spec),
            n(Provenance::Doc),
            n(Provenance::Derived),
            n(Provenance::Unverified)
        );
    }

    let _ = writeln!(s, "\n## Gaps\n");
    let _ = writeln!(
        s,
        "Entries that are not `implemented`, with the reason recorded in the inventory.\n"
    );
    let _ = writeln!(
        s,
        "| Id | Category | Name | Status | Notes |\n| --- | --- | --- | --- | --- |"
    );
    for e in entries.iter().filter(|e| e.get("status") != "implemented") {
        let _ = writeln!(
            s,
            "| `{}` | {} | {} | {} | {} |",
            e.get("id"),
            e.get("category"),
            e.get("name").replace('|', "\\|"),
            e.get("status"),
            e.get("notes").replace('|', "\\|")
        );
    }
    let _ = writeln!(s, "\n## Known deviations\n");
    let _ = writeln!(
        s,
        "Programs whose documented expectation the implementation does not meet (kept failing on purpose).\n"
    );
    let _ = writeln!(s, "| Program | Kind | Reason |\n| --- | --- | --- |");
    for p in progs.iter().filter(|p| p.xfail.is_some()) {
        let (k, why) = p.xfail.as_ref().unwrap();
        let _ = writeln!(
            s,
            "| `{}` | {} | {} |",
            p.id,
            if *k == Xfail::Wrong {
                "wrong"
            } else {
                "unsupported"
            },
            why.replace('|', "\\|")
        );
    }
    s.push_str(RUNNING_UNDER_ANOTHER_ENGINE);
    s
}

/// The static closing section of the generated document.
const RUNNING_UNDER_ANOTHER_ENGINE: &str = "
## Running the corpus under another engine

The programs, the header format and the runner live in the `vba-conformance` crate
(`tooling/vba-conformance`), not in the facade. This is deliberate: a `conformance` feature on
the `vba` facade would put test tooling (file walking, header parsing) in the public API
surface that embedding applications depend on, while a dev-only crate costs them nothing and
can change freely. The repository's own test (`crates/api/vba-rs/tests/conformance.rs`) uses it
with `Engine::new` and owns the inventory and this document, which stay byte-identical for the
default engine.

An embedding application adds `vba-conformance` as a path or git dev-dependency and runs the
corpus (a checkout of `tests/vba/`) with its own engine configuration:

```rust
let options = vba_conformance::Options {
    filter: vec![\"strings/\".into()],                       // optional: ids containing any of these
    provides: Some([\"host-clock\".to_owned()].into()),     // optional: what this host provides
};
let report = vba_conformance::run_corpus(corpus_dir, || my_engine(), &options)?;
assert!(report.is_ok(), \"{}\\n{}\", report.summary(), report.failures().join(\"\\n\"));
```

Semantics:

* The factory is called once per program, so no state leaks between programs. The program's
  own `STRICT_SPEC`, `GRANT`, `CLOCK` and `MAX_STEPS` headers are applied on top of the engine
  the factory returns (they override the same settings of the factory's engine).
* `' REQUIRES: a, b` (repeatable) names host features the program needs. A feature is an
  arbitrary token the embedding application chooses to provide (for example `host-clock`);
  the repository's programs use none yet. Every `GRANT` also implies the feature
  `grant:<capability>` (for example `grant:file-write`).
* With `provides: None` (the default, used by the repository test) every feature is
  provided and nothing is skipped. With `provides: Some(set)`, a program that needs a
  feature outside the set is skipped and reported as `Outcome::Skipped` with the missing
  features; it is neither a pass nor a failure. A skipped program is never run, so it cannot
  fail under a restricted host.
* A program with `XFAIL` must still fail, and one that now passes is reported as
  `Outcome::UnexpectedPass`, exactly as in the repository test. Skipping takes precedence.
* The expected values were produced for the default engine. An embedding application that
  changes language behavior (not just host features) can see real differences; those are
  reported as failures, not hidden.
";

#[test]
fn conformance_doc_is_current() {
    let progs = programs();
    let entries = blessed_inventory();
    let doc = summary(&entries, &progs);
    let path = root().join("docs/tooling/conformance.md");
    if bless() {
        std::fs::write(&path, &doc).unwrap();
        return;
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        have == doc,
        "docs/tooling/conformance.md is stale (run with VBA_BLESS=1 cargo test -p vba-rs --test conformance)"
    );
}
