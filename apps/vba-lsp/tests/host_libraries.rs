//! Protocol tests for `vba.hostLibraries`: a described host library (the
//! JSON an embedding application generates) given to the real `vba-lsp`
//! binary as a file or inline, for completion, hover, signature help and
//! diagnostics.
#![allow(missing_docs)]

mod common;

use std::path::{Path, PathBuf};

use common::{Client, URI};
use serde_json::{Value, json};
use vba_host_model::{HostClass, HostEnum, HostGlobal, HostLibrary, HostMember, HostParam};

fn library() -> HostLibrary {
    HostLibrary::new("DbLib")
        .enumeration(HostEnum::new("RecordsetType").documented("dbOpenTable", 1, "A table."))
        .class(
            HostClass::new("Recordset")
                .doc("Rows of a table.")
                .member(HostMember::property("RecordCount", "Long").read_only())
                .member(
                    HostMember::function("FindFirst", "Boolean")
                        .param(HostParam::new("Criteria", "String"))
                        .doc("Finds the first matching row."),
                ),
        )
        .class(
            HostClass::new("Application").member(
                HostMember::function("OpenRecordset", "Recordset")
                    .param(HostParam::new("Name", "String")),
            ),
        )
        .global(HostGlobal::new("Application", "Application"))
        .function(
            HostMember::function("Nz", "Variant")
                .param(HostParam::new("Value", "Variant"))
                .param(HostParam::new("ValueIfNull", "Variant").optional())
                .doc("Replaces Null by another value."),
        )
}

const SOURCE: &str = "\
Option Explicit
Sub Main()
    Dim rs As Recordset
    Dim n As Long
    Set rs = Application.OpenRecordset(\"t\")
    n = rs.RecordCount
    rs.Bogus
    n = rs.FindFirst(\"a\")
    Debug.Print Nz(Null, 1)
End Sub
";

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("vba-lsp-host-libraries");
    let dir = dir.join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn start(options: Value) -> Client {
    let mut c = Client::spawn();
    c.request(
        "initialize",
        json!({"processId": null, "rootUri": null, "capabilities": {},
               "initializationOptions": options}),
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

fn at(text: &str, needle: &str, back: usize) -> Value {
    let (line, l) = text
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains(needle))
        .expect("needle");
    let col = l.find(needle).unwrap() + needle.len() - back;
    json!({"textDocument": {"uri": URI}, "position": {"line": line, "character": col}})
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

fn check_features(c: &mut Client) {
    open(c, SOURCE);
    let d = c.diagnostics(1);
    // `rs.Bogus` is the one error the description rules out.
    assert_eq!(codes(&d), ["SEM019"], "{d:?}");

    let r = c.request("textDocument/completion", at(SOURCE, "n = rs.", 0));
    let items = r["items"].as_array().unwrap();
    let labels: Vec<&str> = items.iter().map(|i| i["label"].as_str().unwrap()).collect();
    assert_eq!(labels, ["FindFirst", "RecordCount"]);
    let find = &items[0];
    assert_eq!(
        find["documentation"]["value"],
        "Finds the first matching row."
    );
    assert!(
        find["detail"]
            .as_str()
            .unwrap()
            .starts_with("Function Recordset.FindFirst(ByVal Criteria As String)")
    );

    let h = c.request("textDocument/hover", at(SOURCE, "Debug.Print Nz", 1));
    let text = h["contents"]["value"].as_str().unwrap();
    assert!(text.contains("Function DbLib.Nz("), "{text}");
    assert!(text.contains("Replaces Null by another value."), "{text}");

    let s = c.request("textDocument/signatureHelp", at(SOURCE, "Nz(Null, ", 0));
    assert_eq!(s["activeParameter"], 1);
    assert!(
        s["signatures"][0]["label"]
            .as_str()
            .unwrap()
            .starts_with("Function DbLib.Nz(")
    );
}

#[test]
fn a_description_file_drives_every_feature() {
    let dir = scratch("file");
    let path = dir.join("db.json");
    std::fs::write(&path, library().to_json_string()).unwrap();
    let mut c = start(json!({"vba": {"hostLibraries": [path.to_str().unwrap()]}}));
    log_containing(&mut c, "loaded 1 host libraries");
    check_features(&mut c);
}

#[test]
fn an_inline_description_works_too() {
    let mut c = start(json!({"vba": {"hostLibraries": [library().to_json()]}}));
    check_features(&mut c);
}

#[test]
fn without_the_library_nothing_is_known() {
    let mut c = start(json!({"vba": {}}));
    open(&mut c, SOURCE);
    let d = c.diagnostics(1);
    assert!(codes(&d).contains(&"SEM004".to_string()), "{d:?}");
}

#[test]
fn invalid_descriptions_are_reported_and_the_rest_load() {
    let dir = scratch("errors");
    let good = dir.join("good.json");
    std::fs::write(&good, library().to_json_string()).unwrap();
    let invalid = dir.join("invalid.json");
    let bad = HostLibrary::new("Bad")
        .class(HostClass::new("C").member(HostMember::function("F", "Nowhere")));
    std::fs::write(&invalid, bad.to_json_string()).unwrap();
    let garbage = dir.join("garbage.json");
    std::fs::write(&garbage, "{ not json").unwrap();
    let wrong = dir.join("wrong.json");
    std::fs::write(&wrong, r#"{"format": "vba-host-library", "version": 9}"#).unwrap();
    let missing = dir.join("missing.json");

    let mut c = start(json!({"vba": {"hostLibraries": [
        missing.to_str().unwrap(), garbage.to_str().unwrap(), wrong.to_str().unwrap(),
        invalid.to_str().unwrap(), good.to_str().unwrap(), 7
    ]}}));
    let m = log_containing(&mut c, "invalid.json");
    assert!(m.contains("unknown type `Nowhere`"), "{m}");
    log_containing(&mut c, "garbage.json");
    let m = log_containing(&mut c, "wrong.json");
    assert!(m.contains("unsupported format version 9"), "{m}");
    log_containing(&mut c, "missing.json");
    log_containing(
        &mut c,
        "entries must be a file path or a description object",
    );
    log_containing(&mut c, "loaded 1 host libraries");
    check_features(&mut c);
}

#[test]
fn changing_the_setting_reloads() {
    let mut c = start(json!({"vba": {}}));
    open(&mut c, SOURCE);
    let d = c.diagnostics(1);
    assert!(codes(&d).contains(&"SEM004".to_string()), "{d:?}");
    c.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"vba": {"hostLibraries": [library().to_json()]}}}),
    );
    log_containing(&mut c, "loaded 1 host libraries");
    let d = c.diagnostics(1);
    assert_eq!(codes(&d), ["SEM019"], "{d:?}");
    c.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"vba": {}}}),
    );
    let d = c.diagnostics(1);
    assert!(codes(&d).contains(&"SEM004".to_string()), "{d:?}");
}

#[test]
fn module_hosts_scope_a_form_to_its_class() {
    let forms = HostLibrary::new("FormLib")
        .class(HostClass::new("TextBox").member(HostMember::property("Value", "Variant")))
        .class(
            HostClass::new("Form_Orders")
                .member(HostMember::property("txtName", "TextBox").read_only())
                .member(HostMember::sub("Requery").doc("Runs the query again.")),
        );
    let src = "Option Explicit\nSub Main()\n    txtName.Value = 1\n    txtOther.Value = 2\n    Requery\nEnd Sub\n";
    let mut c = start(json!({"vba": {
        "hostLibraries": [forms.to_json()],
        "moduleKinds": {"Module1.bas": "document"},
        "moduleHosts": {"Module1": "Form_Orders"}
    }}));
    open(&mut c, src);
    let d = c.diagnostics(1);
    assert_eq!(codes(&d), ["SEM002"], "{d:?}");
    assert!(
        d[0]["message"]
            .as_str()
            .unwrap()
            .contains("host class `FormLib.Form_Orders`"),
        "{d:?}"
    );
    let r = c.request("textDocument/completion", at(src, "    Requery", 7));
    let labels: Vec<&str> = r["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["label"].as_str().unwrap())
        .collect();
    assert!(
        labels.contains(&"txtName") && labels.contains(&"Requery"),
        "{labels:?}"
    );
    // A changed setting reloads: without the scope the form is not special.
    c.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"vba": {"hostLibraries": [forms.to_json()],
            "moduleKinds": {"Module1.bas": "document"}}}}),
    );
    let d = c.diagnostics(1);
    assert!(!codes(&d).is_empty(), "{d:?}");
}
