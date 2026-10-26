//! Protocol tests for `vba.typeLibraries`: type libraries built in the test
//! (MSFT, SLTG, inside a PE image), written to files and loaded by the real
//! `vba-lsp` binary.
#![allow(missing_docs)]

mod common;

// The synthetic type library writers of the `vba-typelib` tests.
#[path = "../../../crates/interoperability/vba-typelib/tests/common/mod.rs"]
mod typelib;

use std::path::{Path, PathBuf};

use common::{Client, URI};
use serde_json::{Value, json};
use typelib::pe::{Res, Ty, build_pe};

const SOURCE: &str = "\
Option Explicit
Sub Main()
    Dim q As Shapes.IShape
    Dim x As Double
    x = q.Area
    q.Scale(2, \"a\")
    x = q.
End Sub
";

/// A writable directory for this test's files.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("vba-lsp-libraries");
    let dir = dir.join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, data: &[u8]) -> String {
    let p = dir.join(name);
    std::fs::write(&p, data).unwrap();
    p.to_str().unwrap().to_string()
}

fn msft() -> Vec<u8> {
    typelib::builder::build(&typelib::sample())
}

fn sltg() -> Vec<u8> {
    typelib::sltg::build_sltg(&typelib::sltg_libs::sample())
}

fn pe_image() -> Vec<u8> {
    build_pe(
        true,
        &[Res {
            ty: Ty::Name("TYPELIB"),
            id: 1,
            lang: 0,
            data: msft(),
        }],
    )
}

fn start(options: Value) -> Client {
    let mut c = Client::spawn();
    c.request(
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {},
            "initializationOptions": options,
        }),
    );
    c.notify("initialized", json!({}));
    c
}

fn open(c: &mut Client, text: &str) {
    c.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": URI, "languageId": "vba", "version": 1, "text": text}}),
    );
}

/// The position just after the first `needle` on its line, plus `back`
/// characters towards its start.
fn after(text: &str, needle: &str, back: usize) -> Value {
    let (line, l) = text
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains(needle))
        .expect("needle");
    let col = l.find(needle).unwrap() + needle.len() - back;
    json!({"textDocument": {"uri": URI}, "position": {"line": line, "character": col}})
}

fn labels(result: &Value) -> Vec<String> {
    let items = result["items"].as_array().cloned().unwrap_or_default();
    items
        .iter()
        .map(|i| i["label"].as_str().unwrap().to_string())
        .collect()
}

fn codes(d: &[Value]) -> Vec<String> {
    d.iter()
        .filter_map(|d| d["code"].as_str().map(String::from))
        .collect()
}

fn log_containing(c: &mut Client, what: &str) -> String {
    let what = what.to_string();
    let m = c.wait(move |m| {
        m["method"] == "window/logMessage"
            && m["params"]["message"]
                .as_str()
                .is_some_and(|s| s.contains(&what))
    });
    m["params"]["message"].as_str().unwrap().to_string()
}

#[test]
fn msft_library_gives_members_hover_and_signature_help() {
    let dir = scratch("msft");
    let tlb = write(&dir, "shapes.tlb", &msft());
    let mut c = start(json!({"vba": {"typeLibraries": [tlb]}}));
    open(&mut c, SOURCE);

    // The declaration resolves: no unknown-type diagnostic.
    let d = c.diagnostics(1);
    assert!(!codes(&d).contains(&"SEM004".to_string()), "{d:?}");

    let r = c.request("textDocument/completion", after(SOURCE, "x = q.", 0));
    let l = labels(&r);
    assert!(
        l.contains(&"Area".to_string()) && l.contains(&"Scale".to_string()),
        "{l:?}"
    );

    let h = c.request("textDocument/hover", after(SOURCE, "x = q.Area", 2));
    let text = h["contents"]["value"].as_str().unwrap();
    assert!(text.contains("Area") && text.contains("Double"), "{text}");

    let s = c.request(
        "textDocument/signatureHelp",
        after(SOURCE, "q.Scale(2, ", 0),
    );
    let sig = &s["signatures"][0];
    assert!(sig["label"].as_str().unwrap().contains("Scale"), "{s}");
    assert_eq!(s["activeParameter"], 1);

    // Library members have nothing to jump to: a null answer, not an error.
    let def = c.request("textDocument/definition", after(SOURCE, "x = q.Area", 2));
    assert!(def.is_null(), "{def}");
}

#[test]
fn sltg_and_pe_libraries_load_too() {
    let dir = scratch("formats");
    let sltg_file = write(&dir, "shapes.olb", &sltg());
    let pe_file = write(&dir, "shapes.dll", &pe_image());
    for file in [sltg_file, pe_file] {
        let mut c = start(json!({"vba": {"typeLibraries": [file]}}));
        open(&mut c, SOURCE);
        let r = c.request("textDocument/completion", after(SOURCE, "x = q.", 0));
        assert!(labels(&r).contains(&"Area".to_string()), "{file}: {r}");
    }
}

#[test]
fn without_libraries_the_type_is_unknown() {
    let mut c = start(json!({"vba": {}}));
    open(&mut c, SOURCE);
    let d = c.diagnostics(1);
    assert!(codes(&d).contains(&"SEM004".to_string()), "{d:?}");
    let r = c.request("textDocument/completion", after(SOURCE, "x = q.", 0));
    assert!(!labels(&r).contains(&"Area".to_string()), "{r}");
}

#[test]
fn configuration_changes_load_and_unload_libraries() {
    let dir = scratch("reload");
    let tlb = write(&dir, "shapes.tlb", &msft());
    let mut c = start(json!({"vba": {}}));
    open(&mut c, SOURCE);
    let d = c.diagnostics(1);
    assert!(codes(&d).contains(&"SEM004".to_string()), "{d:?}");

    c.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"vba": {"typeLibraries": [tlb]}}}),
    );
    log_containing(&mut c, "loaded 1 type libraries");
    let d = c.diagnostics(1);
    assert!(!codes(&d).contains(&"SEM004".to_string()), "{d:?}");
    let r = c.request("textDocument/completion", after(SOURCE, "x = q.", 0));
    assert!(labels(&r).contains(&"Scale".to_string()), "{r}");

    // Removing the setting removes the libraries.
    c.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"vba": {}}}),
    );
    let d = c.diagnostics(1);
    assert!(codes(&d).contains(&"SEM004".to_string()), "{d:?}");
    let r = c.request("textDocument/completion", after(SOURCE, "x = q.", 0));
    assert!(!labels(&r).contains(&"Scale".to_string()), "{r}");
}

#[test]
fn bad_files_are_reported_and_the_rest_still_load() {
    let dir = scratch("errors");
    let good = write(&dir, "good.tlb", &msft());
    let garbage = write(&dir, "garbage.tlb", b"this is not a type library at all");
    let truncated = write(&dir, "cut.tlb", &msft()[..40]);
    let missing = dir.join("missing.tlb").to_str().unwrap().to_string();
    let folder = dir.to_str().unwrap().to_string();
    let mut c = start(json!({"vba": {"typeLibraries": [
        missing, garbage, truncated, folder, good, 7
    ]}}));
    for name in ["missing.tlb", "garbage.tlb", "cut.tlb"] {
        let m = log_containing(&mut c, name);
        assert!(m.contains("type library"), "{m}");
    }
    log_containing(&mut c, "entries must be non-empty strings");
    log_containing(&mut c, "loaded 1 type libraries");
    open(&mut c, SOURCE);
    let r = c.request("textDocument/completion", after(SOURCE, "x = q.", 0));
    assert!(labels(&r).contains(&"Area".to_string()), "{r}");

    // A malformed setting is a warning, not a failure.
    c.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"vba": {"typeLibraries": "shapes.tlb"}}}),
    );
    log_containing(&mut c, "must be a list of paths");
    let r = c.request("textDocument/hover", after(SOURCE, "x = q.Area", 2));
    assert!(r.is_null() || r.is_object());
}
