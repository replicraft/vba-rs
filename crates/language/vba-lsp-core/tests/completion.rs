//! Completion and signature help.
#![allow(missing_docs)]

mod common;

use common::*;
use vba_lsp_core::*;

fn complete(src: &str) -> Vec<String> {
    let (text, p) = cursor(src);
    let ws = open(&text);
    ws.view(URI)
        .unwrap()
        .completions(p)
        .items
        .into_iter()
        .map(|i| i.label)
        .collect()
}

fn has(items: &[String], l: &str) -> bool {
    items.iter().any(|i| i == l)
}

#[test]
fn statement_start_in_procedure() {
    let items = complete(&MODULE.replace("    Debug.Print s", "    |\n    Debug.Print s"));
    for l in [
        "Dim",
        "If",
        "Select",
        "For",
        "Set",
        "Exit",
        "s",
        "Add",
        "Run",
        "counter",
        "MAX_ITEMS",
        "Left",
        "Mid",
        "Debug",
    ] {
        assert!(has(&items, l), "{l} missing");
    }
    for l in ["Option", "Case", "Next", "Loop", "Sub", "Else"] {
        assert!(!has(&items, l), "{l} unexpected");
    }
    // locals of another procedure are not visible
    assert!(!has(&items, "total"));
}

#[test]
fn block_keywords_follow_the_enclosing_construct() {
    let in_if = complete("Sub A()\n    If x Then\n        |\n    End If\nEnd Sub\n");
    assert!(has(&in_if, "Else") && has(&in_if, "ElseIf"));
    assert!(!has(&in_if, "Next"));
    let in_for = complete("Sub A()\n    For i = 1 To 2\n        |\n    Next\nEnd Sub\n");
    assert!(has(&in_for, "Next"));
    let in_do = complete("Sub A()\n    Do\n        |\n    Loop\nEnd Sub\n");
    assert!(has(&in_do, "Loop"));
    let in_sel = complete(
        "Sub A()\n    Select Case 1\n        Case 1\n            |\n    End Select\nEnd Sub\n",
    );
    assert!(has(&in_sel, "Case"));
}

#[test]
fn module_level_keywords() {
    let items = complete("Option Explicit\n|\nSub A()\nEnd Sub\n");
    for l in [
        "Option", "Public", "Private", "Declare", "Type", "Enum", "Dim", "Sub", "Function", "Const",
    ] {
        assert!(has(&items, l), "{l} missing");
    }
    assert!(!has(&items, "If"));
    assert!(!has(&items, "Left"));
}

#[test]
fn keyword_follow_sets() {
    let table: &[(&str, &[&str])] = &[
        ("Option |", &["Explicit", "Base", "Compare", "Private"]),
        ("Option Compare |", &["Binary", "Text", "Database"]),
        ("Sub A()\n    On |\nEnd Sub\n", &["Error"]),
        ("Sub A()\n    On Error |\nEnd Sub\n", &["GoTo", "Resume"]),
        (
            "Public |",
            &[
                "Sub",
                "Function",
                "Property",
                "Const",
                "Enum",
                "Type",
                "Declare",
                "Event",
                "Static",
                "WithEvents",
            ],
        ),
        ("Public Property |", &["Get", "Let", "Set"]),
        ("Sub A()\n    Select |\nEnd Sub\n", &["Case"]),
        ("Declare |", &["Sub", "Function", "PtrSafe"]),
        (
            "Sub A()\n    If x Then\n        End |\n    End If\nEnd Sub\n",
            &["If", "Sub"],
        ),
        ("Function F()\n    Exit |\nEnd Function\n", &["Function"]),
        (
            "Sub A()\n    For i = 1 To 2\n        Exit |\n    Next\nEnd Sub\n",
            &["For", "Sub"],
        ),
        ("Type T\n    a As Long\n    End |\n", &["Type"]),
    ];
    for (src, expected) in table {
        let items = complete(src);
        assert_eq!(
            items.iter().map(String::as_str).collect::<Vec<_>>(),
            {
                let mut e = expected.to_vec();
                e.sort_by_key(|s| s.to_ascii_lowercase());
                e
            },
            "{src}"
        );
    }
}

#[test]
fn declare_offers_ptrsafe_only_where_supported() {
    let (text, p) = cursor("Declare |");
    let mut ws = open(&text);
    let mut cfg = ws.config().clone();
    cfg.set_profile("vba6-excel-win32").unwrap();
    ws.set_config(cfg);
    let items = ws.view(URI).unwrap().completions(p).items;
    assert!(!items.iter().any(|i| i.label == "PtrSafe"));
}

#[test]
fn type_position() {
    let items =
        complete("Type Point\n    X As Long\nEnd Type\nEnum E\n    a\nEnd Enum\nDim v As |\n");
    for l in [
        "Long", "String", "Variant", "Point", "E", "LongPtr", "LongLong",
    ] {
        assert!(has(&items, l), "{l} missing");
    }
    assert!(!has(&items, "Dim"));
    let items = complete("Dim v As Lo|\n");
    assert_eq!(items, ["Long", "LongLong", "LongPtr"]);
    // 32-bit VBA 6 has neither LongPtr nor LongLong
    let (text, p) = cursor("Dim v As Lo|\n");
    let mut ws = open(&text);
    let mut cfg = ws.config().clone();
    cfg.set_profile("vba6-excel-win32").unwrap();
    ws.set_config(cfg);
    let labels: Vec<String> = ws
        .view(URI)
        .unwrap()
        .completions(p)
        .items
        .into_iter()
        .map(|i| i.label)
        .collect();
    assert_eq!(labels, ["Long"]);
}

#[test]
fn member_completion_for_enums_and_library_qualifiers() {
    let src = "Enum Color\n    Red\n    Green\nEnd Enum\nSub A()\n    x = Color.|\nEnd Sub\n";
    assert_eq!(complete(src), ["Green", "Red"]);
    let items = complete("Sub A()\n    x = VBA.|\nEnd Sub\n");
    assert!(has(&items, "Left") && has(&items, "Strings") && has(&items, "DateAdd"));
    let items = complete("Sub A()\n    x = Strings.|\nEnd Sub\n");
    assert!(has(&items, "Left") && !has(&items, "DateAdd"));
    // unknown receivers offer nothing (no type information)
    assert!(complete("Sub A()\n    x = obj.|\nEnd Sub\n").is_empty());
    assert!(complete("Sub A()\n    With obj\n        .|\n    End With\nEnd Sub\n").is_empty());
}

#[test]
fn expression_position_and_prefix_filter() {
    let items = complete("Sub A()\n    Dim count As Long\n    x = |\nEnd Sub\n");
    for l in ["count", "Left", "Not", "True", "New", "Nothing", "A"] {
        assert!(has(&items, l), "{l} missing");
    }
    assert!(!has(&items, "And"));
    let items = complete("Sub A()\n    Dim count As Long\n    x = C|\nEnd Sub\n");
    assert!(has(&items, "count") && has(&items, "Cos") && has(&items, "CStr"));
    assert!(!has(&items, "Left"));
    let items = complete("Sub A()\n    x = Mid|\nEnd Sub\n");
    assert!(has(&items, "Mid"));
    // after an operand, only operators
    let items = complete("Sub A()\n    x = y |\nEnd Sub\n");
    assert!(has(&items, "And") && has(&items, "Mod"));
    assert!(!has(&items, "Left") && !has(&items, "y"));
    // `As` follows a declarator
    assert!(has(&complete("Sub A()\n    Dim v |\nEnd Sub\n"), "As"));
}

#[test]
fn labels_after_goto() {
    let items = complete(
        "Sub A()\n    GoTo |\nfirst:\n    x = 1\nsecond:\nEnd Sub\nSub B()\nother:\nEnd Sub\n",
    );
    assert_eq!(items, ["first", "second"]);
}

#[test]
fn nothing_inside_comments_and_strings() {
    assert!(complete("Sub A()\n    x = 1 ' le|\nEnd Sub\n").is_empty());
    assert!(complete("Sub A()\n    x = \"le|\"\nEnd Sub\n").is_empty());
    assert!(complete("Sub A()\n    x = 1 '|\nEnd Sub\n").is_empty());
}

#[test]
fn completion_replace_range_covers_the_word() {
    let (text, p) = cursor("Sub A()\n    x = Le|ft\nEnd Sub\n");
    let ws = open(&text);
    let c = ws.view(URI).unwrap().completions(p);
    assert_eq!(slice(&text, c.replace), "Left");
    let (text, p) = cursor("Sub A()\n    x = |\nEnd Sub\n");
    let ws = open(&text);
    let c = ws.view(URI).unwrap().completions(p);
    assert_eq!(c.replace, Range::new(p, p));
}

#[test]
fn completion_details_carry_signatures() {
    let (text, p) = cursor(
        "Function Twice(ByVal n As Long) As Long\nEnd Function\nSub A()\n    x = Tw|\nEnd Sub\n",
    );
    let ws = open(&text);
    let c = ws.view(URI).unwrap().completions(p);
    assert_eq!(c.items.len(), 1);
    assert_eq!(c.items[0].kind, CompletionKind::Procedure);
    assert_eq!(
        c.items[0].detail.as_deref(),
        Some("Function Twice(ByVal n As Long) As Long")
    );
    let (text, p) = cursor("Sub A()\n    x = Lef|\nEnd Sub\n");
    let ws = open(&text);
    let c = ws.view(URI).unwrap().completions(p);
    assert_eq!(c.items[0].kind, CompletionKind::Function);
    assert!(c.items[0].detail.as_deref().unwrap().starts_with("Left("));
}

fn sig(src: &str) -> Option<(SignatureHelp, String)> {
    let (text, p) = cursor(src);
    let ws = open(&text);
    let h = ws.view(URI).unwrap().signature_help(p)?;
    let label = h.signatures[0].label.clone();
    Some((h, label))
}

#[test]
fn signature_help_table() {
    // (source, label prefix, active parameter)
    let cases: &[(&str, &str, Option<u32>)] = &[
        ("Sub A()\n    s = Left(|\nEnd Sub\n", "Left(", Some(0)),
        (
            "Sub A()\n    s = Left(\"abc\", |\nEnd Sub\n",
            "Left(",
            Some(1),
        ),
        (
            "Sub A()\n    s = Left(\"abc\", 1|)\nEnd Sub\n",
            "Left(",
            Some(1),
        ),
        (
            "Sub A()\n    s = Left(Mid(s, 1, |\nEnd Sub\n",
            "Mid(",
            Some(2),
        ),
        (
            "Sub A()\n    s = Left(Mid(s, 1), |\nEnd Sub\n",
            "Left(",
            Some(1),
        ),
        ("Sub A()\n    s = Left(Len(x|)\nEnd Sub\n", "Len(", Some(0)),
        (
            "Sub A()\n    s = Left(String:=\"a\", Length:=|\nEnd Sub\n",
            "Left(",
            Some(1),
        ),
        (
            "Sub A()\n    s = VBA.Strings.Mid(|\nEnd Sub\n",
            "Mid(",
            Some(0),
        ),
        (
            "Sub A()\n    s = Left$(\"abc\", |\nEnd Sub\n",
            "Left(",
            Some(1),
        ),
        (
            "Sub A()\n    s = Left(\"abc\", 1, |\nEnd Sub\n",
            "Left(",
            None,
        ),
        (
            "Function Add(ByVal a As Long, Optional b As Long = 2) As Long\nEnd Function\nSub R()\n    x = Add(1, |\nEnd Sub\n",
            "Function Add(ByVal a As Long, Optional b As Long = 2) As Long",
            Some(1),
        ),
        (
            "Function Add(ByVal a As Long, Optional b As Long = 2) As Long\nEnd Function\nSub R()\n    x = Add(b:=|\nEnd Sub\n",
            "Function Add(",
            Some(1),
        ),
        (
            "Sub Go(x, y, z)\nEnd Sub\nSub R()\n    Go 1, |\nEnd Sub\n",
            "Sub Go(x, y, z)",
            Some(1),
        ),
        (
            "Sub Go(x, y, z)\nEnd Sub\nSub R()\n    Go 1, (2 + 3), |\nEnd Sub\n",
            "Sub Go(x, y, z)",
            Some(2),
        ),
        (
            "Sub Go(ParamArray v())\nEnd Sub\nSub R()\n    Go 1, 2, 3, |\nEnd Sub\n",
            "Sub Go(ParamArray v())",
            Some(0),
        ),
        (
            "Sub A()\n    MsgBox2 = Right(|\nEnd Sub\n",
            "Right(",
            Some(0),
        ),
    ];
    for (src, label, active) in cases {
        let (h, l) = sig(src).unwrap_or_else(|| panic!("no signature for {src}"));
        assert!(l.starts_with(label), "{src}: {l}");
        assert_eq!(h.active_parameter, *active, "{src}");
    }
}

#[test]
fn signature_parameter_ranges_index_the_label() {
    let (h, l) = sig("Sub A()\n    s = Left(|\nEnd Sub\n").unwrap();
    let p = &h.signatures[0].parameters;
    assert_eq!(p.len(), 2);
    let cut = |i: usize| -> String {
        l.encode_utf16()
            .skip(p[i].start as usize)
            .take((p[i].end - p[i].start) as usize)
            .map(|c| char::from_u32(c as u32).unwrap())
            .collect()
    };
    assert!(cut(0).starts_with("String As"), "{}", cut(0));
    assert!(cut(1).contains("Length"), "{}", cut(1));
}

#[test]
fn signature_help_absent_when_not_in_a_call() {
    for src in [
        "Sub A()\n    x = (1 + |2)\nEnd Sub\n",
        "Sub A()\n    x = 1 + |2\nEnd Sub\n",
        "Sub A()\n    x = unknown(|\nEnd Sub\n",
        "Sub A()\n    x = obj.Left(|\nEnd Sub\n",
        "Sub A()\n    Dim arr(1)\n    x = arr(|\nEnd Sub\n",
        "Sub A()\n    s = Left(\"a,|b\")\nEnd Sub\n",
        "Sub A()\n    s = Left(\"a\") ' (|\nEnd Sub\n",
        "Sub A()\n    x = |\nEnd Sub\n",
        "Sub A()\n    If x Then|\nEnd Sub\n",
        "Sub A()\n    x = 1, |\nEnd Sub\n",
    ] {
        assert!(sig(src).is_none(), "{src}");
    }
}

#[test]
fn property_signatures_list_all_declarations() {
    let (h, _) = sig("Property Get P() As Long\nEnd Property\nProperty Let P(ByVal v As Long)\nEnd Property\nSub R()\n    P(|\nEnd Sub\n").unwrap();
    assert_eq!(h.signatures.len(), 2);
}
