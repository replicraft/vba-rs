//! Property tests for the project-wide features: no panics on arbitrary
//! edits across several documents, and rename edits that touch nothing but
//! identifiers.
#![allow(missing_docs)]

use proptest::prelude::*;
use vba_lexer::{LexOptions, TokenKind, lex};
use vba_lsp_core::*;

const LINES: &[&str] = &[
    "Option Explicit",
    "Attribute VB_Name = \"Alpha\"",
    "Attribute VB_Name = \"Beta\"",
    "VERSION 1.0 CLASS",
    "BEGIN",
    "  MultiUse = -1",
    "END",
    "Public Function Twice(ByVal n As Long) As Long",
    "Public Sub Run(Optional ByVal item As Variant)",
    "Private Sub Helper()",
    "Public Property Get Count() As Long",
    "Public Property Let Count(ByVal v As Long)",
    "End Function",
    "End Sub",
    "End Property",
    "Public Const LIMIT As Long = 10",
    "Public Enum Level",
    "    Low = 1",
    "    High",
    "End Enum",
    "Public Type Pt",
    "    X As Long",
    "    Y As Long",
    "End Type",
    "Public Total As Long",
    "Private mItem As Alpha",
    "    Dim a As Alpha, b As Beta, p As Pt, v As Variant, lv As Level",
    "    Set a = New Alpha",
    "    Total = Twice(LIMIT) + Count",
    "    Twice = n * 2",
    "    a.Run 1",
    "    a.Count = a.Count + 1",
    "    b.Total = p.X",
    "    v.Run item",
    "    Me.Count = 3",
    "    With a",
    "        .Run item:=2",
    "        .Count = .Count + 1",
    "    End With",
    "    lv = Level.High",
    "    p.Y = Alpha.Total",
    "    x = Beta.Twice(1)",
    "    Dim n As Long",
    "    n = n + undefinedName",
    "    For n = 1 To 3",
    "    Next n",
    "    GoTo done",
    "done:",
    "    Call Helper",
    "    Debug.Print Left(\"é😀\", 1)",
    "    x = = (",
    "#If VBA7 Then",
    "#End If",
    "",
];

fn doc_strategy() -> impl Strategy<Value = String> {
    let frag = prop_oneof![
        9 => prop::sample::select(LINES).prop_map(str::to_string),
        1 => "[ -~]{0,20}".prop_map(|s| s),
        1 => "\\PC{0,8}".prop_map(|s| s.replace(['\r', '\n'], "")),
    ];
    prop::collection::vec(frag, 0..30).prop_map(|v| v.join("\n"))
}

const URIS: &[&str] = &[
    "file:///p/Alpha.cls",
    "file:///p/Beta.bas",
    "file:///p/Form1.frm",
    "file:///p/Notes.txt",
];

#[derive(Debug, Clone)]
enum Action {
    Edit {
        doc: usize,
        l1: u32,
        c1: u32,
        l2: u32,
        c2: u32,
        text: String,
    },
    Open {
        doc: usize,
        text: String,
    },
    Close {
        doc: usize,
    },
}

fn action_strategy() -> impl Strategy<Value = Action> {
    let text = prop_oneof![
        Just(String::new()),
        "[ -~]{0,8}",
        "\\PC{0,4}",
        Just("\n".to_string()),
        prop::sample::select(LINES).prop_map(|s| format!("{s}\n")),
    ];
    prop_oneof![
        6 => (0usize..4, 0u32..40, 0u32..40, 0u32..40, 0u32..40, text)
            .prop_map(|(doc, l1, c1, l2, c2, text)| Action::Edit { doc, l1, c1, l2, c2, text }),
        1 => (0usize..4, doc_strategy()).prop_map(|(doc, text)| Action::Open { doc, text }),
        1 => (0usize..4).prop_map(|doc| Action::Close { doc }),
    ]
}

fn pos(l: u32, c: u32) -> Position {
    Position::new(l, c)
}

fn exercise(ws: &Workspace, probes: &[(u32, u32)], query: &str) {
    let uris: Vec<String> = ws.open_uris().into_iter().map(String::from).collect();
    for sym in ws.workspace_symbols(query, 50) {
        assert!(uris.contains(&sym.location.uri), "{sym:?}");
        assert!(sym.location.range.start <= sym.location.range.end);
    }
    for u in &uris {
        let v = ws.view(u).unwrap();
        let lines = v.line_count();
        let _ = v.diagnostics();
        for &(l, c) in probes {
            let p = pos(l % (lines + 1), c);
            let _ = v.hover(p);
            let _ = v.completions(p);
            for loc in ws.definition(u, p).unwrap() {
                assert!(uris.contains(&loc.uri), "{loc:?}");
            }
            let refs = ws.references(u, p, true).unwrap();
            for w in refs.windows(2) {
                assert!((&w[0].uri, w[0].range.start) <= (&w[1].uri, w[1].range.start));
            }
            let _ = ws.prepare_rename(u, p);
            if let Ok(files) = ws.rename(u, p, "renamedX") {
                for fe in files {
                    assert!(uris.contains(&fe.uri));
                    let text = ws.document(&fe.uri).unwrap().text();
                    let _ = apply_edits(&text, &fe.edits);
                }
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn no_panics_on_arbitrary_edits_across_documents(
        docs in prop::collection::vec(doc_strategy(), 1..4),
        actions in prop::collection::vec(action_strategy(), 0..8),
        probes in prop::collection::vec((0u32..40, 0u32..40), 0..4),
        query in "[a-zA-Z]{0,4}",
    ) {
        let mut ws = Workspace::new();
        for (i, d) in docs.iter().enumerate() {
            ws.open(URIS[i], d, 1);
        }
        exercise(&ws, &probes, &query);
        for (n, a) in actions.iter().enumerate() {
            match a {
                Action::Edit { doc, l1, c1, l2, c2, text } => {
                    let _ = ws.change(URIS[*doc], n as i32 + 2, &[TextChange {
                        range: Some(Range::new(pos(*l1, *c1), pos(*l2, *c2))),
                        text: text.clone(),
                    }]);
                }
                Action::Open { doc, text } => ws.open(URIS[*doc], text, n as i32 + 2),
                Action::Close { doc } => { ws.close(URIS[*doc]); }
            }
            exercise(&ws, &probes, &query);
        }
    }
}

// ---- rename ------------------------------------------------------------

const NAMES: &[&str] = &["alpha", "Beta", "gamma", "delta"];
const MEMBERS: &[&str] = &["item", "Count"];

#[derive(Debug, Clone)]
struct Names {
    func: &'static str,
    var: &'static str,
    param: &'static str,
    local: &'static str,
    member: &'static str,
    field: &'static str,
}

fn names_strategy() -> impl Strategy<Value = Names> {
    let n = || prop::sample::select(NAMES);
    (
        n(),
        n(),
        n(),
        n(),
        prop::sample::select(MEMBERS),
        prop::sample::select(MEMBERS),
    )
        .prop_map(|(func, var, param, local, member, field)| Names {
            func,
            var,
            param,
            local,
            member,
            field,
        })
}

fn project_sources(n: &Names, extra: &[&str]) -> Vec<(&'static str, String)> {
    let Names {
        func,
        var,
        param,
        local,
        member,
        field,
    } = n;
    let lib = format!(
        "Attribute VB_Name = \"Lib\"\nOption Explicit\nPublic {var} As Long\n\
         Public Function {func}(ByVal {param} As Long) As Long\n    Dim {local} As Long\n    \
         {local} = {param} + {var}\n    {func} = {local}\nEnd Function\n"
    );
    let cls = format!(
        "VERSION 1.0 CLASS\nBEGIN\n  MultiUse = -1\nEND\nAttribute VB_Name = \"Thing\"\n\
         Option Explicit\nPublic {field} As Long\nPublic Sub {member}(ByVal q As Long)\n    \
         {field} = q\n    Me.{field} = {field}\nEnd Sub\n"
    );
    let mut body = format!(
        "Option Explicit\nPublic Sub Run()\n    Dim t As Thing, r As Long\n    Set t = New Thing\n    \
         r = {func}(1) + {var}\n    t.{member} r\n    t.{field} = r\n    With t\n        \
         .{member} 2\n        .{field} = 3\n    End With\n    r = Lib.{func}(2)\n"
    );
    for e in extra {
        body.push_str("    ");
        body.push_str(e);
        body.push('\n');
    }
    body.push_str("End Sub\n");
    vec![
        ("file:///r/Lib.bas", lib),
        ("file:///r/Thing.cls", cls),
        ("file:///r/Main.bas", body),
    ]
}

fn lex_tokens(text: &str) -> Vec<(TokenKind, String)> {
    let out = lex(
        vba_lsp_core::FILE_ID,
        text.as_bytes(),
        LexOptions::for_profile(vba_profile::default_profile()),
    );
    out.tokens
        .iter()
        .map(|t| {
            (
                t.kind,
                text[t.span.start as usize..t.span.end as usize].to_string(),
            )
        })
        .collect()
}

/// The (document, code) pairs reported. Counts are not compared: identical
/// diagnostics at one position merge in the editor view, and a rename
/// changes the message text that decides whether they merge.
fn error_counts(ws: &Workspace, uris: &[&str]) -> Vec<(String, usize)> {
    let mut m: std::collections::BTreeSet<String> = Default::default();
    for u in uris {
        for d in ws.view(u).unwrap().diagnostics() {
            m.insert(format!("{u}:{}", d.code));
        }
    }
    m.into_iter().map(|k| (k, 1)).collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn rename_only_changes_identifier_bytes(
        names in names_strategy(),
        extra in prop::collection::vec(prop::sample::select(&[
            "r = r + 1",
            "Dim v As Variant",
            "v.item = 1",
            "v.Count = v.Count + 1",
            "Debug.Print \"alpha item\" ' Beta gamma",
            "r = t.Count",
            "GoTo lbl",
            "lbl:",
            "Static s As Long",
        ][..]), 0..4),
        new_name in prop::sample::select(&["zeta", "Alpha", "item", "r", "Run", "Lib", "Thing"][..]),
    ) {
        let sources = project_sources(&names, &extra);
        let mut ws = Workspace::new();
        for (u, s) in &sources {
            ws.open(u, s, 1);
        }
        let uris: Vec<&str> = sources.iter().map(|(u, _)| *u).collect();
        let before = error_counts(&ws, &uris);
        let mut successes = 0;
        for (uri, text) in &sources {
            for (line, l) in text.lines().enumerate() {
                let mut col = 0;
                for part in l.split_inclusive(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
                    let word_len = part.trim_end_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_').len();
                    if word_len > 0 {
                        let p = pos(line as u32, col as u32 + 1);
                        if let Ok(files) = ws.rename(uri, p, new_name) {
                            check_rename(&ws, &sources, &files, new_name, &before, &format!("{uri}:{line}:{col}"));
                            successes += 1;
                        }
                    }
                    col += part.len();
                }
            }
        }
        // A fresh, unused name must be accepted for many positions.
        if new_name == "zeta" {
            prop_assert!(successes > 20, "only {successes} renames succeeded");
        }
    }
}

fn check_rename(
    ws: &Workspace,
    sources: &[(&str, String)],
    files: &[FileEdit],
    new_name: &str,
    before: &[(String, usize)],
    at: &str,
) {
    assert!(!files.is_empty());
    let mut renamed = Vec::new();
    let mut old_names = std::collections::BTreeSet::new();
    for (uri, text) in sources {
        let Some(fe) = files.iter().find(|f| f.uri == *uri) else {
            renamed.push((*uri, text.clone()));
            continue;
        };
        let doc = ws.document(uri).unwrap();
        // Edits are ordered, disjoint, and each one replaces exactly one
        // identifier with the new name.
        let mut last_end = None;
        for e in &fe.edits {
            assert_eq!(e.new_text, new_name);
            if let Some(le) = last_end {
                assert!(le <= e.range.start, "overlap in {uri}: {:?}", fe.edits);
            }
            last_end = Some(e.range.end);
            let s = doc.text_map().span(doc.bytes(), e.range);
            let old = &text[s.start as usize..s.end as usize];
            let toks = lex_tokens(old);
            assert!(
                toks.len() == 1 && toks[0].0 == TokenKind::Ident && toks[0].1 == old,
                "edit over `{old}` in {uri} is not one identifier"
            );
            old_names.insert(old.to_ascii_lowercase());
        }
        let out = apply_edits(text, &fe.edits);
        // Outside the edited identifiers the text is byte-for-byte the same:
        // the token streams agree except for the renamed identifiers.
        let (a, b) = (lex_tokens(text), lex_tokens(&out));
        assert_eq!(a.len(), b.len(), "token count changed in {uri}");
        let mut differing = 0;
        for (x, y) in a.iter().zip(&b) {
            if x == y {
                continue;
            }
            differing += 1;
            assert_eq!(x.0, TokenKind::Ident);
            assert_eq!(y.0, TokenKind::Ident);
            assert_eq!(y.1, new_name);
        }
        assert!(differing <= fe.edits.len());
        let removed: usize = fe
            .edits
            .iter()
            .map(|e| {
                let s = doc.text_map().span(doc.bytes(), e.range);
                (s.end - s.start) as usize
            })
            .sum();
        assert_eq!(
            out.len() + removed,
            text.len() + new_name.len() * fe.edits.len()
        );
        renamed.push((*uri, out));
    }
    // All edited identifiers were spellings of one name.
    let norm: std::collections::BTreeSet<String> = old_names
        .iter()
        .map(|n| {
            n.trim_end_matches(['%', '&', '^', '!', '#', '@', '$'])
                .to_string()
        })
        .collect();
    assert_eq!(
        norm.len(),
        1,
        "several different names renamed: {old_names:?}"
    );

    // The renamed project is no worse than the original one.
    let mut ws2 = Workspace::new();
    for (u, t) in &renamed {
        ws2.open(u, t, 1);
    }
    let uris: Vec<&str> = renamed.iter().map(|(u, _)| *u).collect();
    let after = error_counts(&ws2, &uris);
    for (k, n) in &after {
        let was = before.iter().find(|(b, _)| b == k).map_or(0, |(_, c)| *c);
        assert!(
            *n <= was,
            "rename at {at} to {new_name} added {k}\n{renamed:#?}"
        );
    }
}
