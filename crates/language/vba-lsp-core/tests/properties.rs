//! Property tests: robustness on arbitrary edits, formatter laws, position
//! conversion.
#![allow(missing_docs)]

mod common;

use common::*;
use proptest::prelude::*;
use vba_lsp_core::*;

const LINES: &[&str] = &[
    "Option Explicit",
    "Public Sub Run()",
    "Private Function F(ByVal a As Long, Optional b) As Long",
    "End Sub",
    "End Function",
    "    Dim x As Long, y$",
    "    x = x + 1",
    "    If x > 1 Then",
    "    ElseIf x < 0 Then",
    "    Else",
    "    End If",
    "    For i = 1 To 10",
    "    Next i",
    "    Do While x",
    "    Loop",
    "    Select Case x",
    "        Case 1, 2",
    "        Case Else",
    "    End Select",
    "    With obj",
    "        .Prop = Left(\"abc\", 1) & \"é😀\"",
    "    End With",
    "lbl:",
    "    GoTo lbl",
    "    On Error Resume Next",
    "' comment",
    "    ' indented comment _",
    "    x = 1 + _",
    "        2",
    "#If VBA7 Then",
    "#Else",
    "#End If",
    "#Const FLAG = 1",
    "Public Enum E",
    "    A = 1",
    "End Enum",
    "Private Type T",
    "    m As Long",
    "End Type",
    "Declare PtrSafe Function G Lib \"k\" () As Long",
    "Public Property Get P() As Long",
    "End Property",
    "    Debug.Print x",
    "    Call F(1, b:=2)",
    "    s = \"unterminated",
    "    x = = (",
    "VERSION 1.0 CLASS",
    "BEGIN",
    "END",
    "\t\tx=1",
    "",
];

fn doc_strategy() -> impl Strategy<Value = String> {
    let frag = prop_oneof![
        8 => prop::sample::select(LINES).prop_map(str::to_string),
        1 => "[ -~]{0,30}".prop_map(|s| s),
        1 => "\\PC{0,12}".prop_map(|s| s.replace(['\r', '\n'], "")),
    ];
    (
        prop::collection::vec(frag, 0..40),
        prop::sample::select(&["\n", "\r\n", "\r"][..]),
    )
        .prop_map(|(v, nl)| v.join(nl))
}

#[derive(Debug, Clone)]
struct RawEdit {
    l1: u32,
    c1: u32,
    l2: u32,
    c2: u32,
    text: String,
}

fn edit_strategy() -> impl Strategy<Value = RawEdit> {
    (
        0u32..45,
        0u32..40,
        0u32..45,
        0u32..40,
        prop_oneof![
            Just(String::new()),
            "[ -~]{0,8}",
            "\\PC{0,4}",
            Just("\n".to_string()),
            Just("\r\n    ".to_string()),
            prop::sample::select(LINES).prop_map(|s| format!("{s}\n")),
        ],
    )
        .prop_map(|(l1, c1, l2, c2, text)| RawEdit {
            l1,
            c1,
            l2,
            c2,
            text,
        })
}

fn exercise(ws: &Workspace, probes: &[(u32, u32)]) {
    let v = ws.view(URI).unwrap();
    let lines = v.line_count();
    let _ = v.diagnostics();
    for s in v.document_symbols() {
        assert!(s.range.start <= s.range.end);
        assert!(s.selection_range.start >= s.range.start && s.selection_range.end <= s.range.end);
    }
    for f in v.folding_ranges() {
        assert!(
            f.start_line < f.end_line && f.end_line < lines,
            "{f:?} of {lines}"
        );
    }
    let toks = v.semantic_tokens();
    for w in toks.windows(2) {
        let a = (w[0].line, w[0].start);
        let b = (w[1].line, w[1].start);
        assert!(
            a < b && (a.0 != b.0 || a.1 + w[0].length <= b.1),
            "{:?} {:?}",
            w[0],
            w[1]
        );
    }
    assert!(toks.iter().all(|t| t.length > 0 && t.line < lines));
    let _ = encode_semantic_tokens(&toks);
    let ps: Vec<Position> = probes
        .iter()
        .map(|&(l, c)| pos(l % (lines + 1), c))
        .collect();
    for r in v.selection_ranges(&ps) {
        for w in r.chain.windows(2) {
            assert!(w[1].start <= w[0].start && w[0].end <= w[1].end && w[0] != w[1]);
        }
    }
    for &p in &ps {
        let _ = v.hover(p);
        let c = v.completions(p);
        assert!(
            c.items
                .windows(2)
                .all(|w| w[0].label.to_ascii_lowercase() <= w[1].label.to_ascii_lowercase())
        );
        let _ = v.signature_help(p);
        let _ = v.definition(p);
        let _ = v.references(p, true);
        let _ = v.prepare_rename(p);
        if let Ok(edits) = v.rename(p, "renamed_x") {
            let _ = apply_edits(&v.document().text(), &edits);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn no_panics_on_arbitrary_edits(
        src in doc_strategy(),
        edits in prop::collection::vec(edit_strategy(), 0..6),
        probes in prop::collection::vec((0u32..50, 0u32..40), 0..6),
    ) {
        let mut ws = open(&src);
        exercise(&ws, &probes);
        for (i, e) in edits.iter().enumerate() {
            ws.change(URI, i as i32 + 2, &[TextChange {
                range: Some(Range::new(pos(e.l1, e.c1), pos(e.l2, e.c2))),
                text: e.text.clone(),
            }]).unwrap();
            exercise(&ws, &probes);
        }
    }

    #[test]
    fn formatter_is_idempotent_and_only_changes_indentation(src in doc_strategy()) {
        let ws = open(&src);
        let once = apply_edits(&src, &ws.view(URI).unwrap().format(FormatOptions::default()));
        let strip = |s: &str| -> String {
            s.split_inclusive(['\n', '\r']).map(|l| l.trim_start_matches([' ', '\t'])).collect()
        };
        prop_assert_eq!(strip(&once), strip(&src));
        let ws2 = open(&once);
        let edits = ws2.view(URI).unwrap().format(FormatOptions::default());
        prop_assert!(edits.is_empty(), "second pass changed {:?}\n{}", edits, once);
        // tabs variant is idempotent too
        let tabs = FormatOptions { insert_spaces: false, ..FormatOptions::default() };
        let t1 = apply_edits(&src, &ws.view(URI).unwrap().format(tabs));
        prop_assert_eq!(strip(&t1), strip(&src));
        let ws3 = open(&t1);
        prop_assert!(ws3.view(URI).unwrap().format(tabs).is_empty());
    }

    #[test]
    fn formatting_does_not_change_the_token_stream(src in doc_strategy()) {
        let ws = open(&src);
        let out = apply_edits(&src, &ws.view(URI).unwrap().format(FormatOptions::default()));
        let sig = |s: &str| -> Vec<String> {
            let w = open(s);
            let v = w.view(URI).unwrap();
            let toks = v.tokens();
            toks.iter()
                .filter(|t| !matches!(t.kind(), vba_lexer::TokenKind::Whitespace))
                .map(|t| format!("{:?}:{}", t.kind(), String::from_utf8_lossy(t.text())))
                .collect()
        };
        prop_assert_eq!(sig(&out), sig(&src));
    }

    #[test]
    fn format_range_edits_are_a_subset_of_full_formatting(
        src in doc_strategy(), a in 0u32..45, b in 0u32..45,
    ) {
        let ws = open(&src);
        let v = ws.view(URI).unwrap();
        let full = v.format(FormatOptions::default());
        let part = v.format_range(Range::new(pos(a, 0), pos(b, 0)), FormatOptions::default());
        for e in &part {
            prop_assert!(full.contains(e));
        }
    }

    #[test]
    fn utf8_position_roundtrip(text in "([ -~]|\\PC|\n|\r\n|\r){0,80}") {
        let bytes = text.as_bytes();
        let map = TextMap::new(bytes, vba_source::SourceEncoding::Utf8);
        for (i, _) in text.char_indices().chain(std::iter::once((text.len(), ' '))) {
            let o = i as u32;
            // skip the middle of a CRLF pair
            if i > 0 && i < text.len() && bytes[i - 1] == b'\r' && bytes[i] == b'\n' { continue; }
            let p = map.position(bytes, o);
            // reference: UTF-16 length of the text since the last line break
            let before = &text[..i];
            let line_start = before.rfind(['\n', '\r']).map_or(0, |k| k + 1);
            let cols = before[line_start..].encode_utf16().count() as u32;
            prop_assert_eq!(p.character, cols);
            prop_assert_eq!(map.offset(bytes, p), o);
        }
    }

    #[test]
    fn position_conversion_is_monotonic_and_idempotent_for_any_bytes(
        bytes in prop::collection::vec(any::<u8>(), 0..120),
        enc in prop::sample::select(&[
            vba_source::SourceEncoding::Utf8,
            vba_source::SourceEncoding::CodePage(1252),
            vba_source::SourceEncoding::CodePage(932),
            vba_source::SourceEncoding::CodePage(936),
            vba_source::SourceEncoding::Unknown,
        ][..]),
    ) {
        let map = TextMap::new(&bytes, enc);
        let mut last = Position::new(0, 0);
        for o in 0..=bytes.len() as u32 {
            let p = map.position(&bytes, o);
            prop_assert!(p >= last, "not monotonic at {o}");
            last = p;
            let back = map.offset(&bytes, p);
            prop_assert!(back <= o);
            prop_assert_eq!(map.position(&bytes, back), p);
        }
    }

    #[test]
    fn incremental_edits_match_a_string_model(
        src in "([ -~]|é|😀|日|\n){0,60}",
        edits in prop::collection::vec((any::<prop::sample::Index>(), any::<prop::sample::Index>(), "([ -~]|é|😀|\n){0,6}"), 0..8),
    ) {
        let mut model = src.clone();
        let mut doc = Document::new(&src, 1, vba_source::SourceEncoding::Utf8);
        for (n, (a, b, text)) in edits.iter().enumerate() {
            let boundaries: Vec<usize> = model.char_indices().map(|(i, _)| i).chain(std::iter::once(model.len())).collect();
            let (mut x, mut y) = (boundaries[a.index(boundaries.len())], boundaries[b.index(boundaries.len())]);
            if x > y { std::mem::swap(&mut x, &mut y); }
            let to_pos = |off: usize| {
                let before = &model[..off];
                let line = before.matches('\n').count() as u32;
                let ls = before.rfind('\n').map_or(0, |k| k + 1);
                pos(line, before[ls..].encode_utf16().count() as u32)
            };
            let range = Range::new(to_pos(x), to_pos(y));
            doc.apply_changes(n as i32 + 2, &[TextChange { range: Some(range), text: text.clone() }]);
            model.replace_range(x..y, text);
            prop_assert_eq!(doc.text(), model.clone());
        }
    }
}
