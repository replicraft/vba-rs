//! Error recovery: statement-level resynchronisation keeps later code
//! parsed, and the tree always reproduces the input.

mod common;

use common::*;
use vba_syntax::NodeKind;

fn count(p: &vba_syntax::Parse, kind: NodeKind) -> usize {
    p.tree
        .root()
        .descendants()
        .iter()
        .filter(|n| n.kind() == kind)
        .count()
}

#[test]
fn bad_statement_does_not_hide_the_next_one() {
    let p = parse_str("Sub A()\nx = )\ny = 2\nEnd Sub\n");
    assert!(p.has_errors());
    assert_eq!(count(&p, NodeKind::Error), 1);
    assert_eq!(count(&p, NodeKind::AssignStmt), 2);
    assert_eq!(
        p.diagnostics.len(),
        1,
        "one error per bad line: {:?}",
        diag_lines(&p)
    );
}

#[test]
fn colon_resynchronises_within_a_line() {
    let p = parse_str("Sub A()\nx = ) : y = 2\nEnd Sub\n");
    assert!(p.has_errors());
    assert_eq!(count(&p, NodeKind::AssignStmt), 2);
}

#[test]
fn missing_end_if_is_closed_by_end_sub() {
    let p = parse_str("Sub A()\nIf x Then\ny = 1\nEnd Sub\nSub B()\nEnd Sub\n");
    assert_eq!(count(&p, NodeKind::ProcDecl), 2);
    let lines = diag_lines(&p);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("missing `End If`"));
}

#[test]
fn missing_end_sub_is_closed_by_next_procedure() {
    let p = parse_str("Sub A()\nIf x Then\ny = 1\nSub B()\nz = 2\nEnd Sub\n");
    assert_eq!(count(&p, NodeKind::ProcDecl), 2);
    let lines = diag_lines(&p);
    assert!(
        lines.iter().any(|l| l.contains("missing `End If`")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("missing `End Sub`")),
        "{lines:?}"
    );
}

#[test]
fn mismatched_closer_is_reported_once() {
    let p = parse_str("Sub A()\nFor i = 1 To 3\nIf x Then\nNext\nEnd Sub\n");
    assert_eq!(count(&p, NodeKind::ForStmt), 1);
    assert!(p.has_errors());
}

#[test]
fn stray_terminators_become_error_nodes() {
    for src in [
        "Sub A()\nEnd If\nEnd Sub\n",
        "Sub A()\nWend\nEnd Sub\n",
        "Sub A()\nElse\nEnd Sub\n",
        "Sub A()\nCase 1\nEnd Sub\n",
    ] {
        let p = parse_str(src);
        assert!(p.has_errors(), "{src:?}");
        assert_eq!(count(&p, NodeKind::Error), 1, "{src:?}");
        assert_eq!(count(&p, NodeKind::ProcDecl), 1);
    }
}

#[test]
fn garbage_between_procedures() {
    let p = parse_str("Sub A()\nEnd Sub\n@@@ garbage ###\nSub B()\nEnd Sub\n");
    assert_eq!(count(&p, NodeKind::ProcDecl), 2);
    assert!(p.has_errors());
}

#[test]
fn truncated_input_everywhere() {
    let full = "Option Explicit\nPublic Type T\nx As Long\nEnd Type\nSub A(ByVal p As Long)\nIf p Then\nFor i = 1 To 3\nSelect Case i\nCase 1, 2\nDo While True\nWith x\n.y = Foo(1, 2)\nEnd With\nLoop\nEnd Select\nNext\nEnd If\nEnd Sub\n";
    for end in 0..=full.len() {
        let _ = parse_str(&full[..end]);
    }
}

#[test]
fn lexer_errors_surface_as_diagnostics() {
    let p = parse_str("Sub A()\nx = \"abc\nEnd Sub\n");
    assert!(diag_lines(&p).iter().any(|l| l.starts_with("LEX001")));
    assert_eq!(count(&p, NodeKind::ProcDecl), 1);
}

#[test]
fn non_utf8_bytes_round_trip() {
    let src: &[u8] = b"Sub A()\n  x = \"caf\xe9\" ' \xff\xfe\nEnd Sub\n";
    let p = vba_syntax::parse(
        vba_source::FileId(0),
        src,
        profile("vba7.1-excel-win64"),
        &vba_syntax::ParseOptions::tolerant(),
    );
    assert_eq!(p.tree.text(), src);
    assert!(p.diagnostics.is_empty(), "{:?}", diag_lines(&p));
}

#[test]
fn diagnostics_are_sorted_and_within_the_source() {
    let src = "Sub A()\nx = = 1\ny = ) \nEnd Sub\nz = 1\n";
    let p = parse_str(src);
    let spans: Vec<_> = p.diagnostics.iter().map(|d| d.span.start).collect();
    let mut sorted = spans.clone();
    sorted.sort();
    assert_eq!(spans, sorted);
    for d in &p.diagnostics {
        assert!(d.span.end as usize <= src.len());
    }
}
