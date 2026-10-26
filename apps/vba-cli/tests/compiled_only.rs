//! `vbars` on projects whose modules have no source text.

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use vba_format::{WriteOptions, write_project};
use vba_project::{Module, ModuleKind, VbaProject};

fn vbars() -> Command {
    Command::cargo_bin("vbars").unwrap()
}

fn stripped(name: &str) -> Module {
    let mut m = Module::new(name, ModuleKind::Standard, "");
    m.has_source = false;
    m
}

fn save(dir: &Path, name: &str, p: &VbaProject) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, write_project(p, &WriteOptions::default()).unwrap()).unwrap();
    path
}

fn with_main(extra: &[&str]) -> VbaProject {
    let mut p = VbaProject::new("Stripped");
    p.add_module(Module::new(
        "Main",
        ModuleKind::Standard,
        "Public Sub Go()\r\nDebug.Print \"hi\"\r\nEnd Sub\r\n",
    ))
    .unwrap();
    for n in extra {
        p.add_module(stripped(n)).unwrap();
    }
    p
}

fn all_stripped(dir: &Path) -> PathBuf {
    let mut p = VbaProject::new("Stripped");
    for n in ["A", "B"] {
        p.add_module(stripped(n)).unwrap();
    }
    save(dir, "all.bin", &p)
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn inspect_reports_availability_per_project() {
    let dir = tempfile::tempdir().unwrap();
    let out = vbars()
        .arg("inspect")
        .arg(all_stripped(dir.path()))
        .assert()
        .success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        text.contains("Source availability: compiled-only"),
        "{text}"
    );
    assert!(
        text.contains("no source text for 2 module(s): A, B"),
        "{text}"
    );
    assert!(text.contains("never decompiled or run"), "{text}");
}

#[test]
fn check_parse_and_run_refuse_with_exit_code_6() {
    let dir = tempfile::tempdir().unwrap();
    let all = all_stripped(dir.path());
    let mixed = save(dir.path(), "mixed.bin", &with_main(&["Hidden"]));
    for args in [
        vec!["check"],
        vec!["parse"],
        vec!["run", "--memfs", "--entry", "Main.Go"],
    ] {
        for file in [&all, &mixed] {
            let out = vbars().args(&args).arg(file).output().unwrap();
            assert_eq!(out.status.code(), Some(6), "{args:?} {file:?}");
            let err = stderr(&out);
            assert!(err.contains("Stripped"), "{err}");
            assert!(err.contains("no source"), "{err}");
        }
    }
    // The error names the modules.
    let out = vbars().arg("check").arg(&mixed).output().unwrap();
    assert!(stderr(&out).contains("Hidden"));
}

#[test]
fn mixed_project_runs_only_with_allow_partial() {
    let dir = tempfile::tempdir().unwrap();
    let mixed = save(dir.path(), "mixed.bin", &with_main(&["Hidden"]));
    let ok = vbars()
        .args(["run", "--memfs", "--allow-partial", "--entry", "Main.Go"])
        .arg(&mixed)
        .assert()
        .success();
    assert!(String::from_utf8_lossy(&ok.get_output().stdout).starts_with("hi"));
    let out = vbars()
        .args(["run", "--memfs", "--allow-partial", "--entry", "Hidden.X"])
        .arg(&mixed)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(6));
    assert!(stderr(&out).contains("Hidden"));
    // A fully compiled-only project is refused even then.
    let out = vbars()
        .args(["run", "--memfs", "--allow-partial"])
        .arg(all_stripped(dir.path()))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(6));
}

#[test]
fn full_source_projects_are_unaffected() {
    let dir = tempfile::tempdir().unwrap();
    let full = save(dir.path(), "full.bin", &with_main(&[]));
    vbars().arg("check").arg(&full).assert().success();
    vbars()
        .args(["run", "--memfs", "--entry", "Go"])
        .arg(&full)
        .assert()
        .success();
}
