//! UTF-16 columns against byte buffers in different encodings, and
//! incremental sync.
#![allow(missing_docs)]

mod common;

use common::*;
use vba_lsp_core::*;

#[test]
fn hover_after_astral_characters_uses_utf16_columns() {
    // The emoji is 4 bytes in UTF-8 and 2 UTF-16 units.
    let src = "Sub A()\n    s = \"😀é\" & Left(\"a\", 1)\nEnd Sub\n";
    let line = src.lines().nth(1).unwrap();
    let col = line[..line.find("Left").unwrap()].encode_utf16().count() as u32;
    assert_eq!(col, 16);
    let ws = open(src);
    let v = ws.view(URI).unwrap();
    let h = v.hover(pos(1, col + 1)).expect("hover on Left");
    assert!(h.contents.contains("Left("));
    assert_eq!(h.range, Range::new(pos(1, col), pos(1, col + 4)));
    // semantic token for Left at the same UTF-16 column
    assert!(
        v.semantic_tokens()
            .iter()
            .any(|t| t.line == 1 && t.start == col && t.length == 4)
    );
}

#[test]
fn crlf_and_cr_line_endings() {
    for nl in ["\r\n", "\r", "\n"] {
        let src = "Sub A()\n    Dim x\n    x = 1\nEnd Sub\n".replace('\n', nl);
        let ws = open(&src);
        let v = ws.view(URI).unwrap();
        assert_eq!(v.references(pos(2, 5), true).len(), 2, "{nl:?}");
        let syms = v.document_symbols();
        assert_eq!(syms[0].range.end, pos(3, 7), "{nl:?}");
    }
}

#[test]
fn windows_1252_documents() {
    let mut ws = Workspace::new();
    let mut cfg = ws.config().clone();
    cfg.encoding = parse_encoding("cp1252").unwrap();
    ws.set_config(cfg);
    ws.open(
        URI,
        "Sub A()\n    s = \"café €\" & Left(\"a\", 1)\nEnd Sub\n",
        1,
    );
    let doc = ws.document(URI).unwrap();
    // é and € are one byte each in the buffer
    assert!(doc.bytes().contains(&0xE9) && doc.bytes().contains(&0x80));
    assert_eq!(
        doc.text(),
        "Sub A()\n    s = \"café €\" & Left(\"a\", 1)\nEnd Sub\n"
    );
    let v = ws.view(URI).unwrap();
    assert!(v.diagnostics().is_empty());
    let col = "    s = \"café €\" & ".encode_utf16().count() as u32;
    assert!(v.hover(pos(1, col + 1)).unwrap().contents.contains("Left("));
    // an edit that inserts a non-Latin character degrades to `?`
    ws.change(
        URI,
        2,
        &[TextChange {
            range: Some(Range::new(pos(1, 12), pos(1, 13))),
            text: "日".into(),
        }],
    )
    .unwrap();
    assert!(ws.document(URI).unwrap().text().contains("caf?"));
}

#[test]
fn shift_jis_documents() {
    let mut ws = Workspace::new();
    ws.open_with_encoding(
        URI,
        "Sub A()\n    s = \"日本語\" & Left(\"a\", 1)\nEnd Sub\n",
        1,
        parse_encoding("932").unwrap(),
    );
    let doc = ws.document(URI).unwrap();
    // three double-byte characters take six bytes but three columns
    let line_bytes = 4 + 4 + 1 + 6;
    assert!(doc.bytes().len() > line_bytes);
    let v = ws.view(URI).unwrap();
    let col = "    s = \"日本語\" & ".encode_utf16().count() as u32;
    let h = v.hover(pos(1, col + 1)).expect("hover");
    assert!(h.contents.contains("Left("));
    assert_eq!(h.range, Range::new(pos(1, col), pos(1, col + 4)));
    // incremental edit addressed in UTF-16 columns
    ws.change(
        URI,
        2,
        &[TextChange {
            range: Some(Range::new(pos(1, 10), pos(1, 11))),
            text: "中".into(),
        }],
    )
    .unwrap();
    assert!(ws.document(URI).unwrap().text().contains("日中語"));
}

#[test]
fn incremental_sync_keeps_features_consistent() {
    let mut ws = open("Sub A()\nEnd Sub\n");
    ws.change(
        URI,
        2,
        &[TextChange {
            range: Some(Range::new(pos(1, 0), pos(1, 0))),
            text: "    Dim total As Long\n    total = 1\n".into(),
        }],
    )
    .unwrap();
    {
        let v = ws.view(URI).unwrap();
        assert_eq!(v.references(pos(2, 5), true).len(), 2);
    }
    // rename the declaration through an edit; the old reference no longer resolves to it
    ws.change(
        URI,
        3,
        &[TextChange {
            range: Some(Range::new(pos(1, 8), pos(1, 13))),
            text: "sum".into(),
        }],
    )
    .unwrap();
    let v = ws.view(URI).unwrap();
    assert_eq!(v.references(pos(1, 9), true).len(), 1);
    assert_eq!(ws.document(URI).unwrap().version(), 3);
}

#[test]
fn unicode_and_out_of_range_edits_do_not_panic() {
    let mut ws = open("'é😀\nSub A()\nEnd Sub\n");
    for r in [
        Range::new(pos(0, 2), pos(0, 3)), // inside the surrogate pair
        Range::new(pos(9, 9), pos(0, 0)), // reversed and out of range
        Range::new(pos(0, 99), pos(1, 99)),
    ] {
        ws.change(
            URI,
            5,
            &[TextChange {
                range: Some(r),
                text: "x".into(),
            }],
        )
        .unwrap();
        let v = ws.view(URI).unwrap();
        let _ = (
            v.diagnostics(),
            v.document_symbols(),
            v.semantic_tokens(),
            v.folding_ranges(),
        );
    }
}
