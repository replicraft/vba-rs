//! Realistic modules (standard, class, form code-behind, API declares) must
//! parse with zero diagnostics, in tolerant and strict mode, with LF and CRLF
//! line endings, and the tree must reproduce the source.

mod common;

use common::*;
use vba_syntax::{NodeKind, ParseOptions};

const FIXTURES: &[(&str, &str)] = &[
    (
        "standard_module.bas",
        include_str!("fixtures/standard_module.bas"),
    ),
    (
        "class_module.cls",
        include_str!("fixtures/class_module.cls"),
    ),
    (
        "form_codebehind.frm",
        include_str!("fixtures/form_codebehind.frm"),
    ),
    (
        "api_declares.bas",
        include_str!("fixtures/api_declares.bas"),
    ),
];

const PROFILES: &[&str] = &[
    "vba7.1-excel-win64",
    "vba7.1-excel-win32",
    "vba7.0-excel-win32",
];

#[test]
fn fixtures_parse_without_diagnostics() {
    for (name, text) in FIXTURES {
        let crlf = text.replace('\n', "\r\n");
        for profile in PROFILES {
            for opts in [ParseOptions::tolerant(), ParseOptions::strict()] {
                for src in [text.to_string(), crlf.clone()] {
                    let p = parse_with(&src, profile, &opts);
                    assert!(
                        p.diagnostics.is_empty(),
                        "{name} [{profile}, strict={}]: {:?}",
                        opts.strict,
                        diag_lines(&p)
                    );
                    assert!(
                        !p.tree
                            .root()
                            .descendants()
                            .iter()
                            .any(|n| n.kind() == NodeKind::Error),
                        "{name}: error node"
                    );
                }
            }
        }
    }
}

#[test]
fn legacy_branch_of_api_declares_parses_on_vba6() {
    // On VBA 6 the `#Else` branch is active; the rest of the fixture uses
    // LongPtr, so only compare the conditional section.
    let text = include_str!("fixtures/api_declares.bas");
    let head: String = text.lines().take(33).collect::<Vec<_>>().join("\n") + "\n";
    let p = parse_with(&head, "vba6-excel-win32", &ParseOptions::tolerant());
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
    let active: Vec<_> = p
        .tree
        .root()
        .child_nodes()
        .filter(|n| n.kind() == NodeKind::DeclareStmt)
        .collect();
    assert_eq!(active.len(), 3);
}

#[test]
fn exported_header_is_captured() {
    let (_, cls) = FIXTURES[1];
    let p = parse_str(cls);
    let root = p.tree.root();
    assert!(root.child_node(NodeKind::VersionLine).is_some());
    let header = root.child_node(NodeKind::HeaderBlock).expect("header");
    assert!(String::from_utf8_lossy(header.text()).contains("MultiUse"));

    let (_, frm) = FIXTURES[2];
    let p = parse_str(frm);
    let header = p
        .tree
        .root()
        .child_node(NodeKind::HeaderBlock)
        .expect("header");
    let text = String::from_utf8_lossy(header.text()).to_string();
    assert!(text.starts_with("Begin {C62A69F0"), "{text}");
    assert!(text.ends_with("End"), "{text}");
    // The module body starts right after the form header.
    assert!(
        p.tree
            .root()
            .child_nodes()
            .any(|n| n.kind() == NodeKind::AttributeStmt)
    );
}

#[test]
fn fixture_shapes() {
    let (_, bas) = FIXTURES[0];
    let p = parse_str(bas);
    let procs = p
        .tree
        .root()
        .child_nodes()
        .filter(|n| n.kind() == NodeKind::ProcDecl)
        .count();
    assert_eq!(procs, 10);
    let (_, cls) = FIXTURES[1];
    let p = parse_str(cls);
    let n = p
        .tree
        .root()
        .child_nodes()
        .filter(|n| n.kind() == NodeKind::ProcDecl)
        .count();
    assert_eq!(n, 12);
}
