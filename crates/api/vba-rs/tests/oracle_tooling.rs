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
