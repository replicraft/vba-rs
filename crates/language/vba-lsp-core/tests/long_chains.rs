//! Editor features on source with hundreds of thousands of `ElseIf` and `Case` clauses
//! must not use native stack in proportion to the clause count.
#![allow(missing_docs)]

mod common;

use common::*;
use vba_lsp_core::*;

const CLAUSES: usize = 30_000;

fn elseif_chain() -> String {
    let mut s = String::from("Sub T()\nIf x = 0 Then\ny = 0\n");
    for i in 1..CLAUSES {
        s.push_str(&format!("ElseIf x = {i} Then\ny = {i}\n"));
    }
    s.push_str("End If\nEnd Sub\n");
    s
}

fn case_chain() -> String {
    let mut s = String::from("Sub T()\nSelect Case x\n");
    for i in 0..CLAUSES {
        s.push_str(&format!("Case {i}\ny = {i}\n"));
    }
    s.push_str("End Select\nEnd Sub\n");
    s
}

fn exercise(src: String) {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let ws = open(&src);
            let view = ws.view(URI).unwrap();
            let _ = view.diagnostics();
            let _ = view.folding_ranges();
            let _ = view.document_symbols();
            let _ = view.semantic_tokens();
            let _ = view.selection_ranges(&[Position::new(10, 2)]);
            let _ = view.completions(Position::new(2, 1));
            let edits = view.format(FormatOptions::default());
            let _ = apply_edits(&src, &edits);
        })
        .unwrap()
        .join()
        .expect("the thread must not overflow its stack");
}

#[test]
fn editor_features_survive_a_very_long_elseif_chain() {
    exercise(elseif_chain());
}

#[test]
fn editor_features_survive_a_very_long_case_chain() {
    exercise(case_chain());
}
