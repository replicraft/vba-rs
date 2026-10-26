//! Members of library and host types, through an injected type provider.
#![allow(missing_docs)]

mod common;

use std::sync::Arc;

use common::{URI, cursor};
use vba_lsp_core::{CompletionKind, Workspace};
use vba_sema::{LibMember, LibParam, StaticLibrary, Type};

fn provider() -> StaticLibrary {
    StaticLibrary::new("Scripting")
        .with_type(
            "Dictionary",
            vec![
                LibMember::sub(
                    "Add",
                    vec![
                        LibParam::required("Key", Type::Variant),
                        LibParam::required("Item", Type::Variant),
                    ],
                ),
                LibMember::function(
                    "Exists",
                    vec![LibParam::required("Key", Type::Variant)],
                    Type::Boolean,
                ),
                LibMember::property("Count", Type::Long).read_only(),
            ],
        )
        .as_interface("Dictionary")
}

fn open(src: &str) -> (Workspace, vba_lsp_core::Position) {
    let (text, pos) = cursor(src);
    let mut ws = Workspace::new();
    ws.set_type_provider(Some(Arc::new(provider())));
    ws.open(URI, &text, 1);
    (ws, pos)
}

const PRELUDE: &str = "Sub Run()\n    Dim d As Scripting.Dictionary\n";

#[test]
fn members_complete_after_dot() {
    let (ws, pos) = open(&format!("{PRELUDE}    d.|\nEnd Sub\n"));
    let c = ws.view(URI).unwrap().completions(pos);
    let labels: Vec<_> = c.items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, ["Add", "Count", "Exists"]);
    let count = c.items.iter().find(|i| i.label == "Count").unwrap();
    assert_eq!(count.kind, CompletionKind::Property);
    assert_eq!(
        count.detail.as_deref(),
        Some("Property Get Dictionary.Count As Long")
    );
}

#[test]
fn without_provider_nothing_is_offered() {
    let (text, pos) = cursor(&format!("{PRELUDE}    d.|\nEnd Sub\n"));
    let mut ws = Workspace::new();
    ws.open(URI, &text, 1);
    let c = ws.view(URI).unwrap().completions(pos);
    assert!(c.items.iter().all(|i| i.label != "Exists"));
}

#[test]
fn hover_shows_member_signature() {
    let (ws, pos) = open(&format!("{PRELUDE}    d.Ex|ists 1\nEnd Sub\n"));
    let h = ws.view(URI).unwrap().hover(pos).expect("hover");
    assert!(
        h.contents
            .contains("Function Dictionary.Exists(ByVal Key As Variant) As Boolean"),
        "{}",
        h.contents
    );
    assert!(h.contents.contains("Scripting.Dictionary"));
}

#[test]
fn signature_help_for_member_call() {
    let (ws, pos) = open(&format!("{PRELUDE}    d.Add 1, |\nEnd Sub\n"));
    let s = ws.view(URI).unwrap().signature_help(pos).expect("help");
    assert_eq!(
        s.signatures[0].label,
        "Sub Dictionary.Add(ByVal Key As Variant, ByVal Item As Variant)"
    );
    assert_eq!(s.active_parameter, Some(1));
    let (ws, pos) = open(&format!("{PRELUDE}    x = d.Exists(|)\nEnd Sub\n"));
    let s = ws.view(URI).unwrap().signature_help(pos).expect("help");
    assert_eq!(s.active_parameter, Some(0));
}

#[test]
fn library_names_are_known_to_the_analysis() {
    let (ws, _) = open(&format!("{PRELUDE}    |\nEnd Sub\n"));
    let diags = ws.view(URI).unwrap().diagnostics();
    assert!(diags.iter().all(|d| d.code != "SEM004"), "{diags:?}");
}
