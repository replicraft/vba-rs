//! Conditional compilation (MS-VBAL §4), version gating and strict mode.

mod common;

use common::*;
use vba_syntax::{NodeKind, ParseOptions};

fn kinds(p: &vba_syntax::Parse) -> Vec<NodeKind> {
    p.tree.root().child_nodes().map(|n| n.kind()).collect()
}

fn inactive_texts(p: &vba_syntax::Parse) -> Vec<String> {
    p.tree
        .root()
        .descendants()
        .into_iter()
        .filter(|n| n.kind() == NodeKind::InactiveText)
        .map(|n| String::from_utf8_lossy(n.text()).to_string())
        .collect()
}

#[test]
fn active_branch_is_parsed_inactive_is_kept_as_text() {
    let src = "#If Win64 Then\nDim a As Long\n#Else\nDim b As Long\n#End If\n";
    let p = parse_with(src, "vba7.1-excel-win64", &ParseOptions::tolerant());
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
    assert_eq!(
        kinds(&p),
        vec![
            NodeKind::CcIf,
            NodeKind::VarDeclStmt,
            NodeKind::CcElse,
            NodeKind::InactiveText,
            NodeKind::CcEndIf
        ]
    );
    assert_eq!(inactive_texts(&p), vec!["Dim b As Long\n".to_string()]);

    // On a 32-bit profile the other branch is active.
    let p = parse_with(src, "vba7.1-excel-win32", &ParseOptions::tolerant());
    assert_eq!(
        kinds(&p),
        vec![
            NodeKind::CcIf,
            NodeKind::InactiveText,
            NodeKind::CcElse,
            NodeKind::VarDeclStmt,
            NodeKind::CcEndIf
        ]
    );
    let cc_if = p.tree.root().child_node(NodeKind::CcIf).unwrap();
    assert_eq!(cc_if.value(), Some(false));
}

#[test]
fn inactive_text_is_not_parsed_and_its_lexer_errors_are_dropped() {
    let src = "#If False Then\nthis is ~ not \" valid VBA @@\nSub (((\n#End If\nOption Explicit\n";
    let p = parse_str(src);
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
    assert_eq!(inactive_texts(&p).len(), 1);
}

#[test]
fn elseif_chain_picks_first_true_branch() {
    let src = "#If Win32 And Not Win64 Then\nDim a1\n#ElseIf Win64 Then\nDim a2\n#ElseIf True Then\nDim a3\n#Else\nDim a4\n#End If\n";
    let p = parse_str(src);
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
    let decls: Vec<_> = p
        .tree
        .root()
        .child_nodes()
        .filter(|n| n.kind() == NodeKind::VarDeclStmt)
        .map(|n| String::from_utf8_lossy(n.text()).to_string())
        .collect();
    assert_eq!(decls, vec!["Dim a2"]);
    assert_eq!(inactive_texts(&p).len(), 3);
}

#[test]
fn nested_blocks_inside_inactive_branch_are_swallowed_and_paired() {
    let src =
        "#If False Then\n#If True Then\nDim x\n#Else\nDim y\n#End If\n#Else\nDim z\n#End If\n";
    let p = parse_str(src);
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
    let decls: Vec<_> = p
        .tree
        .root()
        .child_nodes()
        .filter(|n| n.kind() == NodeKind::VarDeclStmt)
        .map(|n| String::from_utf8_lossy(n.text()).to_string())
        .collect();
    assert_eq!(decls, vec!["Dim z"]);
    // Only the outer directives appear as nodes.
    let cc: Vec<_> = kinds(&p)
        .into_iter()
        .filter(|k| {
            matches!(
                k,
                NodeKind::CcIf | NodeKind::CcElse | NodeKind::CcEndIf | NodeKind::CcElseIf
            )
        })
        .collect();
    assert_eq!(
        cc,
        vec![NodeKind::CcIf, NodeKind::CcElse, NodeKind::CcEndIf]
    );
}

#[test]
fn directives_inside_procedures_split_statements() {
    let src = "Sub T()\n#If Mac Then\nx = 1\n#Else\nx = 2\n#End If\nEnd Sub\n";
    let p = parse_str(src);
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
    let s = sexp(&p);
    assert!(s.contains("(CcIf #If Mac Then)"), "{s}");
    assert!(s.contains("(InactiveText"), "{s}");
    assert!(
        s.contains("(AssignStmt (NameExpr x) = (LiteralExpr 2))"),
        "{s}"
    );
}

#[test]
fn directives_may_split_a_procedure_header_pair() {
    // A branch may contain just the header of a procedure.
    let src = "#If Win64 Then\nSub A()\n#Else\nSub B()\n#End If\nEnd Sub\n";
    let p = parse_str(src);
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
}

#[test]
fn const_directive_defines_names_for_later_conditions() {
    let src = "#Const DEBUGGING = True\n#Const LEVEL = 2 + 1\n#If DEBUGGING And LEVEL = 3 Then\nDim a\n#Else\nDim b\n#End If\n";
    let p = parse_str(src);
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
    assert_eq!(inactive_texts(&p), vec!["Dim b\n".to_string()]);
}

#[test]
fn const_inside_inactive_branch_is_not_defined() {
    let src =
        "#If False Then\n#Const X = True\n#End If\n#If X Then\nDim a\n#Else\nDim b\n#End If\n";
    let p = parse_str(src);
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
    assert_eq!(inactive_texts(&p).len(), 2);
    assert!(inactive_texts(&p)[1].contains("Dim a"));
}

#[test]
fn project_level_constants() {
    let src = "#If FEATURE = 1 Then\nDim a\n#Else\nDim b\n#End If\n";
    let opts = ParseOptions::tolerant().with_constant("Feature", 1);
    let p = parse_with(src, "vba7.1-excel-win64", &opts);
    assert!(inactive_texts(&p)[0].contains("Dim b"));
    let p = parse_str(src);
    assert!(inactive_texts(&p)[0].contains("Dim a"));
}

#[test]
fn undefined_names_are_empty_and_false() {
    let p = parse_str("#If Nonexistent Then\nDim a\n#End If\n");
    assert!(p.diagnostics.is_empty());
    assert_eq!(inactive_texts(&p).len(), 1);
}

#[test]
fn predefined_constants_follow_the_profile() {
    let src = "#If VBA7 Then\nDim a\n#Else\nDim b\n#End If\n";
    let p = parse_with(src, "vba6-excel-win32", &ParseOptions::tolerant());
    assert!(inactive_texts(&p)[0].contains("Dim a"));
    let p = parse_with(src, "vba7.1-excel-win32", &ParseOptions::tolerant());
    assert!(inactive_texts(&p)[0].contains("Dim b"));
    let p = parse_with(
        "#If Mac Then\nDim a\n#End If\n",
        "vba7.1-excel-mac64",
        &ParseOptions::tolerant(),
    );
    assert!(inactive_texts(&p).is_empty());
}

#[test]
fn directive_errors() {
    for (src, msg) in [
        ("#If True Then\nDim a\n", "without matching `#End If`"),
        ("#End If\n", "without `#If`"),
        ("#Else\n", "without `#If`"),
        ("#ElseIf True Then\n", "without `#If`"),
        (
            "#If True Then\n#Else\n#Else\n#End If\n",
            "duplicate `#Else`",
        ),
        (
            "#If True Then\n#Else\n#ElseIf True Then\n#End If\n",
            "after `#Else`",
        ),
        ("#If True\nDim a\n#End If\n", "expected `Then`"),
        ("#If (1 Then\n#End If\n", "expected `)`"),
        ("#If 1 / 0 Then\n#End If\n", "division by zero"),
        ("#Const = 1\n", "expected `#Const name = expression`"),
        ("#If \"a\" + 1 Then\n#End If\n", "type mismatch"),
    ] {
        let p = parse_str(src);
        assert!(
            p.diagnostics.iter().any(|d| d.message.contains(msg)),
            "{src:?}: {:?}",
            diag_lines(&p)
        );
    }
}

#[test]
fn directive_values_and_operators() {
    // (condition, expected value)
    for (cond, want) in [
        ("1", true),
        ("0", false),
        ("-1", true),
        ("Not 0", true),
        ("Not True", false),
        ("2 > 1", true),
        ("2 < 1", false),
        ("1 <> 2", true),
        ("1 + 1 = 2", true),
        ("2 * 3 = 6", true),
        ("7 \\ 2 = 3", true),
        ("7 Mod 3 = 1", true),
        ("2 ^ 3 = 8", true),
        ("\"a\" & \"b\" = \"ab\"", true),
        ("True Imp False", false),
        ("False Imp False", true),
        ("True Eqv True", true),
        ("True Xor True", false),
        ("True Or False", true),
        ("&H10 = 16", true),
        ("(1 + 2) * 3 = 9", true),
        ("Win32", true),
        ("Win16", false),
        ("Mac", false),
        ("VBA6 And VBA7", true),
        ("Empty", false),
    ] {
        let src = format!("#If {cond} Then\n#End If\n");
        let p = parse_str(&src);
        assert!(p.diagnostics.is_empty(), "{cond}: {:?}", diag_lines(&p));
        let n = p.tree.root().child_node(NodeKind::CcIf).unwrap();
        assert_eq!(n.value(), Some(want), "{cond}");
    }
}

#[test]
fn inactive_region_round_trips_crlf_and_continuations() {
    let src = "#If False Then\r\nx = 1 + _\r\n  2 ' c\r\n#End If\r\n";
    let p = parse_str(src);
    assert!(p.diagnostics.is_empty());
}

// ---- version gating ------------------------------------------------------

#[test]
fn ptrsafe_and_longptr_require_vba7() {
    let src = "Declare PtrSafe Function F Lib \"k\" () As LongPtr\n";
    let ok = parse_with(src, "vba7.1-excel-win64", &ParseOptions::tolerant());
    assert!(ok.diagnostics.is_empty(), "{:?}", diag_lines(&ok));
    let old = parse_with(src, "vba6-excel-win32", &ParseOptions::tolerant());
    let lines = diag_lines(&old);
    assert!(
        lines
            .iter()
            .any(|l| l.contains("`PtrSafe` requires VBA 7.0")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("`LongPtr` requires VBA 7.0")),
        "{lines:?}"
    );
}

#[test]
fn declare_on_64bit_needs_ptrsafe() {
    let src = "Declare Function F Lib \"k\" () As Long\n";
    let p = parse_with(src, "vba7.1-excel-win64", &ParseOptions::tolerant());
    assert!(
        diag_lines(&p)
            .iter()
            .any(|l| l.contains("requires `PtrSafe`"))
    );
    for id in ["vba7.1-excel-win32", "vba6-excel-win32"] {
        let p = parse_with(src, id, &ParseOptions::tolerant());
        assert!(p.diagnostics.is_empty(), "{id}: {:?}", diag_lines(&p));
    }
}

#[test]
fn longlong_needs_a_64bit_vba7_target() {
    let src = "Dim x As LongLong\n";
    let p = parse_with(src, "vba7.1-excel-win64", &ParseOptions::tolerant());
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
    for id in ["vba7.1-excel-win32", "vba6-excel-win32"] {
        let p = parse_with(src, id, &ParseOptions::tolerant());
        assert!(
            diag_lines(&p).iter().any(|l| l.contains("`LongLong`")),
            "{id}"
        );
    }
}

#[test]
fn events_and_implements_are_plain_names_in_access95() {
    let p = parse_with(
        "Event Foo()\n",
        "access95-research",
        &ParseOptions::tolerant(),
    );
    assert!(p.has_errors());
    let p = parse_with(
        "Event Foo()\n",
        "vba5-excel-win32",
        &ParseOptions::tolerant(),
    );
    assert!(!p.has_errors());
}

// ---- strict versus tolerant ------------------------------------------------

#[test]
fn strict_mode_turns_tolerated_deviations_into_errors() {
    let src = "Sub T()\nIf a Then\nx = 1\nElse y = 2\nEnd If\n  lbl:\nEnd Sub\n";
    let tol = parse_with(src, "vba7.1-excel-win64", &ParseOptions::tolerant());
    assert!(!tol.has_errors());
    assert_eq!(tol.diagnostics.len(), 2, "{:?}", diag_lines(&tol));
    let strict = parse_with(src, "vba7.1-excel-win64", &ParseOptions::strict());
    assert!(strict.has_errors());
    assert_eq!(strict.diagnostics.len(), 2);
    assert!(strict.clone().into_result().is_err());
    assert!(tol.into_result().is_ok());
}

#[test]
fn strict_mode_reports_non_ascii_identifiers() {
    let src = "Dim caf\u{e9}\n";
    let tol = parse_with(src, "vba7.1-excel-win64", &ParseOptions::tolerant());
    assert!(tol.diagnostics.is_empty());
    let strict = parse_with(src, "vba7.1-excel-win64", &ParseOptions::strict());
    assert!(strict.has_errors());
}
