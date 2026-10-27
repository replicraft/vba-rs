//! Tests of the differential-testing (oracle) tooling.
//!
//! The Windows side (`tests/oracle/*.ps1`, `OracleHarness.bas`) is UNTESTED here.
//! Everything below tests the Rust side only. All "oracle results" in this file
//! are SYNTHETIC, fabricated to exercise each verdict; they are never stored as
//! real oracle data, and every test that writes works in a temporary copy: the
//! committed conformance data is not changed.

#[path = "oracle_support/mod.rs"]
mod oracle_support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oracle_support as o;
use serde_json::json;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn real_skip() -> BTreeMap<String, (String, String)> {
    let text = std::fs::read_to_string(root().join("tests/oracle/oracle-skip.txt"))
        .expect("tests/oracle/oracle-skip.txt");
    o::parse_skip_list(&text)
}

// ------------------------------------------------------------ the rewriter

#[test]
fn rewriter_translates_the_statement_forms() {
    let cases = [
        ("Debug.Print x", "Print #255, x"),
        ("    debug.print \"a\"; 1", "    Print #255, \"a\"; 1"),
        ("Debug.Print", "Print #255, \"\""),
        ("Debug.Print ' comment", "Print #255, \"\" ' comment"),
        ("Debug.Print x;", "Print #255, x;"),
        (
            "Debug.Print Spc(3); Tab(9); \"z\"",
            "Print #255, Spc(3); Tab(9); \"z\"",
        ),
        ("10 Debug.Print 1", "10 Print #255, 1"),
        ("    10 Debug.Print 1", "    10 Print #255, 1"),
        ("Lbl: Debug.Print 2", "Lbl: Print #255, 2"),
        ("a = 1: Debug.Print a", "a = 1: Print #255, a"),
        (
            "If x Then Debug.Print \"t\" Else Debug.Print \"f\"",
            "If x Then Print #255, \"t\" Else Print #255, \"f\"",
        ),
        (
            "If x Then Debug.Print Else Debug.Print 1",
            "If x Then Print #255, \"\" Else Print #255, 1",
        ),
        // Text inside strings and comments is left alone.
        ("s = \"Debug.Print x\"", "s = \"Debug.Print x\""),
        ("x = 1 ' Debug.Print x", "x = 1 ' Debug.Print x"),
        ("Rem Debug.Print x", "Rem Debug.Print x"),
        ("MyDebug.Print", "MyDebug.Print"),
    ];
    for (input, want) in cases {
        assert_eq!(o::rewrite_line(input).as_deref(), Ok(want), "{input}");
    }
}

#[test]
fn rewriter_refuses_what_it_cannot_translate() {
    for bad in [
        "Debug.Assert x",
        "Debug.Print ; x",
        "Debug.Print , x",
        "y = Debug",
        "Debug _",
        "Call Debug.Print(1)",
        "x = Foo(Debug.Print)",
        "With Debug",
        "s = \"\u{e9}\": Debug.Print s",
    ] {
        assert!(o::rewrite_line(bad).is_err(), "{bad} must be refused");
    }
}

// ---------------------------------------------------------- the real corpus

#[test]
fn corpus_classification_is_consistent() {
    let progs = o::load_corpus(&root().join("tests/vba")).unwrap();
    assert!(progs.len() > 600);
    let skip = real_skip();
    let ids: Vec<&str> = progs.iter().map(|p| p.id.as_str()).collect();
    for id in skip.keys() {
        assert!(
            ids.contains(&id.as_str()),
            "oracle-skip.txt lists unknown id {id}"
        );
    }
    let mut capturable = 0;
    for p in &progs {
        match o::classify(p) {
            Ok(prep) => {
                if skip.contains_key(&p.id) {
                    continue;
                }
                capturable += 1;
                for (name, _, code) in &prep.modules {
                    for line in code.lines() {
                        let masked = o::mask_code(line);
                        assert!(
                            !o::words(&masked)
                                .iter()
                                .any(|w| w.lower == "debug" && !w.after_dot),
                            "{}: {name}: Debug left in `{line}`",
                            p.id
                        );
                    }
                }
            }
            Err(r) => assert!(
                skip.contains_key(&p.id),
                "{} is refused ({}) but missing from oracle-skip.txt; regenerate it with \
                 `cargo run -p vba-rs --example oracle -- skiplist`",
                p.id,
                r[0].reason
            ),
        }
    }
    assert!(
        capturable > 400,
        "only {capturable} programs are capturable"
    );
}

#[test]
fn bundle_is_written_for_the_real_corpus() {
    let progs = o::load_corpus(&root().join("tests/vba")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let m = o::write_bundle(
        &progs,
        &real_skip(),
        &root().join("tests/oracle/OracleHarness.bas"),
        dir.path(),
    )
    .unwrap();
    let entries = m["programs"].as_array().unwrap();
    assert_eq!(entries.len(), progs.len());
    let cap: Vec<_> = entries
        .iter()
        .filter(|e| e["status"] == "capturable")
        .collect();
    assert!(!cap.is_empty());
    for e in cap {
        let d = dir.path().join(e["dir"].as_str().unwrap());
        for f in e["files"].as_array().unwrap() {
            let text = std::fs::read(d.join(f.as_str().unwrap())).unwrap();
            assert!(text.is_ascii(), "{f} must be ASCII (the log is ANSI)");
            assert!(String::from_utf8(text).unwrap().contains("\r\n"));
        }
        assert!(d.join("OracleEntry.bas").exists());
    }
    assert!(dir.path().join("OracleHarness.bas").exists());
}

/// Self-consistency of the rewrite: running the rewritten program in our engine
/// with `Print #255` writing to a (virtual) log file, then replaying the file,
/// must give what the original `Debug.Print` program gives. This checks that the
/// rewritten text is valid and keeps column state and separators; it does NOT
/// prove that real VBA formats `Debug.Print` like `Print #` (that is what the
/// oracle run is for).
#[test]
fn rewritten_programs_reproduce_the_output_when_logged_to_a_file() {
    let progs = o::load_corpus(&root().join("tests/vba")).unwrap();
    let skip = real_skip();
    let mut failures = Vec::new();
    let mut checked = 0;
    let mut skipped: Vec<String> = Vec::new();
    for p in &progs {
        let Ok(prep) = o::classify(p) else { continue };
        if skip.contains_key(&p.id) || p.xfail {
            continue;
        }
        // A bare `End` ends the whole run, including this check's replay step.
        if p.modules.iter().any(|m| {
            m.2.lines()
                .any(|l| o::mask_code(l).trim().eq_ignore_ascii_case("end"))
        }) {
            continue;
        }
        let original = o::run_engine(p).unwrap();
        let mut project = vba::Project::new();
        for (name, kind, code) in &prep.modules {
            let k = match kind {
                o::Kind::Class => vba::ModuleKind::Class,
                _ => vba::ModuleKind::Standard,
            };
            project.add_module(name, k, code).unwrap();
        }
        let driver = format!(
            "Public Sub OracleSelf()\r\n\
             Dim e As Long\r\n\
             MkDir \"/ok\"\r\n\
             Open \"/ok/log.txt\" For Output As #255\r\n\
             On Error GoTo EH\r\n\
             {}.{}\r\n\
             GoTo Done\r\n\
             EH:\r\n\
             e = Err.Number\r\n\
             Done:\r\n\
             Close #255\r\n\
             Open \"/ok/log.txt\" For Binary As #1\r\n\
             Dim t As String\r\n\
             t = Space$(LOF(1))\r\n\
             Get #1, , t\r\n\
             Debug.Print t;\r\n\
             Close #1\r\n\
             If e <> 0 Then Err.Raise e\r\n\
             End Sub\r\n",
            prep.entry_module, prep.entry
        );
        project
            .add_module("OracleSelfCheck", vba::ModuleKind::Standard, &driver)
            .unwrap();
        let engine = vba::Engine::new()
            .grant_prefixes(vba::Capability::FileWrite, ["/ok".to_owned()])
            .grant_prefixes(vba::Capability::FileRead, ["/ok".to_owned()]);
        let mut inst = match engine.load(&project) {
            Ok(i) => i,
            Err(e) => {
                // Known engine limitation: `Print #255, 5#` lexes `#255, 5#` as a
                // date literal. Real VBA parses it; only the self-check skips it.
                assert!(e.to_string().contains("file number"), "{}: {e}", p.id);
                skipped.push(p.id.clone());
                continue;
            }
        };
        inst.trust();
        let result = inst.run("OracleSelf", vec![]);
        let _ = inst.shutdown();
        let err = match result {
            Ok(_) => None,
            Err(vba::Error::Run(vba::runtime::Error::Vba(r))) => Some(r.number.to_string()),
            Err(e) => Some(format!("other: {e}")),
        };
        checked += 1;
        if o::normalize(&inst.output()) != o::normalize(&original.output) || err != original.error {
            failures.push(format!(
                "{}: logged {:?} / {:?}, direct {:?} / {:?}",
                p.id,
                inst.output(),
                err,
                original.output,
                original.error
            ));
        }
    }
    assert!(
        skipped.len() <= 5,
        "too many programs the engine cannot load once rewritten: {skipped:?}"
    );
    assert!(checked > 400);
    assert!(
        failures.is_empty(),
        "{} of {checked} rewritten programs differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ------------------------------------------- synthetic comparison and apply

/// Writes a small SYNTHETIC corpus into `dir/vba` and returns the programs.
fn synthetic_corpus(dir: &Path) -> Vec<o::Program> {
    let make = |name: &str, header: &str, body: &str| {
        let p = dir.join("vba/demo").join(format!("{name}.bas"));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        let text = format!("' INV: synthetic.{name}\n{header}Public Sub Main()\n{body}End Sub\n");
        std::fs::write(p, text).unwrap();
    };
    let doc = "' SOURCE: doc https://example.invalid/doc\n";
    make(
        "match",
        &format!("{doc}' EXPECT:  1 \n"),
        "    Debug.Print 1\n",
    );
    make(
        "mismatch",
        &format!("{doc}' EXPECT: a\n"),
        "    Debug.Print \"a\"\n",
    );
    make("errdiff", &format!("{doc}' ERROR: 5\n"), "    Error 5\n");
    make(
        "err_same",
        &format!("{doc}' ERROR: 11\n"),
        "    Debug.Print \"x\"\n    Debug.Print 1 / 0\n",
    );
    make(
        "dangerous",
        &format!("{doc}' EXPECT: x\n"),
        "    Shell \"cmd\"\n",
    );
    make(
        "refused",
        &format!("{doc}' EXPECT: r\n"),
        "    Debug.Print \"r\"\n",
    );
    make(
        "noresult",
        &format!("{doc}' EXPECT: n\n"),
        "    Debug.Print \"n\"\n",
    );
    make(
        "stale",
        &format!("{doc}' EXPECT: s\n"),
        "    Debug.Print \"s\"\n",
    );
    make(
        "timeout",
        &format!("{doc}' EXPECT: t\n"),
        "    Debug.Print \"t\"\n",
    );
    make(
        "xfail",
        "' SOURCE: derived synthetic\n' XFAIL: wrong synthetic\n' EXPECT: q\n",
        "    Debug.Print \"x\"\n",
    );
    make(
        "ansi",
        &format!("{doc}' EXPECT: euro\n"),
        "    Debug.Print ChrW(8364)\n",
    );
    make(
        "lossy",
        &format!("{doc}' EXPECT: pi\n"),
        "    Debug.Print ChrW(960)\n",
    );
    o::load_corpus(&dir.join("vba")).unwrap()
}

fn synthetic_results(progs: &[o::Program]) -> BTreeMap<String, o::OracleResult> {
    let digest = |id: &str| {
        progs
            .iter()
            .find(|p| p.id == format!("demo/{id}"))
            .unwrap()
            .digest()
    };
    let lines = [
        json!({"id":"demo/match","env":"SYNTHETIC","status":"ok","output":" 1 \r\n","error_number":null,"digest":digest("match")}),
        json!({"id":"demo/mismatch","env":"SYNTHETIC","status":"ok","output":"b\r\n","digest":digest("mismatch")}),
        json!({"id":"demo/errdiff","env":"SYNTHETIC","status":"error","output":"","error_number":11,"error_description":"Division by zero","digest":digest("errdiff")}),
        json!({"id":"demo/err_same","env":"SYNTHETIC","status":"error","output":"x\r\n","error_number":11,"error_description":"Division by zero","digest":digest("err_same")}),
        json!({"id":"demo/refused","env":"SYNTHETIC","status":"not-capturable","reason":"driver refused","digest":digest("refused")}),
        json!({"id":"demo/stale","env":"SYNTHETIC","status":"ok","output":"s\r\n","digest":"0000000000000000"}),
        json!({"id":"demo/timeout","env":"SYNTHETIC","status":"timeout","error_description":"killed after 30 s"}),
        json!({"id":"demo/xfail","env":"SYNTHETIC","status":"ok","output":"x\r\n","digest":digest("xfail")}),
        json!({"id":"demo/ansi","env":"SYNTHETIC","status":"ok","output":"\u{20ac}\r\n","digest":digest("ansi")}),
        json!({"id":"demo/lossy","env":"SYNTHETIC","status":"ok","output":"?\r\n","digest":digest("lossy")}),
    ];
    let text: String = lines.iter().map(|l| format!("{l}\n")).collect();
    o::parse_results(&text).unwrap()
}

fn synthetic_env(locale: &str) -> o::Environment {
    o::parse_environment(
        &json!({
            "env_id": format!("synthetic-excel-16.0.1-{locale}-x64"),
            "host": "excel", "version": "16.0", "build": "1", "vba_version": "7.1",
            "bitness": "x64", "os_version": "synthetic", "locale_name": locale,
            "lcid": "1033", "locale_override": false, "captured_at": "2000-01-01T00:00:00Z"
        })
        .to_string(),
    )
    .unwrap()
}

fn verdict_of<'a>(rows: &'a [o::Row], id: &str) -> &'a o::Row {
    rows.iter().find(|r| r.id == format!("demo/{id}")).unwrap()
}

#[test]
fn synthetic_results_cover_every_verdict() {
    let dir = tempfile::tempdir().unwrap();
    let progs = synthetic_corpus(dir.path());
    let results = synthetic_results(&progs);
    let rows = o::compare(&progs, &BTreeMap::new(), &results);
    use o::Verdict::*;
    for (id, want) in [
        ("match", Match),
        ("mismatch", Mismatch),
        ("errdiff", ErrorDiffers),
        ("err_same", Match),
        ("dangerous", NotCapturable),
        ("refused", NotCapturable),
        ("noresult", NoResult),
        ("stale", Stale),
        ("timeout", OracleFailed),
        ("xfail", Match),
        ("ansi", Match),
        ("lossy", NotCapturable),
    ] {
        assert_eq!(verdict_of(&rows, id).verdict, want, "{id}");
    }
    // The diff names both sides.
    let d = &verdict_of(&rows, "mismatch").detail;
    assert!(d.contains("- |b|") && d.contains("+ |a|"), "{d}");
    let d = &verdict_of(&rows, "errdiff").detail;
    assert!(d.contains("oracle 11") && d.contains("engine 5"), "{d}");

    let report = o::render_report(&rows, Some(&synthetic_env("en-US")));
    for needle in [
        "## Summary",
        "## By area",
        "## By current provenance",
        "| MATCH | 4 |",
        "## MISMATCH",
        "## ERROR-DIFFERS",
        "## NOT-CAPTURABLE",
        "| doc |",
    ] {
        assert!(
            report.contains(needle),
            "report lacks `{needle}`:\n{report}"
        );
    }
}

#[test]
fn apply_upgrades_matches_records_the_environment_and_reports_mismatches() {
    let dir = tempfile::tempdir().unwrap();
    let progs = synthetic_corpus(dir.path());
    let rows = o::compare(&progs, &BTreeMap::new(), &synthetic_results(&progs));
    let oracle_dir = dir.path().join("oracle");
    std::fs::create_dir_all(&oracle_dir).unwrap();
    let env = synthetic_env("en-US");

    // A foreign locale is refused unless explicitly allowed; nothing changes.
    let fr = synthetic_env("fr-FR");
    assert!(o::apply(&progs, &rows, &fr, &oracle_dir, false).is_err());
    assert!(!oracle_dir.join("ENVIRONMENTS.md").exists());

    let out = o::apply(&progs, &rows, &env, &oracle_dir, false).unwrap();
    assert!(out.environment_row_added);
    let mut up = out.upgraded.clone();
    up.sort();
    // xfail programs are never upgraded.
    assert_eq!(up, ["demo/ansi", "demo/err_same", "demo/match"]);

    let m = std::fs::read_to_string(dir.path().join("vba/demo/match.bas")).unwrap();
    assert!(
        m.contains(&format!(
            "' SOURCE: oracle {} was: doc https://example.invalid/doc\n",
            env.env_id
        )),
        "{m}"
    );
    let mm = std::fs::read_to_string(dir.path().join("vba/demo/mismatch.bas")).unwrap();
    assert!(mm.contains("' SOURCE: doc https://example.invalid/doc"));
    let xf = std::fs::read_to_string(dir.path().join("vba/demo/xfail.bas")).unwrap();
    assert!(xf.contains("' SOURCE: derived synthetic"));

    let envs = std::fs::read_to_string(oracle_dir.join("ENVIRONMENTS.md")).unwrap();
    assert!(envs.contains(&format!("| `{}` |", env.env_id)));
    let report = std::fs::read_to_string(out.mismatch_report.unwrap()).unwrap();
    assert!(report.contains("demo/mismatch") && report.contains("demo/errdiff"));
    assert!(!report.contains("demo/match`"));

    // Re-running changes nothing and does not duplicate the table row.
    let progs = o::load_corpus(&dir.path().join("vba")).unwrap();
    let rows = o::compare(&progs, &BTreeMap::new(), &synthetic_results(&progs));
    let again = o::apply(&progs, &rows, &env, &oracle_dir, false).unwrap();
    assert!(again.upgraded.is_empty());
    assert!(!again.environment_row_added);
    let envs2 = std::fs::read_to_string(oracle_dir.join("ENVIRONMENTS.md")).unwrap();
    assert_eq!(envs, envs2);
}

#[test]
fn oracle_provenance_is_accepted_by_the_conformance_header_format() {
    // The conformance runner requires `SOURCE: <kind> <reference>`; the rewritten
    // line has that shape with kind `oracle`.
    let line = "' SOURCE: oracle excel-16.0-x64 was: unverified a note";
    let rest = line.strip_prefix("' SOURCE: ").unwrap();
    let (kind, r) = rest.split_once(' ').unwrap();
    assert_eq!(kind, "oracle");
    assert!(!r.trim().is_empty());
}

#[test]
fn results_parser_tolerates_a_bom_and_blank_lines() {
    let text = format!(
        "\u{feff}\n{}\n\n",
        json!({"id":"a/b","status":"ok","output":"x"})
    );
    let m = o::parse_results(&text).unwrap();
    assert_eq!(m["a/b"].output.as_deref(), Some("x"));
    assert!(o::parse_results("not json").is_err());
}

#[test]
fn every_oracle_program_names_an_environment_recorded_in_environments_md() {
    let envs = std::fs::read_to_string(root().join("tests/oracle/ENVIRONMENTS.md"))
        .expect("tests/oracle/ENVIRONMENTS.md");
    for p in o::load_corpus(&root().join("tests/vba")).unwrap() {
        if p.source_kind != "oracle" {
            continue;
        }
        let env = p.source_ref.split_whitespace().next().unwrap_or("");
        assert!(
            envs.contains(&format!("| `{env}` |")),
            "{}: environment `{env}` is not in tests/oracle/ENVIRONMENTS.md",
            p.id
        );
        assert!(
            p.source_ref.contains(" was: "),
            "{}: an oracle SOURCE must keep the previous provenance after `was:`",
            p.id
        );
    }
}

// ---------------------------------------------- the fake backend, end to end
//
// These tests stand in for the Windows lab. `fake_oracle` is a test double for
// `run-oracle.ps1`: it answers with our own engine's outputs (then with
// perturbed ones) in the exact results-file format. Everything after the
// driver is then run for real over a temporary copy of the whole corpus:
// directory ingest, report, promotion to `oracle`, the environment table and
// the mismatch report. None of this is evidence about real VBA.

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dest = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &dest);
        } else {
            std::fs::copy(e.path(), dest).unwrap();
        }
    }
}

/// A temporary copy of the corpus and of the oracle directory.
struct Lab {
    dir: tempfile::TempDir,
}

impl Lab {
    fn new() -> Lab {
        let dir = tempfile::tempdir().unwrap();
        copy_dir(&root().join("tests/vba"), &dir.path().join("vba"));
        let oracle = dir.path().join("oracle");
        std::fs::create_dir_all(&oracle).unwrap();
        for f in ["ENVIRONMENTS.md", "run-oracle.ps1", "LAB-README.txt"] {
            std::fs::copy(root().join("tests/oracle").join(f), oracle.join(f)).unwrap();
        }
        Lab { dir }
    }
    fn vba(&self) -> PathBuf {
        self.dir.path().join("vba")
    }
    fn oracle(&self) -> PathBuf {
        self.dir.path().join("oracle")
    }
    fn corpus(&self) -> Vec<o::Program> {
        o::load_corpus(&self.vba()).unwrap()
    }
    fn texts(&self) -> BTreeMap<String, String> {
        self.corpus().into_iter().map(|p| (p.id, p.text)).collect()
    }
    /// Plays the lab: writes what the driver would leave in its `out` folder.
    fn fake_run(&self, mode: o::FakeMode, env_id: &str) -> PathBuf {
        let out = self.dir.path().join(format!("out-{env_id}"));
        std::fs::create_dir_all(&out).unwrap();
        let (results, env) = o::fake_oracle(&self.corpus(), &real_skip(), mode).unwrap();
        let results = results.replace(o::FAKE_ENV_ID, env_id);
        let env = env.replace(o::FAKE_ENV_ID, env_id);
        std::fs::write(out.join(format!("results-{env_id}.jsonl")), results).unwrap();
        std::fs::write(out.join(format!("environment-{env_id}.json")), env).unwrap();
        out
    }
    /// `report` then `apply` for the one pair in `out`, as the command line does.
    fn ingest(&self, out: &Path) -> (Vec<o::Row>, o::ApplyOutcome, o::Environment) {
        let pairs = o::find_result_pairs(out).unwrap();
        assert_eq!(pairs.len(), 1);
        let progs = self.corpus();
        let results = o::parse_results(&std::fs::read_to_string(&pairs[0].0).unwrap()).unwrap();
        let env = o::parse_environment(&std::fs::read_to_string(&pairs[0].1).unwrap()).unwrap();
        let rows = o::compare(&progs, &real_skip(), &results);
        let applied = o::apply_opts(&progs, &rows, &env, &self.oracle(), false, true).unwrap();
        (rows, applied, env)
    }
}

fn count_verdict(rows: &[o::Row], v: o::Verdict) -> usize {
    rows.iter().filter(|r| r.verdict == v).count()
}

#[test]
fn the_fake_backend_that_echoes_our_answers_matches_everywhere_and_promotes() {
    let lab = Lab::new();
    let before = lab.corpus();
    let capturable = before
        .iter()
        .filter(|p| !real_skip().contains_key(&p.id) && o::classify(p).is_ok())
        .count();
    assert!(capturable > 400, "{capturable}");

    let out = lab.fake_run(o::FakeMode::Echo, o::FAKE_ENV_ID);
    let (rows, applied, env) = lab.ingest(&out);

    // Every capturable program matches, except the ones our engine does
    // not answer faithfully (it cannot load them, or ends with a limit or
    // "unsupported" outcome that real VBA has no number for): the fake answers
    // those with the program's own expectation, which the comparison reports.
    let unfaithful: Vec<&str> = before
        .iter()
        .filter(|p| !real_skip().contains_key(&p.id) && o::classify(p).is_ok())
        .filter(|p| match o::run_engine(p) {
            Err(_) => true,
            Ok(out) => out.error.is_some_and(|e| e.parse::<i64>().is_err()),
        })
        .map(|p| p.id.as_str())
        .collect();
    let matched = count_verdict(&rows, o::Verdict::Match);
    let differing =
        count_verdict(&rows, o::Verdict::Mismatch) + count_verdict(&rows, o::Verdict::ErrorDiffers);
    // Every capturable program has exactly one of the three outcomes.
    assert_eq!(matched + differing, capturable);
    assert!(differing <= unfaithful.len(), "{differing}");
    assert!(matched > capturable - 40, "{matched} of {capturable}");
    for r in rows
        .iter()
        .filter(|r| matches!(r.verdict, o::Verdict::Mismatch | o::Verdict::ErrorDiffers))
    {
        assert!(unfaithful.contains(&r.id.as_str()), "{}", r.id);
    }
    for v in [
        o::Verdict::OracleFailed,
        o::Verdict::NoResult,
        o::Verdict::Stale,
    ] {
        assert_eq!(count_verdict(&rows, v.clone()), 0, "{v:?}");
    }
    assert_eq!(applied.mismatch_report.is_some(), differing > 0);
    assert!(applied.environment_row_added);

    // Promotion: every matching program that is not a recorded deviation now
    // says `oracle <env> was: <old source>`; the conformance runner reads it.
    let xfail_matching = rows
        .iter()
        .filter(|r| r.verdict == o::Verdict::Match && r.xfail)
        .count();
    assert_eq!(applied.upgraded.len(), matched - xfail_matching);
    let after = lab.corpus();
    let promoted = after.iter().filter(|p| p.source_kind == "oracle").count();
    assert_eq!(promoted, applied.upgraded.len());
    for p in after.iter().filter(|p| p.source_kind == "oracle") {
        assert!(p.source_ref.starts_with(&env.env_id), "{}", p.id);
        assert!(p.source_ref.contains(" was: "), "{}", p.id);
        // Expectations are never touched by promotion.
        let old = before.iter().find(|q| q.id == p.id).unwrap();
        assert_eq!(old.expect, p.expect, "{}", p.id);
        assert_eq!(old.error, p.error, "{}", p.id);
    }
    let loaded = vba_conformance::load_programs(&lab.vba()).unwrap();
    let oracle_count = loaded
        .iter()
        .filter(|p| p.provenance == vba_conformance::Provenance::Oracle)
        .count();
    assert_eq!(oracle_count, promoted);
    // The corpus still passes with the promoted headers.
    let report =
        vba_conformance::run_programs(&loaded, vba::Engine::new, &vba_conformance::Options::all());
    assert!(report.failures().is_empty(), "{:?}", report.failures());

    let envs = std::fs::read_to_string(lab.oracle().join("ENVIRONMENTS.md")).unwrap();
    assert!(envs.contains(&format!("| `{}` |", env.env_id)));

    // Re-ingesting the same file is idempotent.
    let (_, again, _) = lab.ingest(&out);
    assert!(again.upgraded.is_empty() && !again.environment_row_added);
    assert_eq!(again.already_oracle.len(), promoted);
}

#[test]
fn the_fake_backend_with_perturbed_answers_is_reported_and_never_promoted() {
    let lab = Lab::new();
    let before = lab.texts();
    let out = lab.fake_run(o::FakeMode::Perturbed, o::FAKE_ENV_ID);
    let (rows, applied, env) = lab.ingest(&out);

    let mismatch = count_verdict(&rows, o::Verdict::Mismatch);
    let err_differs = count_verdict(&rows, o::Verdict::ErrorDiffers);
    let failed = count_verdict(&rows, o::Verdict::OracleFailed);
    assert!(
        mismatch > 0 && err_differs > 0 && failed > 0,
        "{mismatch} {err_differs} {failed}"
    );
    let matches = count_verdict(&rows, o::Verdict::Match);
    assert!(matches > 0);

    // Only matching programs changed; a perturbed one keeps its text exactly.
    let after = lab.texts();
    let bad = |r: &&o::Row| {
        matches!(
            r.verdict,
            o::Verdict::Mismatch | o::Verdict::ErrorDiffers | o::Verdict::OracleFailed
        )
    };
    for r in rows.iter().filter(bad) {
        assert_eq!(before[&r.id], after[&r.id], "{} changed", r.id);
    }
    assert!(applied.upgraded.len() <= matches);

    // Every non-matching program is in the mismatch report, with its diff.
    let report = std::fs::read_to_string(applied.mismatch_report.unwrap()).unwrap();
    for r in rows.iter().filter(bad) {
        assert!(report.contains(&r.id), "{} missing from the report", r.id);
    }
    assert!(
        report.contains("perturbed"),
        "the diff names the extra line"
    );
    let envs = std::fs::read_to_string(lab.oracle().join("ENVIRONMENTS.md")).unwrap();
    assert!(envs.contains(&format!("| `{}` |", env.env_id)));
}

#[test]
fn results_of_several_environments_merge_without_overwriting() {
    let lab = Lab::new();
    // Both environments ran the same bundle, before anything was applied.
    let first = lab.fake_run(o::FakeMode::Echo, "fake-excel-0.0.0-en-US-x64");
    let second = lab.fake_run(o::FakeMode::Perturbed, "fake-access-0.0.0-en-US-x64");

    // Environment one agrees everywhere and promotes.
    let (_, one, _) = lab.ingest(&first);
    assert!(!one.upgraded.is_empty());
    let text_after_one = lab.texts();

    // Environment two disagrees about some programs. Its results are not
    // stale although applying environment one rewrote the SOURCE lines.
    let (rows, two, env2) = lab.ingest(&second);
    assert_eq!(count_verdict(&rows, o::Verdict::Stale), 0);
    assert!(
        two.upgraded.is_empty(),
        "everything matching was already oracle"
    );
    assert!(!two.already_oracle.is_empty());

    // A program confirmed by environment one keeps that environment, even
    // where environment two disagrees; the disagreement is only reported.
    assert_eq!(text_after_one, lab.texts());
    let mism = std::fs::read_to_string(two.mismatch_report.unwrap()).unwrap();
    let a_disagreement = rows
        .iter()
        .find(|r| r.verdict == o::Verdict::Mismatch)
        .unwrap();
    assert!(mism.contains(&a_disagreement.id));
    assert!(mism.contains("fake-access-0.0.0-en-US-x64"));

    // Both environments are recorded, once each.
    let envs = std::fs::read_to_string(lab.oracle().join("ENVIRONMENTS.md")).unwrap();
    assert_eq!(envs.matches("| `fake-excel-0.0.0-en-US-x64` |").count(), 1);
    assert_eq!(envs.matches(&format!("| `{}` |", env2.env_id)).count(), 1);
}

#[test]
fn fake_results_are_refused_by_the_normal_apply() {
    let lab = Lab::new();
    let out = lab.fake_run(o::FakeMode::Echo, o::FAKE_ENV_ID);
    let pairs = o::find_result_pairs(&out).unwrap();
    let env = o::parse_environment(&std::fs::read_to_string(&pairs[0].1).unwrap()).unwrap();
    assert!(env.fake);
    let progs = lab.corpus();
    let results = o::parse_results(&std::fs::read_to_string(&pairs[0].0).unwrap()).unwrap();
    let rows = o::compare(&progs, &real_skip(), &results);
    let err = o::apply(&progs, &rows, &env, &lab.oracle(), true).unwrap_err();
    assert!(err.contains("fake backend"), "{err}");
    // Nothing was written.
    assert!(lab.corpus().iter().all(|p| p.source_kind != "oracle"));
    let envs = std::fs::read_to_string(lab.oracle().join("ENVIRONMENTS.md")).unwrap();
    assert!(!envs.contains("fake-"));
}

#[test]
fn a_stale_result_is_not_promoted_when_the_corpus_changed_after_the_run() {
    let lab = Lab::new();
    let out = lab.fake_run(o::FakeMode::Echo, o::FAKE_ENV_ID);
    // Change a program after the bundle was made.
    let victim = lab
        .vba()
        .join("procedures/named_argument_evaluation_order.bas");
    let mut text = std::fs::read_to_string(&victim).unwrap();
    text.push_str("' edited after the lab run\n");
    std::fs::write(&victim, text).unwrap();
    let (rows, applied, _) = lab.ingest(&out);
    let row = rows
        .iter()
        .find(|r| r.id == "procedures/named_argument_evaluation_order")
        .unwrap();
    assert_eq!(row.verdict, o::Verdict::Stale);
    assert!(!applied.upgraded.contains(&row.id));
}

#[test]
fn result_pairs_are_found_in_a_folder_and_a_missing_half_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(o::find_result_pairs(dir.path()).is_err());
    std::fs::write(dir.path().join("results-b.jsonl"), "").unwrap();
    assert!(
        o::find_result_pairs(dir.path())
            .unwrap_err()
            .contains("environment-b.json")
    );
    std::fs::write(dir.path().join("environment-b.json"), "{}").unwrap();
    std::fs::write(dir.path().join("results-a.jsonl"), "").unwrap();
    std::fs::write(dir.path().join("environment-a.json"), "{}").unwrap();
    std::fs::write(dir.path().join("return-a.zip"), "").unwrap();
    let pairs = o::find_result_pairs(dir.path()).unwrap();
    let names: Vec<String> = pairs
        .iter()
        .map(|(r, _)| r.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["results-a.jsonl", "results-b.jsonl"]);
}

#[test]
fn the_bundle_is_self_contained_for_the_lab() {
    let dir = tempfile::tempdir().unwrap();
    o::write_lab_files(&root().join("tests/oracle"), dir.path()).unwrap();
    let readme = std::fs::read_to_string(dir.path().join("LAB-README.txt")).unwrap();
    let ps1 = std::fs::read_to_string(dir.path().join("run-oracle.ps1")).unwrap();
    // The one documented command, and the defaults that make it work from
    // inside the bundle folder.
    let command = "powershell -ExecutionPolicy Bypass -File .\\run-oracle.ps1 -ConfirmThrowawayVm";
    assert!(readme.contains(command));
    for needle in [
        "$HostApp = 'excel'",
        "$Bundle = $PSScriptRoot",
        "(Join-Path $PSScriptRoot 'out')",
        "return-$envId.zip",
    ] {
        assert!(ps1.contains(needle), "run-oracle.ps1 lacks `{needle}`");
    }
    let docs =
        std::fs::read_to_string(root().join("docs/tooling/differential-testing.md")).unwrap();
    assert!(
        docs.contains(command),
        "the docs must quote the same command"
    );
    // What the lab person is asked to send is what `apply` reads.
    assert!(readme.contains("results-<id>.jsonl") && readme.contains("environment-<id>.json"));
}

#[test]
fn the_named_argument_order_program_is_ready_for_the_oracle_run() {
    let progs = o::load_corpus(&root().join("tests/vba")).unwrap();
    let p = progs
        .iter()
        .find(|p| p.id == "procedures/named_argument_evaluation_order")
        .expect("the program that settles the open question");
    assert!(o::classify(p).is_ok(), "it must be capturable");
    assert!(!real_skip().contains_key(&p.id), "and not skipped");
    assert_eq!(p.source_kind, "unverified");
}
