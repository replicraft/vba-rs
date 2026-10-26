//! Cross-module features backed by `vba-sema`: diagnostics, definition,
//! references, rename, hover, completion and workspace symbols.
#![allow(missing_docs)]

mod common;

use common::*;
use vba_lsp_core::*;

pub const UTILS: &str = "\
Attribute VB_Name = \"Utils\"
Option Explicit
Public Const LIMIT As Long = 100
Public Enum Level
    Low = 1
    High
End Enum
Public Type Pt
    X As Long
    Y As Long
End Type
Public Function Twice(ByVal n As Long) As Long
    Twice = n * 2
End Function
";

pub const ACCOUNT: &str = "\
VERSION 1.0 CLASS
BEGIN
  MultiUse = -1
END
Attribute VB_Name = \"Account\"
Option Explicit
Public Balance As Currency
Private mOwner As String
Public Sub Deposit(ByVal amount As Currency)
    Balance = Balance + amount
    Me.Balance = Me.Balance
End Sub
Public Property Get Owner() As String
    Owner = mOwner
End Property
";

pub const MAIN: &str = "\
Option Explicit
Sub Run()
    Dim a As Account, p As Pt, lv As Level
    Set a = New Account
    a.Deposit Twice(LIMIT)
    p.X = 1
    Debug.Print a.Balance
    With a
        .Deposit 5
    End With
    lv = Level.High
End Sub
";

const U: &str = "file:///proj/Utils.bas";
const A: &str = "file:///proj/Account.cls";
const M: &str = "file:///proj/Main.bas";

fn project() -> Workspace {
    let mut ws = Workspace::new();
    ws.open(U, UTILS, 1);
    ws.open(A, ACCOUNT, 1);
    ws.open(M, MAIN, 1);
    ws
}

/// Position of the `n`-th (0-based) occurrence of `needle` in `text`, plus
/// `delta` columns.
fn at(text: &str, needle: &str, n: usize, delta: u32) -> Position {
    let mut from = 0;
    let mut idx = 0;
    for _ in 0..=n {
        idx = from + text[from..].find(needle).expect("needle");
        from = idx + needle.len();
    }
    let before = &text[..idx];
    let line = before.matches('\n').count() as u32;
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    Position::new(line, before[start..].encode_utf16().count() as u32 + delta)
}

fn text_of(ws: &Workspace, loc: &Location) -> String {
    let d = ws.document(&loc.uri).unwrap();
    let s = d.text_map().span(d.bytes(), loc.range);
    String::from_utf8_lossy(&d.bytes()[s.start as usize..s.end as usize]).into_owned()
}

fn apply(ws: &Workspace, edits: &[FileEdit]) -> Vec<(String, String)> {
    edits
        .iter()
        .map(|fe| {
            let text = ws.document(&fe.uri).unwrap().text();
            (fe.uri.clone(), apply_edits(&text, &fe.edits))
        })
        .collect()
}

#[test]
fn consistent_project_has_no_semantic_diagnostics() {
    let ws = project();
    for u in [U, A, M] {
        let d = ws.view(u).unwrap().diagnostics();
        assert!(d.is_empty(), "{u}: {d:?}");
    }
}

#[test]
fn semantic_diagnostics_land_in_the_right_document() {
    let mut ws = project();
    let bad = MAIN.replace("    p.X = 1\n", "    p.X = 1\n    nope = a.Missing\n");
    ws.open(M, &bad, 2);
    let main: Vec<_> = ws.view(M).unwrap().diagnostics();
    let codes: Vec<&str> = main.iter().map(|d| d.code.as_str()).collect();
    assert!(codes.contains(&"SEM002"), "{main:?}");
    assert!(codes.contains(&"SEM019"), "{main:?}");
    let sem002 = main.iter().find(|d| d.code == "SEM002").unwrap();
    assert_eq!(slice(&bad, sem002.range), "nope");
    assert_eq!(sem002.range.start.line, 6);
    for u in [U, A] {
        assert!(ws.view(u).unwrap().diagnostics().is_empty());
    }
}

#[test]
fn a_change_in_one_module_updates_diagnostics_of_another() {
    let mut ws = project();
    assert!(ws.view(M).unwrap().diagnostics().is_empty());
    let utils = UTILS.replace("Twice", "Thrice");
    ws.change(
        U,
        2,
        &[TextChange {
            range: None,
            text: utils,
        }],
    )
    .unwrap();
    let d = ws.view(M).unwrap().diagnostics();
    assert!(d.iter().any(|d| d.code == "SEM003"), "{d:?}");
}

#[test]
fn analysis_is_cached_until_something_changes() {
    let mut ws = project();
    assert_eq!(ws.semantic_runs(), 0);
    for u in [U, A, M] {
        let _ = ws.view(u).unwrap().diagnostics();
        let _ = ws.definition(M, Position::new(4, 6));
        let _ = ws.workspace_symbols("", 10);
    }
    assert_eq!(ws.semantic_runs(), 1);
    ws.change(
        M,
        2,
        &[TextChange {
            range: Some(Range::new(Position::new(0, 0), Position::new(0, 0))),
            text: "' c\n".into(),
        }],
    )
    .unwrap();
    let _ = ws.view(M).unwrap().diagnostics();
    let _ = ws.view(U).unwrap().diagnostics();
    assert_eq!(ws.semantic_runs(), 2);
    let mut cfg = ws.config().clone();
    cfg.project_name = "Other".into();
    ws.set_config(cfg);
    let _ = ws.view(M).unwrap().diagnostics();
    assert_eq!(ws.semantic_runs(), 3);
}

#[test]
fn semantic_features_can_be_disabled() {
    let mut ws = project();
    let mut cfg = ws.config().clone();
    cfg.semantic = false;
    ws.set_config(cfg);
    ws.open(M, &MAIN.replace("a.Deposit", "a.Nope"), 2);
    assert!(ws.view(M).unwrap().diagnostics().is_empty());
    assert_eq!(ws.semantic_runs(), 0);
    assert!(ws.workspace_symbols("", 10).is_empty());
}

#[test]
fn diagnostic_columns_are_utf16_and_follow_the_encoding() {
    let mut ws = Workspace::new();
    let src =
        "Option Explicit\nSub A()\n    Dim x As String\n    x = \"\u{1F600}\" & nope\nEnd Sub\n";
    ws.open("file:///u.bas", src, 1);
    let d = ws.view("file:///u.bas").unwrap().diagnostics();
    let nope = d.iter().find(|d| d.code == "SEM002").unwrap();
    // 4 spaces + `x = ` + quote + surrogate pair (2) + quote + ` & `
    assert_eq!(nope.range.start, Position::new(3, 4 + 4 + 1 + 2 + 1 + 3));
    assert_eq!(slice(src, nope.range), "nope");

    // A double-byte code page: the Japanese letter is one column.
    let mut ws = Workspace::new();
    let mut cfg = ws.config().clone();
    cfg.encoding = parse_encoding("932").unwrap();
    ws.set_config(cfg);
    let src =
        "Option Explicit\nSub A()\n    Dim x As String\n    x = \"\u{3042}\" & nope\nEnd Sub\n";
    ws.open("file:///j.bas", src, 1);
    let d = ws.view("file:///j.bas").unwrap().diagnostics();
    let nope = d.iter().find(|d| d.code == "SEM002").unwrap();
    assert_eq!(nope.range.start, Position::new(3, 4 + 4 + 1 + 1 + 1 + 3));
    assert_eq!(nope.range.end.character - nope.range.start.character, 4);
}

#[test]
fn module_kind_is_inferred_and_configurable() {
    // A class used from another module only works when it is a class.
    let mut ws = Workspace::new();
    ws.open("file:///x/Thing.txt", "Public V As Long\n", 1);
    ws.open(
        "file:///x/Use.bas",
        "Sub A()\n    Dim t As New Thing\n    t.V = 1\nEnd Sub\n",
        1,
    );
    let d = ws.view("file:///x/Use.bas").unwrap().diagnostics();
    assert!(d.iter().any(|d| d.code == "SEM004"), "{d:?}");
    let mut cfg = ws.config().clone();
    cfg.module_kinds = vec![("*.txt".into(), ModuleKind::Class)];
    ws.set_config(cfg);
    let d = ws.view("file:///x/Use.bas").unwrap().diagnostics();
    assert!(d.is_empty(), "{d:?}");

    // Export header without an extension hint.
    let mut ws = Workspace::new();
    ws.open("file:///x/Thing", ACCOUNT, 1);
    ws.open(
        "file:///x/Use.bas",
        "Sub A()\n    Dim t As New Account\n    t.Deposit 1\nEnd Sub\n",
        1,
    );
    assert!(
        ws.view("file:///x/Use.bas")
            .unwrap()
            .diagnostics()
            .is_empty()
    );
    // The name comes from `Attribute VB_Name`, not from the file.
    let syms = ws.workspace_symbols("Account", 10);
    assert!(
        syms.iter()
            .any(|s| s.kind == SymbolKind::Class && s.name == "Account")
    );
}

#[test]
fn definition_across_modules() {
    let ws = project();
    let cases: &[(&str, usize, u32, &str, &str)] = &[
        // (text, nth, delta, expected uri, expected text)
        ("Twice(LIMIT)", 0, 2, U, "Twice"),
        ("LIMIT)", 0, 2, U, "LIMIT"),
        ("Level.High", 0, 8, U, "High"),
        ("Level.High", 0, 2, U, "Level"),
        ("As Account", 0, 5, A, ""),
        ("a.Deposit Twice", 0, 3, A, "Deposit"),
        (".Deposit 5", 0, 3, A, "Deposit"),
        ("a.Balance", 0, 3, A, "Balance"),
        ("As Pt", 0, 4, U, "Pt"),
        ("p.X", 0, 2, U, "X"),
    ];
    for (needle, n, delta, uri, text) in cases {
        let p = at(MAIN, needle, *n, *delta);
        let locs = ws.definition(M, p).unwrap();
        assert_eq!(locs.len(), 1, "{needle}: {locs:?}");
        assert_eq!(locs[0].uri, *uri, "{needle}");
        assert_eq!(text_of(&ws, &locs[0]), *text, "{needle}");
    }
    // Inside the class: Me.Balance resolves to the field.
    let p = at(ACCOUNT, "Me.Balance", 0, 4);
    let locs = ws.definition(A, p).unwrap();
    assert_eq!(locs.len(), 1);
    assert_eq!(text_of(&ws, &locs[0]), "Balance");
    assert_eq!(locs[0].range.start.line, 6);
}

#[test]
fn definition_falls_back_to_the_syntactic_walk() {
    let mut ws = Workspace::new();
    ws.open(
        "file:///l.bas",
        "Sub A()\n    GoTo done\ndone:\n    Dim v As Variant\n    v.Foo = 1\nEnd Sub\n",
        1,
    );
    // Labels are not in the project model: the syntactic walk answers.
    let locs = ws
        .definition("file:///l.bas", Position::new(1, 10))
        .unwrap();
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].range.start.line, 2);
    // A late-bound member has no definition.
    let locs = ws.definition("file:///l.bas", Position::new(4, 7)).unwrap();
    assert!(locs.is_empty());
}

#[test]
fn references_across_modules() {
    let ws = project();
    let p = at(UTILS, "Twice(ByVal", 0, 1);
    let refs = ws.references(U, p, true).unwrap();
    let got: Vec<(String, u32)> = refs
        .iter()
        .map(|l| {
            (
                l.uri.rsplit('/').next().unwrap().to_string(),
                l.range.start.line,
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            ("Main.bas".to_string(), 4),
            ("Utils.bas".to_string(), 11),
            ("Utils.bas".to_string(), 12)
        ]
    );
    let no_decl = ws.references(U, p, false).unwrap();
    assert_eq!(no_decl.len(), 2);

    // Deposit: the call, the With member, and the declaration.
    let p = at(MAIN, "a.Deposit", 0, 3);
    let refs = ws.references(M, p, true).unwrap();
    assert_eq!(refs.len(), 3, "{refs:?}");
    assert!(refs.iter().all(|l| text_of(&ws, l) == "Deposit"));

    // Balance inside the class and from outside.
    let p = at(ACCOUNT, "Balance As", 0, 1);
    let refs = ws.references(A, p, true).unwrap();
    assert_eq!(refs.len(), 6, "{refs:?}");
}

#[test]
fn references_distinguish_same_named_locals_and_members() {
    let mut ws = Workspace::new();
    ws.open(
        "file:///a.bas",
        "Public Counter As Long\nSub A()\n    Dim Counter As Long\n    Counter = 1\nEnd Sub\nSub B()\n    Counter = 2\nEnd Sub\n",
        1,
    );
    let refs = ws
        .references("file:///a.bas", Position::new(0, 9), true)
        .unwrap();
    assert_eq!(
        refs.iter().map(|l| l.range.start.line).collect::<Vec<_>>(),
        [0, 6]
    );
    let refs = ws
        .references("file:///a.bas", Position::new(3, 6), true)
        .unwrap();
    assert_eq!(
        refs.iter().map(|l| l.range.start.line).collect::<Vec<_>>(),
        [2, 3]
    );
}

#[test]
fn rename_across_modules() {
    let ws = project();
    let p = at(UTILS, "Twice(ByVal", 0, 1);
    let edits = ws.rename(U, p, "Doubled").unwrap();
    let out = apply(&ws, &edits);
    assert_eq!(out.len(), 2);
    for (uri, text) in &out {
        if uri == M {
            assert_eq!(text, &MAIN.replace("Twice", "Doubled"));
        } else {
            assert_eq!(text, &UTILS.replace("Twice", "Doubled"));
        }
    }
    // The renamed workspace is still clean.
    let mut ws2 = Workspace::new();
    ws2.open(U, &out.iter().find(|o| o.0 == U).unwrap().1, 1);
    ws2.open(A, ACCOUNT, 1);
    ws2.open(M, &out.iter().find(|o| o.0 == M).unwrap().1, 1);
    assert!(ws2.view(M).unwrap().diagnostics().is_empty());
}

#[test]
fn rename_class_members_updates_every_use() {
    let ws = project();
    let p = at(ACCOUNT, "Balance As", 0, 1);
    let edits = ws.rename(A, p, "Funds").unwrap();
    let out = apply(&ws, &edits);
    for (uri, text) in &out {
        if uri == M {
            assert_eq!(text, &MAIN.replace("a.Balance", "a.Funds"));
        } else {
            assert_eq!(text, &ACCOUNT.replace("Balance", "Funds"));
        }
    }
    // Enum members and UDT fields too.
    let p = at(MAIN, "Level.High", 0, 8);
    let edits = ws.rename(M, p, "Top").unwrap();
    let out = apply(&ws, &edits);
    assert!(out.iter().any(|(_, t)| t.contains("lv = Level.Top")));
    assert!(out.iter().any(|(_, t)| t.contains("    Top\nEnd Enum")));
    let p = at(MAIN, "p.X", 0, 2);
    let edits = ws.rename(M, p, "Horizontal").unwrap();
    let out = apply(&ws, &edits);
    assert!(out.iter().any(|(_, t)| t.contains("p.Horizontal = 1")));
    assert!(
        out.iter()
            .any(|(_, t)| t.contains("    Horizontal As Long"))
    );
}

#[test]
fn local_rename_stays_in_its_procedure() {
    let mut ws = Workspace::new();
    ws.open(
        "file:///a.bas",
        "Sub A()\n    Dim n As Long\n    n = n + 1\nEnd Sub\nSub B()\n    Dim n As Long\n    n = 2\nEnd Sub\n",
        1,
    );
    let edits = ws
        .rename("file:///a.bas", Position::new(1, 8), "count")
        .unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].edits.len(), 3);
}

#[test]
fn renaming_an_interface_member_is_refused_when_implementers_would_break() {
    let mut ws = Workspace::new();
    ws.open(
        "file:///i/IShape.cls",
        "Option Explicit\nPublic Sub Draw()\nEnd Sub\n",
        1,
    );
    ws.open(
        "file:///i/Circle.cls",
        "Option Explicit\nImplements IShape\nPrivate Sub IShape_Draw()\nEnd Sub\n",
        1,
    );
    assert!(
        ws.view("file:///i/Circle.cls")
            .unwrap()
            .diagnostics()
            .is_empty()
    );
    let e = ws
        .rename("file:///i/IShape.cls", Position::new(1, 13), "Render")
        .unwrap_err();
    assert!(
        matches!(&e, Error::Invalid(m) if m.contains("SEM011")),
        "{e:?}"
    );
}

#[test]
fn project_qualified_names_resolve() {
    let mut ws = project();
    let main = MAIN.replace(
        "a.Deposit Twice(LIMIT)",
        "a.Deposit VBAProject.Utils.Twice(LIMIT)",
    );
    ws.open(M, &main, 2);
    assert!(ws.view(M).unwrap().diagnostics().is_empty());
    let p = at(&main, "Utils.Twice", 0, 7);
    let locs = ws.definition(M, p).unwrap();
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].uri, U);
    assert_eq!(text_of(&ws, &locs[0]), "Twice");
}

#[test]
fn static_locals_rename_like_locals_and_cannot_shadow_a_module_qualifier() {
    let mut ws = project();
    let main = MAIN.replace(
        "    lv = Level.High\n",
        "    lv = Level.High\n    Static calls As Long\n    calls = calls + Twice(1)\n",
    );
    ws.open(M, &main, 2);
    let p = at(&main, "Static calls", 0, 8);
    let files = ws.rename(M, p, "total").unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].edits.len(), 3);
    // `Utils` is qualified elsewhere in the procedure: a local called
    // `Utils` would capture it.
    let main = main.replace("Twice(1)", "Utils.Twice(1)");
    ws.open(M, &main, 3);
    let p = at(&main, "Static calls", 0, 8);
    assert!(matches!(ws.rename(M, p, "Utils"), Err(Error::Invalid(_))));
}

#[test]
fn late_bound_members_are_never_renamed() {
    let mut ws = project();
    let main = MAIN.replace(
        "    lv = Level.High\n",
        "    lv = Level.High\n    Dim v As Variant\n    Set v = a\n    v.Deposit 1\n    v.Unknown = 2\n",
    );
    ws.open(M, &main, 2);
    // Cursor on the late-bound member itself.
    let p = at(&main, "v.Unknown", 0, 3);
    let e = ws.rename(M, p, "Other").unwrap_err();
    assert!(matches!(e, Error::Unsupported(_)), "{e:?}");
    assert!(ws.prepare_rename(M, p).is_err());
    assert!(ws.definition(M, p).unwrap().is_empty());
    // The declaration of a class member that is also used late bound.
    let p = at(ACCOUNT, "Deposit(", 0, 1);
    let e = ws.rename(A, p, "Place").unwrap_err();
    assert!(
        matches!(&e, Error::Unsupported(m) if m.contains("late bound")),
        "{e:?}"
    );
    // A member without late uses is still fine.
    let p = at(ACCOUNT, "Owner()", 0, 1);
    assert!(ws.rename(A, p, "Holder").is_ok());
}

#[test]
fn rename_refuses_clashes_and_shadowing() {
    let ws = project();
    let twice = at(UTILS, "Twice(ByVal", 0, 1);
    for (name, why) in [
        ("LIMIT", "same module"),
        ("limit", "same module, case-insensitive"),
        ("Account", "module name"),
        ("Main", "module name"),
        ("1bad", "not an identifier"),
        ("End", "keyword"),
        ("a b", "not an identifier"),
    ] {
        let e = ws.rename(U, twice, name);
        assert!(matches!(e, Err(Error::Invalid(_))), "{name} ({why}): {e:?}");
    }
    // A local `a` in Run would capture a renamed module-level name.
    let e = ws.rename(U, at(UTILS, "LIMIT", 0, 1), "a").unwrap_err();
    assert!(matches!(e, Error::Invalid(_)), "{e:?}");
    // Same-case rename is a no-op rename, allowed.
    assert!(ws.rename(U, twice, "TWICE").is_ok());
}

#[test]
fn rename_refuses_a_name_that_would_be_ambiguous_across_modules() {
    let mut ws = project();
    ws.open(
        "file:///proj/Other.bas",
        "Public Function Helper() As Long\nEnd Function\n",
        1,
    );
    let e = ws
        .rename(U, at(UTILS, "Twice(ByVal", 0, 1), "Helper")
        .unwrap_err();
    assert!(matches!(e, Error::Invalid(_)), "{e:?}");
}

#[test]
fn rename_parameter_is_refused_when_named_arguments_exist() {
    let mut ws = Workspace::new();
    ws.open(
        "file:///a.bas",
        "Sub F(ByVal count As Long)\nEnd Sub\nSub G()\n    F count:=1\nEnd Sub\n",
        1,
    );
    let e = ws
        .rename("file:///a.bas", Position::new(0, 14), "n")
        .unwrap_err();
    assert!(matches!(e, Error::Unsupported(_)), "{e:?}");
    ws.open(
        "file:///a.bas",
        "Sub F(ByVal count As Long)\n    Debug.Print count\nEnd Sub\n",
        2,
    );
    let edits = ws
        .rename("file:///a.bas", Position::new(0, 14), "n")
        .unwrap();
    assert_eq!(edits[0].edits.len(), 2);
}

#[test]
fn rename_in_a_label_falls_back_to_the_syntactic_walk() {
    let mut ws = Workspace::new();
    ws.open(
        "file:///a.bas",
        "Sub A()\n    GoTo done\ndone:\nEnd Sub\n",
        1,
    );
    let edits = ws
        .rename("file:///a.bas", Position::new(2, 1), "finish")
        .unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].edits.len(), 2);
}

#[test]
fn hover_is_type_aware() {
    let ws = project();
    let h = |uri: &str, text: &str, needle: &str, n: usize, d: u32| {
        ws.view(uri)
            .unwrap()
            .hover(at(text, needle, n, d))
            .unwrap_or_else(|| panic!("no hover for {needle}"))
            .contents
    };
    let t = h(M, MAIN, "Twice(LIMIT)", 0, 2);
    assert!(
        t.contains("Public Function Twice(ByVal n As Long) As Long"),
        "{t}"
    );
    assert!(t.contains("Module `Utils`"), "{t}");
    let t = h(M, MAIN, "LIMIT)", 0, 2);
    assert!(t.contains("Public Const LIMIT As Long = 100"), "{t}");
    let t = h(M, MAIN, "a.Balance", 0, 3);
    assert!(t.contains("Public Balance As Currency"), "{t}");
    let t = h(M, MAIN, "a = New", 0, 0);
    assert!(t.contains("(local variable) a As Account"), "{t}");
    let t = h(M, MAIN, "p.X", 0, 2);
    assert!(t.contains("(field) X As Long"), "{t}");
    let t = h(M, MAIN, "Level.High", 0, 8);
    assert!(t.contains("(enum member) High = 2"), "{t}");
    let t = h(M, MAIN, ".Deposit 5", 0, 2);
    assert!(
        t.contains("Public Sub Deposit(ByVal amount As Currency)"),
        "{t}"
    );
    let t = h(A, ACCOUNT, "Me.Balance", 0, 0);
    assert!(t.contains("Me As Account"), "{t}");
    let t = h(A, ACCOUNT, "Owner = mOwner", 0, 1);
    assert!(t.contains("Property Get Owner() As String"), "{t}");
}

#[test]
fn hover_on_a_late_bound_member_says_so() {
    let mut ws = Workspace::new();
    ws.open(
        "file:///a.bas",
        "Sub A()\n    Dim v As Variant\n    v.Foo = 1\nEnd Sub\n",
        1,
    );
    let h = ws
        .view("file:///a.bas")
        .unwrap()
        .hover(Position::new(2, 6))
        .unwrap();
    assert!(h.contents.contains("late bound"), "{}", h.contents);
}

fn with_marker(ws: &mut Workspace, uri: &str, base: &str, anchor: &str, insert: &str) -> String {
    let text = base.replacen(anchor, &format!("{anchor}{insert}"), 1);
    ws.open(uri, &text, 9);
    text
}

#[test]
fn completion_after_dot_uses_project_types() {
    let mut ws = project();
    let cases: &[(&str, &str, &[&str], &[&str])] = &[
        // (statement inserted after `Run()` line, marker, must have, must not have)
        (
            "    a.\n",
            "a.",
            &["Balance", "Deposit", "Owner"],
            &["mOwner", "LIMIT"],
        ),
        ("    p.\n", "p.", &["X", "Y"], &["Twice"]),
        ("    x = Level.\n", "Level.", &["High", "Low"], &["X"]),
        (
            "    x = Utils.\n",
            "Utils.",
            &["Twice", "LIMIT", "Level", "Pt"],
            &["Run"],
        ),
        (
            "    With a\n        .\n    End With\n",
            "        .",
            &["Balance", "Deposit"],
            &[],
        ),
        ("    x = a.Dep\n", "a.Dep", &["Deposit"], &["Balance"]),
    ];
    for (insert, marker, have, not) in cases {
        let text = with_marker(&mut ws, M, MAIN, "Sub Run()\n", insert);
        let off = text.find(marker).unwrap() + marker.len();
        let before = &text[..off];
        let line = before.matches('\n').count() as u32;
        let start = before.rfind('\n').map_or(0, |i| i + 1);
        let pos = Position::new(line, before[start..].encode_utf16().count() as u32);
        let got: Vec<String> = ws
            .view(M)
            .unwrap()
            .completions(pos)
            .items
            .into_iter()
            .map(|i| i.label)
            .collect();
        for h in *have {
            assert!(
                got.iter().any(|g| g == h),
                "{marker}: {h} missing in {got:?}"
            );
        }
        for n in *not {
            assert!(
                !got.iter().any(|g| g == n),
                "{marker}: {n} present in {got:?}"
            );
        }
    }
}

#[test]
fn completion_of_me_members_and_other_modules() {
    let mut ws = project();
    let text = ACCOUNT.replacen("    Me.Balance = Me.Balance\n", "    Me.\n", 1);
    ws.open(A, &text, 2);
    let off = text.find("Me.").unwrap() + 3;
    let before = &text[..off];
    let pos = Position::new(
        before.matches('\n').count() as u32,
        (off - before.rfind('\n').map_or(0, |i| i + 1)) as u32,
    );
    let got: Vec<String> = ws
        .view(A)
        .unwrap()
        .completions(pos)
        .items
        .into_iter()
        .map(|i| i.label)
        .collect();
    // Inside the class the private field is accessible too.
    for want in ["Balance", "Deposit", "Owner", "mOwner"] {
        assert!(got.iter().any(|g| g == want), "{want} in {got:?}");
    }

    // Unqualified completion in another module offers the public names.
    let got = labels_at_statement(&mut ws, "    ");
    for want in ["Twice", "LIMIT", "Level", "High", "Utils", "Account"] {
        assert!(got.iter().any(|g| g == want), "{want} in {got:?}");
    }
    // Type position offers class modules and public types.
    let got = labels_at_statement(&mut ws, "    Dim q As ");
    for want in ["Account", "Pt", "Level", "Long"] {
        assert!(got.iter().any(|g| g == want), "{want} in {got:?}");
    }
    assert!(!got.iter().any(|g| g == "Twice"));
}

fn labels_at_statement(ws: &mut Workspace, line: &str) -> Vec<String> {
    let text = MAIN.replacen("Sub Run()\n", &format!("Sub Run()\n{line}\n"), 1);
    ws.open(M, &text, 10);
    ws.view(M)
        .unwrap()
        .completions(Position::new(2, line.len() as u32))
        .items
        .into_iter()
        .map(|i| i.label)
        .collect()
}

#[test]
fn completion_on_variant_receivers_offers_nothing() {
    let mut ws = Workspace::new();
    let src = "Sub A()\n    Dim v As Variant\n    v.\nEnd Sub\n";
    ws.open("file:///a.bas", src, 1);
    let got = ws
        .view("file:///a.bas")
        .unwrap()
        .completions(Position::new(2, 6))
        .items;
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn workspace_symbols_cover_all_modules() {
    let ws = project();
    let all = ws.workspace_symbols("", 1000);
    let names: Vec<(&str, &str)> = all
        .iter()
        .map(|s| (s.name.as_str(), s.container.as_str()))
        .collect();
    for want in [
        ("Twice", "Utils"),
        ("LIMIT", "Utils"),
        ("Level", "Utils"),
        ("High", "Level"),
        ("Pt", "Utils"),
        ("X", "Pt"),
        ("Deposit", "Account"),
        ("Balance", "Account"),
        ("Owner", "Account"),
        ("Run", "Main"),
        ("Utils", ""),
        ("Account", ""),
        ("Main", ""),
    ] {
        assert!(names.contains(&want), "{want:?} missing from {names:?}");
    }
    let kind = |n: &str| all.iter().find(|s| s.name == n).unwrap().kind;
    assert_eq!(kind("Twice"), SymbolKind::Function);
    assert_eq!(kind("Run"), SymbolKind::Method);
    assert_eq!(kind("Owner"), SymbolKind::Property);
    assert_eq!(kind("LIMIT"), SymbolKind::Constant);
    assert_eq!(kind("Level"), SymbolKind::Enum);
    assert_eq!(kind("Account"), SymbolKind::Class);
    assert_eq!(kind("Utils"), SymbolKind::Module);
    // No locals.
    assert!(!names.iter().any(|(n, _)| *n == "amount" || *n == "lv"));

    // Matching: exact, prefix, substring and subsequence; best first.
    let q = ws.workspace_symbols("dep", 10);
    assert_eq!(q[0].name, "Deposit");
    let q = ws.workspace_symbols("twice", 10);
    assert_eq!(q[0].name, "Twice");
    assert_eq!(q[0].location.uri, U);
    assert_eq!(q[0].location.range.start, Position::new(11, 16));
    let q = ws.workspace_symbols("dpst", 10);
    assert!(q.iter().any(|s| s.name == "Deposit"));
    assert!(ws.workspace_symbols("zzzz", 10).is_empty());
    assert_eq!(ws.workspace_symbols("", 3).len(), 3);
}

#[test]
fn closing_a_document_removes_it_from_the_project() {
    let mut ws = project();
    assert!(!ws.workspace_symbols("Twice", 5).is_empty());
    ws.close(U);
    assert!(ws.workspace_symbols("Twice", 5).is_empty());
    let d = ws.view(M).unwrap().diagnostics();
    assert!(
        d.iter().any(|d| d.code == "SEM003" || d.code == "SEM002"),
        "{d:?}"
    );
}
