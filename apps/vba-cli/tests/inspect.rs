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

/// Writes the project's streams as a directory of files named by stream path.
fn write_dir(root: &Path, p: &VbaProject, patch_vp: Option<[u8; 2]>) -> std::path::PathBuf {
    let (streams, _) = vba_format::write_project_streams(p, &WriteOptions::default()).unwrap();
    let dir = root.join("streams");
    for (path, bytes) in streams.iter() {
        let mut bytes = bytes.to_vec();
        if path == "VBA/_VBA_PROJECT"
            && let Some(w) = patch_vp
        {
            bytes[2..4].copy_from_slice(&w);
        }
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, bytes).unwrap();
    }
    dir
}

fn stdout_of(args: &[&Path]) -> String {
    let out = vbars().arg("inspect").args(args).assert().success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

fn json_of(path: &Path) -> serde_json::Value {
    let out = vbars()
        .arg("inspect")
        .arg(path)
        .arg("--json")
        .assert()
        .success();
    serde_json::from_slice(&out.get_output().stdout).expect("stdout is one JSON document")
}

#[test]
fn reports_version_word_platform_and_hint() {
    let root = tempfile::tempdir().unwrap();
    let dir = write_dir(root.path(), &sample_project(), Some([0x97, 0x00]));
    let text = stdout_of(&[&dir]);
    assert!(text.contains("Project: DemoProject"), "{text}");
    assert!(text.contains("Version word: 0x0097"), "{text}");
    assert!(text.contains("Platform: 32-bit Windows"), "{text}");
    assert!(
        text.contains("Profile hint: vba7.0-excel-win32 (hypothesis)"),
        "{text}"
    );
    assert!(text.contains("Reference version hints"), "{text}");
    assert!(
        text.contains("guid={00020430-0000-0000-C000-000000000046}"),
        "{text}"
    );
    assert!(text.contains("typelib=2.0"), "{text}");

    // The no-cache word and any unknown word give no hint.
    for word in [[0xFF, 0xFF], [0x34, 0x12]] {
        let dir = write_dir(root.path(), &sample_project(), Some(word));
        let text = stdout_of(&[&dir]);
        assert!(text.contains("Profile hint: unknown"), "{text}");
    }
}

#[test]
fn directory_and_compound_file_inputs_agree() {
    let root = tempfile::tempdir().unwrap();
    let p = sample_project();
    let bin = write_bin(root.path(), "p.bin", &p);
    let dir = write_dir(root.path(), &p, None);
    let from_bin = stdout_of(&[&bin]);
    let from_dir = stdout_of(&[&dir]);
    // Only the preserved-data summary differs: a directory has no storages.
    let strip = |t: &str| {
        t.lines()
            .filter(|l| !l.starts_with("Preserved:"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(strip(&from_bin), strip(&from_dir));
}

#[test]
fn directory_stream_names_are_case_insensitive() {
    let root = tempfile::tempdir().unwrap();
    let dir = write_dir(root.path(), &sample_project(), None);
    std::fs::rename(dir.join("VBA"), dir.join("vba_tmp")).unwrap();
    std::fs::rename(dir.join("vba_tmp"), dir.join("vba")).unwrap();
    std::fs::rename(dir.join("PROJECT"), dir.join("project")).unwrap();
    let text = stdout_of(&[&dir]);
    assert!(text.contains("Modules (3):"), "{text}");
}

#[test]
fn directory_errors_are_clean() {
    let root = tempfile::tempdir().unwrap();
    let empty = root.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    let out = vbars()
        .arg("inspect")
        .arg(&empty)
        .assert()
        .failure()
        .code(1);
    let err = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(err.starts_with("error: "), "{err}");
    assert!(err.contains("VBA/dir"), "{err}");

    let dir = write_dir(root.path(), &sample_project(), None);
    std::fs::write(dir.join("VBA").join("dir"), [0xFF; 12]).unwrap();
    let out = vbars().arg("inspect").arg(&dir).assert().failure().code(1);
    let err = String::from_utf8(out.get_output().stderr.clone()).unwrap();
    assert!(err.starts_with("error: "), "{err}");
    assert!(!err.contains("panicked"), "{err}");
}

#[test]
fn json_output_has_a_stable_documented_shape() {
    let root = tempfile::tempdir().unwrap();
    let dir = write_dir(root.path(), &sample_project(), Some([0x97, 0x00]));
    let v = json_of(&dir);
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["kind"], "project");
    assert_eq!(v["input"], "directory");
    assert_eq!(v["name"], "DemoProject");
    assert_eq!(v["code_page"], 1252);
    assert_eq!(v["lcid"], 0x409);
    assert_eq!(v["platform"]["name"], "32-bit Windows");
    assert_eq!(v["platform"]["value"], 1);
    assert_eq!(v["version_word"], 0x97);
    assert_eq!(v["version_word_hex"], "0x0097");
    assert_eq!(v["profile_hint"], "vba7.0-excel-win32");
    assert_eq!(v["compiled_only"], false);
    assert_eq!(v["source_availability"], "full source");
    assert_eq!(v["protection"]["visible"], true);
    assert_eq!(v["references"][0]["kind"], "registered");
    let h = &v["reference_hints"][0];
    assert_eq!(h["guid"], "{00020430-0000-0000-C000-000000000046}");
    assert_eq!(h["type_lib_version"], "2.0");
    assert_eq!(h["office_dir"], serde_json::Value::Null);
    assert_eq!(v["modules"].as_array().unwrap().len(), 3);
    assert_eq!(v["modules"][0]["name"], "Module1");
    assert_eq!(v["modules"][0]["has_source"], true);

    // Same input, same bytes.
    let again = vbars()
        .arg("inspect")
        .arg(&dir)
        .arg("--json")
        .output()
        .unwrap();
    let first = vbars()
        .arg("inspect")
        .arg(&dir)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(again.stdout, first.stdout);

    // Unknown word: null hint, not a missing key.
    let dir = write_dir(root.path(), &sample_project(), None);
    let v = json_of(&dir);
    assert_eq!(v["version_word"], 0xFFFF);
    assert!(v["profile_hint"].is_null());
    assert!(v.get("profile_hint").is_some());
}

#[test]
fn json_for_compound_files_and_sources() {
    let root = tempfile::tempdir().unwrap();
    let bin = write_bin(root.path(), "p.bin", &sample_project());
    let v = json_of(&bin);
    assert_eq!(v["input"], "compound-file");
    assert_eq!(v["schema_version"], 1);

    let bas = root.path().join("Util.bas");
    std::fs::write(&bas, "Sub A()\r\nEnd Sub\r\n").unwrap();
    let v = json_of(&bas);
    assert_eq!(v["kind"], "source");
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["module"]["kind"], "standard");
    assert_eq!(v["module"]["lines"], 2);
}

#[test]
fn json_project_without_source() {
    let mut p = sample_project();
    for m in &mut p.modules {
        m.has_source = false;
        m.source.clear();
        m.preserved
            .insert(vba_project::PERFORMANCE_CACHE_KEY, vec![1, 2, 3]);
    }
    let root = tempfile::tempdir().unwrap();
    let v = json_of(&write_bin(root.path(), "c.bin", &p));
    assert_eq!(v["compiled_only"], true);
    assert_eq!(v["modules"][0]["source_availability"], "absent, no cache");
    assert_eq!(v["source_availability"], "compiled-only");
}
