//! Hover, definition, references and rename.
#![allow(missing_docs)]

mod common;

use common::*;
use vba_lsp_core::*;

fn hover_text(src: &str) -> Option<String> {
    let (text, p) = cursor(src);
    let ws = open(&text);
    ws.view(URI).unwrap().hover(p).map(|h| h.contents)
}

#[test]
fn hover_table() {
    let with = |body: &str| MODULE.replace("    Debug.Print s", body);
    let cases: Vec<(String, &str)> = vec![
        (
            with("    Debug.Print s\n    Di|m x As Long"),
            "Declares variables",
        ),
        (with("    Debug.Print s\n    x = Le|ft(\"a\", 1)"), "Left("),
        (
            with("    Debug.Print s\n    x = Le|ft(\"a\", 1)"),
            "VBA.Strings.Left",
        ),
        (with("    Debug.Print s\n    x = Mi|d$(s, 1)"), "Mid("),
        (
            with("    Debug.Print s\n    x = VBA.Strings.Le|ft(s, 1)"),
            "Left(",
        ),
        (
            with("    Debug.Print s\n    x = Ad|d(1)"),
            "Public Function Add(ByVal a As Long, Optional b As Long = 2) As Long",
        ),
        (
            with("    Debug.Print s\n    x = cou|nter"),
            "Private counter As Long",
        ),
        (
            with("    Debug.Print s\n    x = MAX_|ITEMS"),
            "Private Const MAX_ITEMS As Long = 10",
        ),
        (
            with("    Debug.Print s\n    x = Color.Gr|een"),
            "(enum member) Green",
        ),
        (
            with("    Debug.Print s\n    x = Color.Green")
                .replace("Dim s As String", "Dim s As Str|ing"),
            "Type: text",
        ),
        (with("    Debug.Print s\n    x = Tr|ue"), "True"),
    ];
    for (src, needle) in cases {
        let text = hover_text(&src);
        let text = text.unwrap_or_else(|| panic!("no hover for {src}"));
        assert!(text.contains(needle), "{needle:?} not in {text:?}");
    }
}

#[test]
fn hover_none_on_whitespace_and_unknown() {
    assert!(hover_text("Sub A()\n  |  x = 1\nEnd Sub\n").is_none());
    // An undeclared variable is an implicit Variant for the analysis.
    let h = hover_text("Sub A()\n    unk|nown = 1\nEnd Sub\n").unwrap();
    assert!(h.contains("unknown As Variant"), "{h}");
    assert!(hover_text("Sub A()\n    x = 1 ' co|mment\nEnd Sub\n").is_none());
}

#[test]
fn hover_in_file_declaration_shadows_library() {
    let h =
        hover_text("Function Left(x)\nEnd Function\nSub A()\n    y = Le|ft(1)\nEnd Sub\n").unwrap();
    assert!(h.contains("Function Left(x As Variant) As Variant"), "{h}");
    assert!(!h.contains("VBA.Strings"), "{h}");
}

#[test]
fn hover_stdlib_signature_has_types_and_optional_markers() {
    let h = hover_text("Sub A()\n    y = Mi|d(\"abc\", 2)\nEnd Sub\n").unwrap();
    assert!(h.contains("Mid(String As Variant"), "{h}");
    assert!(h.contains("[Length As Variant]"), "{h}");
    assert!(h.contains("Mid$"), "{h}");
}

fn def(src: &str) -> (String, Vec<String>) {
    let (text, p) = cursor(src);
    let ws = open(&text);
    let v = ws.view(URI).unwrap();
    let d = v.definition(p).iter().map(|r| slice(&text, *r)).collect();
    (text, d)
}

#[test]
fn definition_and_references_table() {
    // (source with cursor, expected definition texts, reference count with decl)
    let cases: &[(&str, usize, usize)] = &[
        (
            "Sub A()\n    Dim x As Long\n    x = 1\n    y = |x + x\nEnd Sub\n",
            1,
            4,
        ),
        (
            "Sub A()\n    Dim x As Long\n    |x = 1\n    y = x\nEnd Sub\n",
            1,
            3,
        ),
        // local shadows module variable
        (
            "Dim v As Long\nSub A()\n    Dim v As Long\n    |v = 1\nEnd Sub\nSub B()\n    v = 2\nEnd Sub\n",
            1,
            2,
        ),
        (
            "Dim v As Long\nSub A()\n    Dim v As Long\n    v = 1\nEnd Sub\nSub B()\n    |v = 2\nEnd Sub\n",
            1,
            2,
        ),
        // parameters
        (
            "Sub A(ByVal p As Long)\n    q = |p\nEnd Sub\nSub B(p)\nEnd Sub\n",
            1,
            2,
        ),
        // procedures are visible module-wide, calls and function-result assignments count
        (
            "Function F() As Long\n    F = 1\nEnd Function\nSub B()\n    x = |F()\nEnd Sub\n",
            1,
            3,
        ),
        // labels
        ("Sub A()\n    GoTo |done\n    x = 1\ndone:\nEnd Sub\n", 1, 2),
        (
            "Sub A()\n    On Error GoTo h\n    Resume |h\nh:\nEnd Sub\n",
            1,
            3,
        ),
        // user types and enums
        (
            "Type T\n    a As Long\nEnd Type\nDim v As |T\nDim w As T\n",
            1,
            3,
        ),
        (
            "Enum E\n    one\n    two\nEnd Enum\nSub A()\n    x = E.|two\n    y = two\nEnd Sub\n",
            1,
            3,
        ),
        // constants, case-insensitive match and type suffix
        ("Const K = 1\nSub A()\n    x = k + |K\nEnd Sub\n", 1, 3),
        (
            "Sub A()\n    Dim s$\n    |s = \"a\"\n    s$ = \"b\"\nEnd Sub\n",
            1,
            3,
        ),
        // a name used in another procedure's local scope does not leak
        (
            "Sub A()\n    Dim loc\n    |loc = 1\nEnd Sub\nSub B()\n    loc = 2\nEnd Sub\n",
            1,
            2,
        ),
    ];
    for (src, defs, refs) in cases {
        let (text, p) = cursor(src);
        let ws = open(&text);
        let v = ws.view(URI).unwrap();
        assert_eq!(v.definition(p).len(), *defs, "defs of {src}");
        assert_eq!(v.references(p, true).len(), *refs, "refs of {src}");
        assert_eq!(
            v.references(p, false).len(),
            refs - defs,
            "refs without declaration of {src}"
        );
    }
}

#[test]
fn definition_points_at_the_declaration_name() {
    let (text, d) = def(&MODULE.replace("total > MAX", "tot|al > MAX"));
    assert_eq!(d, ["total"]);
    let (text2, p) = cursor(&MODULE.replace("total > MAX", "tot|al > MAX"));
    let ws = open(&text2);
    let r = ws.view(URI).unwrap().definition(p)[0];
    assert_eq!(r.start, pos(16, 8));
    assert_eq!(text, text2);
}

#[test]
fn property_pairs_have_several_definitions() {
    let src = "\
Public Property Get P() As Long
End Property
Public Property Let P(ByVal v As Long)
End Property
Sub A()
    x = |P
End Sub
";
    let (text, p) = cursor(src);
    let ws = open(&text);
    let v = ws.view(URI).unwrap();
    assert_eq!(v.definition(p).len(), 2);
    assert_eq!(v.references(p, true).len(), 3);
}

#[test]
fn unresolved_names_and_members_have_no_definition() {
    for src in [
        "Sub A()\n    x = un|known\nEnd Sub\n",
        "Type T\n    a As Long\nEnd Type\nSub A()\n    Dim v As T\n    v.|a = 1\nEnd Sub\n",
        "Sub A()\n    x = Le|ft(\"a\", 1)\nEnd Sub\n",
        "Sub A()\n    x = 1 ' |x\nEnd Sub\n",
    ] {
        let (text, p) = cursor(src);
        let ws = open(&text);
        let v = ws.view(URI).unwrap();
        assert!(v.definition(p).is_empty(), "{src}");
        assert!(v.references(p, true).is_empty(), "{src}");
    }
}

#[test]
fn named_arguments_are_not_references_to_variables() {
    let src = "Sub F(ByVal b As Long)\nEnd Sub\nSub A()\n    Dim b\n    |F b:=b\nEnd Sub\n";
    let (text, p) = cursor(src);
    let ws = open(&text);
    let v = ws.view(URI).unwrap();
    // `b:=` is the argument name, `b` after it is the local variable
    let b_pos = {
        let (_, p) = cursor(&text.replace("b:=b", "b:=|b"));
        p
    };
    assert_eq!(v.references(b_pos, true).len(), 2);
    assert_eq!(v.references(p, true).len(), 2); // F: decl + call
}

fn rename(src: &str, new: &str) -> Result<String, Error> {
    let (text, p) = cursor(src);
    let ws = open(&text);
    let v = ws.view(URI).unwrap();
    let edits = v.rename(p, new)?;
    Ok(apply_edits(&text, &edits))
}

#[test]
fn rename_table() {
    let ok: &[(&str, &str, &str)] = &[
        (
            "Sub A()\n    Dim x As Long\n    |x = x + 1\nEnd Sub\n",
            "count",
            "Sub A()\n    Dim count As Long\n    count = count + 1\nEnd Sub\n",
        ),
        (
            "Function F() As Long\n    F = 1\nEnd Function\nSub A()\n    y = |F()\nEnd Sub\n",
            "Compute",
            "Function Compute() As Long\n    Compute = 1\nEnd Function\nSub A()\n    y = Compute()\nEnd Sub\n",
        ),
        (
            "Sub A()\n    Dim s$\n    |s = \"a\"\n    s$ = \"b\"\nEnd Sub\n",
            "t",
            "Sub A()\n    Dim t$\n    t = \"a\"\n    t$ = \"b\"\nEnd Sub\n",
        ),
        (
            "Sub A()\n    GoTo |done\ndone:\nEnd Sub\n",
            "finish",
            "Sub A()\n    GoTo finish\nfinish:\nEnd Sub\n",
        ),
        (
            "Enum E\n    one\nEnd Enum\nSub A()\n    x = E.|one\nEnd Sub\n",
            "uno",
            "Enum E\n    uno\nEnd Enum\nSub A()\n    x = E.uno\nEnd Sub\n",
        ),
        // renaming a module variable to a name only used in other procedures
        (
            "Dim g\nSub A()\n    |g = 1\nEnd Sub\nSub B()\n    Dim z\nEnd Sub\n",
            "h",
            "Dim h\nSub A()\n    h = 1\nEnd Sub\nSub B()\n    Dim z\nEnd Sub\n",
        ),
    ];
    for (src, new, expected) in ok {
        assert_eq!(&rename(src, new).unwrap(), expected, "{src}");
    }
}

#[test]
fn rename_rejections() {
    // (source, new name, expected error kind)
    let cases: &[(&str, &str, &str)] = &[
        (
            "Sub A()\n    Dim x, y\n    |x = 1\nEnd Sub\n",
            "y",
            "invalid",
        ),
        // module variable renamed to a name that a local of a using procedure has
        (
            "Dim g\nSub A()\n    Dim z\n    |g = z\nEnd Sub\n",
            "z",
            "invalid",
        ),
        // local renamed to a module name that is used in the same procedure
        (
            "Dim g\nSub A()\n    Dim z\n    |z = g\nEnd Sub\n",
            "g",
            "invalid",
        ),
        (
            "Sub A()\n    Dim x\n    |x = 1\nEnd Sub\n",
            "End",
            "invalid",
        ),
        (
            "Sub A()\n    Dim x\n    |x = 1\nEnd Sub\n",
            "1abc",
            "invalid",
        ),
        (
            "Sub A()\n    Dim x\n    |x = 1\nEnd Sub\n",
            "a b",
            "invalid",
        ),
        ("Sub A()\n    Dim x\n    |x = 1\nEnd Sub\n", "", "invalid"),
        ("Sub A()\n    Dim x\n    |x = 1\nEnd Sub\n", "y$", "invalid"),
        ("Sub A()\n    Dim x\n    |x = 1\nEnd Sub\n", "é", "invalid"),
        (
            "Sub A()\n    x = Le|ft(\"a\", 1)\nEnd Sub\n",
            "L",
            "unsupported",
        ),
        ("Sub A()\n    obj.Me|th = 1\nEnd Sub\n", "m", "unsupported"),
        ("Sub A()\n    Dim x\n    |Dim y\nEnd Sub\n", "m", "invalid"),
        ("Sub A()\n    x = unk|nown\nEnd Sub\n", "m", "invalid"),
    ];
    for (src, new, kind) in cases {
        let err = rename(src, new).expect_err(src);
        let got = match err {
            Error::Invalid(_) => "invalid",
            Error::Unsupported(_) => "unsupported",
            Error::NotOpen(_) => "notopen",
        };
        assert_eq!(got, *kind, "{src} -> {new}: {err}");
    }
}

#[test]
fn rename_to_same_case_variant_is_allowed() {
    assert_eq!(
        rename("Sub A()\n    Dim x\n    |x = 1\nEnd Sub\n", "X").unwrap(),
        "Sub A()\n    Dim X\n    X = 1\nEnd Sub\n"
    );
}

#[test]
fn prepare_rename_returns_the_name_range() {
    let (text, p) = cursor(&MODULE.replace("total = a", "tot|al = a"));
    let ws = open(&text);
    let r = ws.view(URI).unwrap().prepare_rename(p).unwrap();
    assert_eq!(slice(&text, r), "total");
}

#[test]
fn requests_on_unopened_documents_fail() {
    let ws = Workspace::new();
    assert!(matches!(ws.view("file:///none"), Err(Error::NotOpen(_))));
}
