//! Diagnostics, document symbols, folding, semantic tokens and selection
//! ranges.
#![allow(missing_docs)]

mod common;

use common::*;
use vba_lsp_core::*;

#[test]
fn clean_module_has_no_diagnostics() {
    let ws = open(MODULE);
    assert_eq!(ws.view(URI).unwrap().diagnostics(), vec![]);
}

#[test]
fn diagnostics_table() {
    // (source, line of the first diagnostic, code prefix, severity)
    let cases: &[(&str, u32, &str, Severity)] = &[
        ("Sub A()\n    x = = 1\nEnd Sub\n", 1, "SYN", Severity::Error),
        ("Sub A()\n    x = 1\n", 2, "SYN003", Severity::Error),
        (
            "Sub A()\n    s = \"abc\nEnd Sub\n",
            1,
            "LEX",
            Severity::Error,
        ),
        (
            "Declare Function F Lib \"k\" () As Long\n",
            0,
            "SYN010",
            Severity::Error,
        ),
    ];
    for (src, line, code, sev) in cases {
        let ws = open(src);
        let d = ws.view(URI).unwrap().diagnostics();
        let first = d
            .first()
            .unwrap_or_else(|| panic!("no diagnostic for {src:?}"));
        assert_eq!(first.range.start.line, *line, "{src:?}: {d:?}");
        assert!(first.code.starts_with(code), "{src:?}: {d:?}");
        assert_eq!(first.severity, *sev);
    }
}

#[test]
fn unbalanced_directive_is_reported() {
    let ws = open("#End If\n");
    let d = ws.view(URI).unwrap().diagnostics();
    assert!(d.iter().any(|d| d.code == "SYN040"), "{d:?}");
}

#[test]
fn profile_selection_changes_diagnostics() {
    let src = "Declare Function F Lib \"k\" () As Long\n";
    let mut ws = open(src);
    assert!(!ws.view(URI).unwrap().diagnostics().is_empty());
    let mut cfg = ws.config().clone();
    cfg.set_profile("vba7.1-excel-win32").unwrap();
    ws.set_config(cfg);
    assert_eq!(ws.view(URI).unwrap().diagnostics(), vec![]);
    let mut cfg = ws.config().clone();
    assert!(cfg.set_profile("nope").is_err());
}

#[test]
fn project_constants_drive_conditional_compilation() {
    let src = "#If DEBUGGING Then\nSub A()\nEnd Sub\n#Else\nSub B()\nEnd Sub\n#End If\n";
    let mut ws = open(src);
    let names = |ws: &Workspace| -> Vec<String> {
        ws.view(URI)
            .unwrap()
            .document_symbols()
            .into_iter()
            .map(|s| s.name)
            .collect()
    };
    assert_eq!(names(&ws), ["B"]);
    let mut cfg = ws.config().clone();
    cfg.constants.push(("DEBUGGING".into(), 1));
    ws.set_config(cfg);
    assert_eq!(names(&ws), ["A"]);
}

struct Sema;
impl SemanticDiagnostics for Sema {
    fn diagnose(&self, input: &SemanticInput<'_>) -> Vec<vba_diagnostics::Diagnostic> {
        vec![vba_diagnostics::Diagnostic::new(
            vba_diagnostics::Severity::Warning,
            vba_diagnostics::Code("SEM001"),
            input.file,
            vba_source::Span::new(0, 6),
            "semantic finding",
        )]
    }
}

#[test]
fn semantic_providers_plug_in() {
    let mut ws = open(MODULE);
    ws.add_diagnostics_provider(Box::new(Sema));
    let d = ws.view(URI).unwrap().diagnostics();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "SEM001");
    assert_eq!(d[0].severity, Severity::Warning);
    assert_eq!(slice(MODULE, d[0].range), "Option");
}

#[test]
fn symbols_outline() {
    let ws = open(MODULE);
    let syms = ws.view(URI).unwrap().document_symbols();
    let flat: Vec<(&str, SymbolKind)> = syms.iter().map(|s| (s.name.as_str(), s.kind)).collect();
    assert_eq!(
        flat,
        [
            ("MAX_ITEMS", SymbolKind::Constant),
            ("Color", SymbolKind::Enum),
            ("Point", SymbolKind::Struct),
            ("counter", SymbolKind::Field),
            ("Add", SymbolKind::Function),
            ("Run", SymbolKind::Method),
        ]
    );
    let members =
        |i: usize| -> Vec<&str> { syms[i].children.iter().map(|c| c.name.as_str()).collect() };
    assert_eq!(members(1), ["Red", "Green"]);
    assert_eq!(members(2), ["X", "Y"]);
    assert_eq!(members(4), ["total"]);
    assert_eq!(syms[4].children[0].kind, SymbolKind::Variable);
    assert_eq!(
        syms[4].detail,
        "Public Function Add(ByVal a As Long, Optional b As Long = 2) As Long"
    );
    assert_eq!(slice(MODULE, syms[4].selection_range), "Add");
    assert_eq!(syms[4].range.start, pos(15, 0));
    assert_eq!(syms[4].range.end, pos(24, 12));
}

#[test]
fn symbols_cover_declares_events_properties() {
    let src = "\
Declare PtrSafe Function GetTick Lib \"kernel32\" Alias \"GetTickCount\" () As Long
Public Event Changed(ByVal v As Long)
Dim a As Long, b(1 To 3) As String
Public Property Get P() As Long
End Property
Public Property Let P(ByVal v As Long)
End Property
";
    let ws = open(src);
    let syms = ws.view(URI).unwrap().document_symbols();
    let flat: Vec<(&str, SymbolKind)> = syms.iter().map(|s| (s.name.as_str(), s.kind)).collect();
    assert_eq!(
        flat,
        [
            ("GetTick", SymbolKind::Function),
            ("Changed", SymbolKind::Event),
            ("a", SymbolKind::Field),
            ("b", SymbolKind::Field),
            ("P", SymbolKind::Property),
            ("P", SymbolKind::Property),
        ]
    );
}

#[test]
fn symbols_survive_broken_code() {
    let src = "Sub A()\n    If x Then\n  y = \nEnd Sub\nSub B()\nEnd Sub\n";
    let ws = open(src);
    let names: Vec<String> = ws
        .view(URI)
        .unwrap()
        .document_symbols()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, ["A", "B"]);
}

#[test]
fn folding_ranges() {
    let ws = open(MODULE);
    let f: Vec<(u32, u32, FoldingKind)> = ws
        .view(URI)
        .unwrap()
        .folding_ranges()
        .iter()
        .map(|r| (r.start_line, r.end_line, r.kind))
        .collect();
    use FoldingKind::*;
    assert_eq!(
        f,
        [
            (3, 6, Region),
            (7, 10, Region),
            (13, 14, Comment),
            (15, 24, Region),
            (19, 22, Region),
            (26, 30, Region),
        ]
    );
}

#[test]
fn folding_conditional_compilation_and_select() {
    let src = "\
#If VBA7 Then
Sub A()
    Select Case 1
        Case 1
            x = 1
        Case Else
            x = 2
    End Select
End Sub
#Else
Sub A()
End Sub
#End If
";
    let ws = open(src);
    let f: Vec<(u32, u32)> = ws
        .view(URI)
        .unwrap()
        .folding_ranges()
        .iter()
        .map(|r| (r.start_line, r.end_line))
        .collect();
    assert!(f.contains(&(0, 8)), "{f:?}"); // active #If branch
    assert!(f.contains(&(9, 11)), "{f:?}"); // inactive #Else branch
    assert!(f.contains(&(2, 7)), "{f:?}"); // Select
    assert!(f.contains(&(3, 4)), "{f:?}"); // Case 1
}

fn tok_at(ws: &Workspace, line: u32, start: u32) -> (String, Vec<String>, u32) {
    let v = ws.view(URI).unwrap();
    let t = v
        .semantic_tokens()
        .into_iter()
        .find(|t| t.line == line && t.start == start)
        .unwrap_or_else(|| panic!("no token at {line}:{start}"));
    let mods = TOKEN_MODIFIERS
        .iter()
        .enumerate()
        .filter(|(i, _)| t.modifiers & (1 << i) != 0)
        .map(|(_, m)| m.to_string())
        .collect();
    (
        TOKEN_TYPES[t.token_type as usize].to_string(),
        mods,
        t.length,
    )
}

#[test]
fn semantic_token_table() {
    let ws = open(MODULE);
    // (line, col, type, modifiers, length)
    let cases: &[(u32, u32, &str, &[&str], u32)] = &[
        (0, 0, "keyword", &[], 6),                             // Option
        (0, 7, "keyword", &[], 8),                             // Explicit
        (2, 8, "keyword", &[], 5),                             // Const
        (2, 14, "variable", &["declaration", "readonly"], 9),  // MAX_ITEMS
        (2, 27, "type", &["defaultLibrary"], 4),               // Long
        (2, 34, "number", &[], 2),                             // 10
        (4, 4, "enumMember", &["declaration", "readonly"], 3), // Red
        (8, 4, "property", &["declaration"], 1),               // X
        (13, 0, "comment", &[], 19),                           // ' Adds two numbers.
        (15, 16, "function", &["declaration"], 3),             // Add
        (15, 20, "keyword", &[], 5),                           // ByVal
        (15, 26, "parameter", &["declaration"], 1),            // a
        (17, 0, "label", &["declaration"], 5),                 // start
        (18, 4, "variable", &[], 5),                           // total
        (18, 12, "parameter", &[], 1),                         // a use
        (18, 14, "operator", &[], 1),                          // +
        (19, 15, "variable", &["readonly"], 9),                // MAX_ITEMS use
        (21, 13, "label", &[], 5),                             // GoTo start
        (28, 8, "function", &["defaultLibrary"], 4),           // Left
        (28, 13, "string", &[], 5),                            // "abc"
        (28, 25, "function", &["defaultLibrary"], 4),          // CStr
        (28, 30, "function", &[], 3),                          // Add call
        (29, 4, "keyword", &[], 5),                            // Debug
    ];
    for &(l, c, ty, mods, len) in cases {
        let (t, m, n) = tok_at(&ws, l, c);
        assert_eq!(
            (t.as_str(), m, n),
            (
                ty,
                mods.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                len
            ),
            "at {l}:{c}"
        );
    }
}

#[test]
fn semantic_tokens_inactive_code_and_directives() {
    let src = "#If Win16 Then\n    x = 1\n#End If\n";
    let ws = open(src);
    let (t, m, _) = tok_at(&ws, 0, 0);
    assert_eq!((t.as_str(), m), ("macro", vec![]));
    let (t, m, n) = tok_at(&ws, 1, 4);
    assert_eq!(
        (t.as_str(), m, n),
        ("comment", vec!["inactive".to_string()], 1)
    );
}

#[test]
fn semantic_tokens_split_multiline_comment_and_encode() {
    let src = "x = 1 ' a _\n  b\ny = 2\n";
    let ws = open(src);
    let v = ws.view(URI).unwrap();
    let toks = v.semantic_tokens();
    assert!(toks.iter().all(|t| t.length > 0));
    let comment_lines: Vec<u32> = toks
        .iter()
        .filter(|t| t.token_type == token_type::COMMENT)
        .map(|t| t.line)
        .collect();
    assert_eq!(comment_lines, [0, 1]);
    let enc = encode_semantic_tokens(&toks);
    assert_eq!(enc.len(), toks.len() * 5);
    // relative encoding: second token on the first line starts after the first
    assert_eq!(&enc[0..2], &[0, toks[0].start]);
    let ranged = v.semantic_tokens_range(Range::new(pos(2, 0), pos(3, 0)));
    assert!(ranged.iter().all(|t| t.line == 2));
    assert!(!ranged.is_empty());
}

#[test]
fn selection_ranges_expand() {
    let (src, p) = cursor(&MODULE.replace("total = a + b", "tot|al = a + b"));
    let ws = open(&src);
    let r = &ws.view(URI).unwrap().selection_ranges(&[p])[0];
    assert_eq!(slice(&src, r.chain[0]), "total");
    let texts: Vec<String> = r.chain.iter().map(|c| slice(&src, *c)).collect();
    assert!(texts.contains(&"total = a + b".to_string()), "{texts:?}");
    assert!(texts.iter().any(|t| t.starts_with("Public Function Add")));
    assert_eq!(slice(&src, *r.chain.last().unwrap()), src);
    for w in r.chain.windows(2) {
        assert!(w[0] != w[1]);
        assert!(w[1].start <= w[0].start && w[0].end <= w[1].end);
    }
}

#[test]
fn selection_ranges_in_whitespace_and_empty_document() {
    let ws = open("");
    let r = ws
        .view(URI)
        .unwrap()
        .selection_ranges(&[pos(0, 0), pos(5, 5)]);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].chain.len(), 1);
    let ws = open("Sub A()\n    x = 1\nEnd Sub\n");
    let r = &ws.view(URI).unwrap().selection_ranges(&[pos(1, 1)])[0];
    assert!(r.chain.len() >= 2);
}
