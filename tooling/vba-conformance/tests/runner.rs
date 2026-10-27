//! Tests of the corpus runner on a small synthetic corpus.

use std::collections::BTreeSet;

use vba::{Capability, Engine};
use vba_conformance::{Options, Outcome, Report, run_corpus};

const PASS: &str = "' INV: x.a\n' SOURCE: derived test\n' EXPECT:  3 \nPublic Sub Main()\n    Debug.Print 1 + 2\nEnd Sub\n";
const WRONG: &str = "' INV: x.a\n' SOURCE: derived test\n' EXPECT: 4\nPublic Sub Main()\n    Debug.Print 1 + 2\nEnd Sub\n";
const NEEDS_HOST: &str = "' INV: x.a\n' SOURCE: derived test\n' REQUIRES: host-widget\n' EXPECT: nope\nPublic Sub Main()\nEnd Sub\n";
const NEEDS_GRANT: &str = "' INV: x.a\n' SOURCE: derived test\n' GRANT: file-write\n' EXPECT:  3 \nPublic Sub Main()\n    Debug.Print 1 + 2\nEnd Sub\n";
const XFAIL_PASSES: &str = "' INV: x.a\n' SOURCE: derived test\n' XFAIL: wrong reason\n' EXPECT:  3 \nPublic Sub Main()\n    Debug.Print 1 + 2\nEnd Sub\n";

fn corpus() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("area")).unwrap();
    for (name, text) in [
        ("pass", PASS),
        ("wrong", WRONG),
        ("needs_host", NEEDS_HOST),
        ("needs_grant", NEEDS_GRANT),
        ("xfail_passes", XFAIL_PASSES),
    ] {
        std::fs::write(dir.path().join("area").join(format!("{name}.bas")), text).unwrap();
    }
    dir
}

fn outcome<'a>(r: &'a Report, id: &str) -> &'a Outcome {
    &r.outcomes.iter().find(|(i, _)| i == id).unwrap().1
}

#[test]
fn everything_provided_runs_everything() {
    let dir = corpus();
    let r = run_corpus(dir.path(), Engine::new, &Options::all()).unwrap();
    assert_eq!(outcome(&r, "area/pass"), &Outcome::Passed);
    assert!(matches!(outcome(&r, "area/wrong"), Outcome::Failed(_)));
    // Provided by default, so the program runs and its expectation fails.
    assert!(matches!(outcome(&r, "area/needs_host"), Outcome::Failed(_)));
    assert_eq!(outcome(&r, "area/needs_grant"), &Outcome::Passed);
    assert!(matches!(
        outcome(&r, "area/xfail_passes"),
        Outcome::UnexpectedPass(_)
    ));
    assert_eq!(r.skipped(), 0);
    assert!(!r.is_ok());
}

#[test]
fn a_restricted_host_skips_by_header_and_by_grant() {
    let dir = corpus();
    let options = Options {
        provides: Some(BTreeSet::new()),
        ..Options::default()
    };
    let r = run_corpus(dir.path(), Engine::new, &options).unwrap();
    assert_eq!(
        outcome(&r, "area/needs_host"),
        &Outcome::Skipped(vec!["host-widget".into()])
    );
    assert_eq!(
        outcome(&r, "area/needs_grant"),
        &Outcome::Skipped(vec!["grant:file-write".into()])
    );
    assert_eq!(outcome(&r, "area/pass"), &Outcome::Passed);
    assert_eq!(r.skipped(), 2);
}

#[test]
fn providing_the_feature_runs_the_program() {
    let dir = corpus();
    let options = Options {
        provides: Some(["grant:file-write".to_owned()].into()),
        ..Options::default()
    };
    let r = run_corpus(dir.path(), Engine::new, &options).unwrap();
    assert_eq!(outcome(&r, "area/needs_grant"), &Outcome::Passed);
}

#[test]
fn the_filter_selects_by_substring_and_the_factory_is_used() {
    let dir = corpus();
    let options = Options {
        filter: vec!["area/pass".into()],
        ..Options::default()
    };
    let r = run_corpus(
        dir.path(),
        || Engine::new().grant(Capability::FileRead),
        &options,
    )
    .unwrap();
    assert_eq!(r.outcomes.len(), 1);
    assert!(r.is_ok());
}

#[test]
fn malformed_programs_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bad.bas"), "Public Sub Main()\nEnd Sub\n").unwrap();
    std::fs::write(
        dir.path().join("badreq.bas"),
        "' INV: a\n' SOURCE: derived t\n' REQUIRES:\n",
    )
    .unwrap();
    let errors = run_corpus(dir.path(), Engine::new, &Options::all()).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
}
