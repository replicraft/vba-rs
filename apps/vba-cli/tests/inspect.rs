//! End-to-end tests of `vbars inspect`.

use assert_cmd::Command;
use std::path::Path;
use vba_format::{WriteOptions, write_project};
use vba_project::{Module, ModuleKind, Reference, VbaProject};

fn vbars() -> Command {
    Command::cargo_bin("vbars").unwrap()
}

fn sample_project() -> VbaProject {
    let mut p = VbaProject::new("DemoProject");
    p.description = "Demo description".into();
    p.constants = "FLAG = 1".into();
    p.references.push(Reference::Registered {
        name: Some("stdole".into()),
        libid: "*\\G{00020430-0000-0000-C000-000000000046}#2.0#0#C:\\Windows\\system32\\stdole2.tlb#OLE Automation".into(),
    });
    p.add_module(Module::new(
        "Module1",
        ModuleKind::Standard,
        "Option Explicit\r\nSub A()\r\nEnd Sub\r\n",
    ))
    .unwrap();
    p.add_module(Module::new(
        "Thing",
        ModuleKind::Class,
        "Option Explicit\r\n",
    ))
    .unwrap();
    p.add_module(Module::new("ThisDocument", ModuleKind::Document, ""))
        .unwrap();
    p
}

fn write_bin(dir: &Path, name: &str, p: &VbaProject) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, write_project(p, &WriteOptions::default()).unwrap()).unwrap();
    path
}

#[test]
fn inspects_a_project_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_bin(dir.path(), "vbaProject.bin", &sample_project());
    let out = vbars().arg("inspect").arg(&path).assert().success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(text.contains("Project: DemoProject"), "{text}");
    assert!(text.contains("Description: Demo description"));
    assert!(text.contains("Code page: 1252"));
    assert!(text.contains("Platform: 32-bit Windows"));
    assert!(text.contains("Constants: FLAG = 1"));
    assert!(text.contains("Protection: none"));
    assert!(text.contains("References (1):"));
    assert!(text.contains("stdole [registered] *\\G{00020430"));
    assert!(text.contains("Modules (3):"));
    assert!(text.contains("Thing"));
    assert!(text.contains("class"));
    assert!(text.contains("document"));
    assert!(text.contains("Preserved:"));
}

#[test]
fn reports_module_kind_and_size_per_line() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_bin(dir.path(), "p.bin", &sample_project());
    let out = vbars().arg("inspect").arg(&path).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let line = text.lines().find(|l| l.contains("Module1")).unwrap();
    assert!(line.contains("standard"), "{line}");
    assert!(line.contains("35 chars"), "{line}");
    assert!(line.contains("3 lines"), "{line}");
}

#[test]
fn reports_locked_and_source_less_projects() {
    let mut p = sample_project();
    p.protection.vbe_protected = true;
    p.protection.visible = false;
    p.modules[1].has_source = false;
    p.modules[1].source.clear();
    let dir = tempfile::tempdir().unwrap();
    let path = write_bin(dir.path(), "locked.bin", &p);
    let out = vbars().arg("inspect").arg(&path).assert().success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        text.contains("Protection: locked for viewing, not visible"),
        "{text}"
    );
    assert!(text.contains("no source"), "{text}");
    assert!(
        text.contains("no source text for 1 module(s): Thing"),
        "{text}"
    );
    assert!(text.contains("Source availability: mixed"), "{text}");
    assert!(text.contains("[absent, no cache]"), "{text}");
}

#[test]
fn inspects_standalone_sources() {
    let dir = tempfile::tempdir().unwrap();
    let bas = dir.path().join("Util.bas");
    std::fs::write(
        &bas,
        "Attribute VB_Name = \"Helpers\"\r\nSub A()\r\nEnd Sub\r\n",
    )
    .unwrap();
    let out = vbars().arg("inspect").arg(&bas).assert().success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(text.contains("Module: Helpers"), "{text}");
    assert!(text.contains("Kind: standard"));
    assert!(text.contains("Export header: none"));

    let cls = dir.path().join("Thing.cls");
    std::fs::write(
        &cls,
        "VERSION 1.0 CLASS\r\nBEGIN\r\n  MultiUse = -1\r\nEND\r\nAttribute VB_Name = \"Thing\"\r\nOption Explicit\r\n",
    )
    .unwrap();
    let out = vbars().arg("inspect").arg(&cls).assert().success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(text.contains("Module: Thing"), "{text}");
    assert!(text.contains("Kind: class"));
    assert!(text.contains("Export header: 4 lines"));

    // Non-UTF-8 bytes are read as Windows-1252.
    let frm = dir.path().join("Form1.frm");
    std::fs::write(
        &frm,
        b"VERSION 5.00\r\nBegin {C62A69F0-16DC-11CE-9E98-00AA00574A4F} Form1\r\nEnd\r\nAttribute VB_Name = \"Form1\"\r\n' caf\xE9\r\n",
    )
    .unwrap();
    let out = vbars().arg("inspect").arg(&frm).assert().success();
    let text = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(text.contains("Kind: designer"), "{text}");
}

#[test]
fn missing_file_fails_cleanly() {
    let out = vbars()
        .args(["inspect", "/nonexistent/definitely/missing.bin"])
        .assert()
        .failure()
        .code(1);
    let err = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(err.starts_with("error: cannot read"), "{err}");
    assert!(out.get_output().stdout.is_empty());
}

#[test]
fn unrecognized_file_fails_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.txt");
    std::fs::write(&path, "hello").unwrap();
    let out = vbars().arg("inspect").arg(&path).assert().failure().code(1);
    let err = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(err.contains("not a recognized VBA file"), "{err}");
}

#[test]
fn corrupt_project_fails_cleanly_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let good = write_project(&sample_project(), &WriteOptions::default()).unwrap();
    // Valid signature, damaged body.
    let mut bad = good.clone();
    for b in bad.iter_mut().skip(512).step_by(3) {
        *b = 0xA5;
    }
    let truncated = &good[..good.len() / 2];
    for (name, data) in [("bad.bin", bad.as_slice()), ("trunc.bin", truncated)] {
        let path = dir.path().join(name);
        std::fs::write(&path, data).unwrap();
        let out = vbars().arg("inspect").arg(&path).assert().failure().code(1);
        let err = String::from_utf8(out.get_output().stderr.clone()).unwrap();
        assert!(err.starts_with("error: "), "{name}: {err}");
        assert!(!err.contains("panicked"), "{name}: {err}");
    }
}

#[test]
fn bad_source_files_fail_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Broken.cls");
    std::fs::write(&path, "VERSION 1.0 CLASS\r\nBEGIN\r\n").unwrap();
    let out = vbars().arg("inspect").arg(&path).assert().failure().code(1);
    let err = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(err.contains("unterminated header"), "{err}");
}

#[test]
fn usage_errors_exit_with_code_two() {
    vbars().assert().failure().code(2);
    vbars().arg("inspect").assert().failure().code(2);
    vbars().arg("bogus").assert().failure().code(2);
}

#[test]
fn help_and_version_still_work() {
    vbars().arg("--help").assert().success();
    vbars().arg("--version").assert().success();
    vbars().args(["inspect", "--help"]).assert().success();
}
