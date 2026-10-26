//! The conformance harness.
//!
//! Runs every program under `tests/vba/` through the public facade and
//! compares its output with the expectation embedded in the program, then
//! checks that `tests/inventory.csv` is well-formed, agrees with the programs
//! and that `docs/tooling/conformance.md` is the summary of that inventory.
//!
//! Program format (comment lines at the top of a `.bas` file):
//!
//! ```text
//! ' INV: stmt.if, op.eq          inventory ids this program tests
//! ' SOURCE: doc https://...      spec <ref> | doc <url> | derived <why> | unverified <note>
//! ' EXPECT: text                 one line of Debug.Print output (repeatable, exact text)
//! ' ENTRY: Main                  procedure to run (default Main)
//! ' ERROR: 11                    the run must end with this unhandled VBA error number,
//!                                or `load` (does not compile), `limit`, `cancelled`, `unsupported`
//! ' GRANT: file-write            capability grants (repeatable; `name=prefix` scopes it)
//! ' MAX_STEPS: 100000            step budget
//! ' CLOCK: 2024-06-15 12:30:00   fix the clock (default 2000-01-01 00:00:00)
//! ' STRICT_SPEC: true            analyse with `Engine::strict_spec(true)`: the spec-strict
//!                                diagnostics are errors (default: warnings, the program runs)
//! ' XFAIL: wrong|unsupported why known deviation: the program must still fail
//! ```
//!
//! A line `'#MODULE Name standard|class` starts another module in the same file.
//!
//! Regenerate the derived inventory columns and `docs/tooling/conformance.md` with
//! `VBA_BLESS=1 cargo test -p vba-rs --test conformance`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use vba::runtime::Error as RunError;
use vba::{Capability, Engine, Error, ModuleKind, Project};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn bless() -> bool {
    std::env::var_os("VBA_BLESS").is_some()
}

// ---------------------------------------------------------------- programs

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Provenance {
    Oracle,
    Spec,
    Doc,
    Derived,
    Unverified,
}

impl Provenance {
    fn name(self) -> &'static str {
        match self {
            Provenance::Oracle => "oracle",
            Provenance::Spec => "spec",
            Provenance::Doc => "doc",
            Provenance::Derived => "derived",
            Provenance::Unverified => "unverified",
        }
    }
    const ALL: [Provenance; 5] = [
        Provenance::Oracle,
        Provenance::Spec,
        Provenance::Doc,
        Provenance::Derived,
        Provenance::Unverified,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Xfail {
    Wrong,
    Unsupported,
}

#[derive(Debug)]
struct Program {
    id: String,
    inv: Vec<String>,
    provenance: Provenance,
    source_ref: String,
    expect: Vec<String>,
    entry: String,
    error: Option<String>,
    grants: Vec<String>,
    max_steps: Option<u64>,
    strict_spec: bool,
    clock: Option<(i32, u32, u32, u32, u32, u32)>,
    xfail: Option<(Xfail, String)>,
    raises: Vec<(String, String)>,
    modules: Vec<(String, ModuleKind, String)>,
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
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

fn parse_program(base: &Path, path: &Path) -> Result<Program, String> {
    let id = path
        .strip_prefix(base)
        .unwrap()
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/");
    let text = std::fs::read_to_string(path).map_err(|e| format!("{id}: {e}"))?;
    let mut p = Program {
        id: id.clone(),
        inv: Vec::new(),
        provenance: Provenance::Unverified,
        source_ref: String::new(),
        expect: Vec::new(),
        entry: "Main".into(),
        error: None,
        grants: Vec::new(),
        max_steps: None,
        strict_spec: false,
        clock: None,
        xfail: None,
        raises: Vec::new(),
        modules: Vec::new(),
    };
    let mut have_source = false;
    let mut current: (String, ModuleKind, String) =
        ("Module1".into(), ModuleKind::Standard, String::new());
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("'#MODULE ") {
            let mut it = rest.split_whitespace();
            let name = it.next().ok_or(format!("{id}: #MODULE without a name"))?;
            let kind = match it.next() {
                Some("standard") | None => ModuleKind::Standard,
                Some("class") => ModuleKind::Class,
                Some("document") => ModuleKind::Document,
                Some(k) => return Err(format!("{id}: unknown module kind `{k}`")),
            };
            let previous = std::mem::replace(&mut current, (name.to_owned(), kind, String::new()));
            // The header comments before the first marker are not a module.
            let blank = previous
                .2
                .lines()
                .all(|l| l.trim().is_empty() || l.trim_start().starts_with('\''));
            if !(blank && p.modules.is_empty() && previous.0 == "Module1") {
                p.modules.push(previous);
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("' ")
            && let Some((key, value)) = rest.split_once(':')
            && key.chars().all(|c| c.is_ascii_uppercase() || c == '_')
            && !key.is_empty()
        {
            let value = value.strip_prefix(' ').unwrap_or(value);
            let value = value.trim_end_matches('\r');
            match key {
                "INV" => p.inv.extend(
                    value
                        .split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty()),
                ),
                "SOURCE" => {
                    have_source = true;
                    let (kind, r) = value.split_once(' ').unwrap_or((value, ""));
                    p.provenance = match kind {
                        "oracle" => Provenance::Oracle,
                        "spec" => Provenance::Spec,
                        "doc" => Provenance::Doc,
                        "derived" => Provenance::Derived,
                        "unverified" => Provenance::Unverified,
                        k => return Err(format!("{id}: unknown provenance `{k}`")),
                    };
                    if r.trim().is_empty() {
                        return Err(format!("{id}: SOURCE needs a reference or note"));
                    }
                    p.source_ref = r.trim().to_owned();
                }
                "EXPECT" => p.expect.push(value.to_owned()),
                "ENTRY" => p.entry = value.to_owned(),
                "ERROR" => p.error = Some(value.trim().to_owned()),
                "RAISE" => {
                    let (a, b) = value
                        .split_once(' ')
                        .ok_or(format!("{id}: RAISE needs a source and an event"))?;
                    p.raises.push((a.to_owned(), b.trim().to_owned()));
                }
                "GRANT" => p.grants.push(value.trim().to_owned()),
                "MAX_STEPS" => {
                    p.max_steps = Some(
                        value
                            .trim()
                            .parse()
                            .map_err(|_| format!("{id}: bad MAX_STEPS"))?,
                    )
                }
                "CLOCK" => {
                    let n: Vec<u32> = value
                        .split(|c: char| !c.is_ascii_digit())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.parse().map_err(|_| format!("{id}: bad CLOCK")))
                        .collect::<Result<_, _>>()?;
                    if n.len() < 3 || n.len() > 6 {
                        return Err(format!("{id}: CLOCK is `Y-M-D [h:m:s]`"));
                    }
                    let at = |i: usize| n.get(i).copied().unwrap_or(0);
                    p.clock = Some((n[0] as i32, n[1], n[2], at(3), at(4), at(5)));
                }
                "STRICT_SPEC" => p.strict_spec = value.trim() == "true",
                "XFAIL" => {
                    let (k, why) = value.split_once(' ').unwrap_or((value, ""));
                    let kind = match k {
                        "wrong" => Xfail::Wrong,
                        "unsupported" => Xfail::Unsupported,
                        _ => return Err(format!("{id}: XFAIL must start with wrong|unsupported")),
                    };
                    p.xfail = Some((kind, why.to_owned()));
                }
                _ => {}
            }
        }
        current.2.push_str(line);
        current.2.push_str("\r\n");
    }
    p.modules.push(current);
    if !have_source {
        return Err(format!("{id}: missing SOURCE"));
    }
    if p.inv.is_empty() {
        return Err(format!("{id}: missing INV"));
    }
    if matches!(p.provenance, Provenance::Spec) && !p.source_ref.starts_with("ms-vbal") {
        return Err(format!("{id}: a `spec` SOURCE must cite ms-vbal"));
    }
    if matches!(p.provenance, Provenance::Doc) && !p.source_ref.starts_with("https://") {
        return Err(format!("{id}: a `doc` SOURCE must cite a URL"));
    }
    Ok(p)
}

fn programs() -> Vec<Program> {
    let base = root().join("tests/vba");
    let mut files = Vec::new();
    collect(&base, &mut files);
    let mut errors = Vec::new();
    let mut out = Vec::new();
    for f in files {
        match parse_program(&base, &f) {
            Ok(p) => out.push(p),
            Err(e) => errors.push(e),
        }
    }
    assert!(
        errors.is_empty(),
        "malformed programs:\n{}",
        errors.join("\n")
    );
    out
}

fn capability(name: &str) -> Capability {
    match name {
        "file-read" => Capability::FileRead,
        "file-write" => Capability::FileWrite,
        "process-spawn" => Capability::ProcessSpawn,
        "network" => Capability::Network,
        "com-create" => Capability::ComCreate,
        "native-declare" => Capability::NativeDeclare,
        "ui-automation" => Capability::UiAutomation,
        other => panic!("unknown capability `{other}`"),
    }
}

/// Runs one program; `Err` describes the mismatch.
fn run_program(p: &Program) -> Result<(), String> {
    let mut project = Project::new();
    for (name, kind, src) in &p.modules {
        project
            .add_module(name, *kind, src)
            .map_err(|e| format!("cannot add module {name}: {e}"))?;
    }
    let mut engine = Engine::new().strict_spec(p.strict_spec);
    for g in &p.grants {
        engine = match g.split_once('=') {
            Some((n, prefix)) => engine.grant_prefixes(capability(n), [prefix.to_owned()]),
            None => engine.grant(capability(g)),
        };
    }
    if let Some((y, mo, d, h, mi, s)) = p.clock {
        let now = vba::Date::from_ymd_hms(y, mo, d, h, mi, s)
            .map_err(|_| format!("{}: bad CLOCK", p.id))?;
        engine = engine.fixed_clock(now);
    }
    // A program that does not terminate must fail, not hang the suite.
    engine = engine.max_steps(p.max_steps.unwrap_or(20_000_000));
    let report = engine.analyze(&project);
    if p.error.as_deref() == Some("load") {
        return if report.has_errors() {
            Ok(())
        } else {
            Err("expected the project not to compile, but it did".into())
        };
    }
    let mut inst = engine
        .load(&project)
        .map_err(|e| format!("load failed: {e}"))?;
    inst.trust();
    let mut result = inst.run(&p.entry, vec![]).map(|_| ());
    if result.is_ok() {
        for (src, ev) in &p.raises {
            if let Err(e) = inst.raise_event(src, ev, vec![]) {
                result = Err(e);
                break;
            }
        }
    }
    let _ = inst.shutdown();
    let output = inst.output();
    let expected: String = p.expect.iter().map(|l| format!("{l}\n")).collect();
    let actual_error = match &result {
        Ok(_) => None,
        Err(Error::Run(RunError::Vba(r) | RunError::Host(r) | RunError::PermissionDenied(r))) => {
            Some(r.number.to_string())
        }
        Err(Error::Run(RunError::Limit(_))) => Some("limit".to_owned()),
        Err(Error::Run(RunError::Cancelled(_))) => Some("cancelled".to_owned()),
        Err(Error::Run(RunError::Bridge(_))) => Some("bridge".to_owned()),
        Err(Error::Run(RunError::Unsupported(_))) => Some("unsupported".to_owned()),
        Err(e) => return Err(format!("unexpected failure: {e}")),
    };
    if actual_error != p.error {
        return Err(format!(
            "error mismatch: expected {:?}, got {:?} ({:?})\noutput so far: {:?}",
            p.error,
            actual_error,
            result.err().map(|e| e.to_string()),
            output
        ));
    }
    if output != expected {
        return Err(format!(
            "output mismatch\n--- expected\n{expected:?}\n--- actual\n{output:?}"
        ));
    }
    Ok(())
}

#[test]
fn every_program_matches_its_expectation() {
    let mut failures = Vec::new();
    let progs = programs();
    for p in &progs {
        let outcome = run_program(p);
        match (&p.xfail, outcome) {
            (None, Ok(())) => {}
            (None, Err(e)) => failures.push(format!("{}: {e}", p.id)),
            (Some(_), Err(_)) => {}
            (Some((_, why)), Ok(())) => failures.push(format!(
                "{}: expected to fail ({why}) but passes; remove its XFAIL",
                p.id
            )),
        }
    }
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
    s
}

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
