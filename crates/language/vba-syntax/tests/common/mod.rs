//! Shared helpers for the integration tests.
#![allow(dead_code)]

use vba_lexer::TokenKind;
use vba_profile::{Profile, find_profile};
use vba_source::FileId;
use vba_syntax::{NodeKind, Parse, ParseOptions, SyntaxElement, SyntaxNode, parse};

pub fn profile(id: &str) -> &'static Profile {
    find_profile(id).expect("profile")
}

pub fn parse_with(src: &str, profile_id: &str, opts: &ParseOptions) -> Parse {
    let p = parse(FileId(0), src.as_bytes(), profile(profile_id), opts);
    assert_eq!(p.tree.text(), src.as_bytes(), "tree is not lossless");
    assert_eq!(p.tree.root().text(), src.as_bytes());
    p
}

pub fn parse_str(src: &str) -> Parse {
    parse_with(src, "vba7.1-excel-win64", &ParseOptions::tolerant())
}

/// Diagnostics as `CODE severity: message` lines.
pub fn diag_lines(p: &Parse) -> Vec<String> {
    p.diagnostics
        .iter()
        .map(|d| format!("{} {}: {}", d.code.0, d.severity.as_str(), d.message))
        .collect()
}

/// Parses and asserts there are no diagnostics at all.
pub fn parse_clean(src: &str) -> Parse {
    let p = parse_str(src);
    assert!(
        p.diagnostics.is_empty(),
        "unexpected diagnostics for {src:?}:\n{}",
        diag_lines(&p).join("\n")
    );
    p
}

/// A compact S-expression of a node: child nodes nest, tokens print as
/// their text; trivia and newlines are omitted.
pub fn sexp_node(n: SyntaxNode<'_>) -> String {
    let mut out = String::new();
    out.push('(');
    out.push_str(&format!("{:?}", n.kind()));
    for c in n.children() {
        match c {
            SyntaxElement::Node(c) => {
                out.push(' ');
                out.push_str(&sexp_node(c));
            }
            SyntaxElement::Token(t) => {
                if t.kind().is_trivia() || t.kind() == TokenKind::Newline {
                    continue;
                }
                out.push(' ');
                out.push_str(&String::from_utf8_lossy(t.text()));
            }
        }
    }
    out.push(')');
    out
}

pub fn sexp(p: &Parse) -> String {
    sexp_node(p.tree.root())
}

/// Wraps `body` in a `Sub`, parses it without diagnostics and returns the
/// S-expressions of the statements of the body, space separated.
pub fn stmts(body: &str) -> String {
    let src = format!("Sub T()\n{body}\nEnd Sub\n");
    let p = parse_clean(&src);
    let root = p.tree.root();
    let proc_ = root.child_node(NodeKind::ProcDecl).expect("proc");
    let block = proc_.child_node(NodeKind::Block).expect("block");
    block
        .child_nodes()
        .map(sexp_node)
        .collect::<Vec<_>>()
        .join(" ")
}

/// S-expression of the right-hand side expression of `x = <expr>`.
pub fn expr(e: &str) -> String {
    let src = format!("Sub T()\nx = {e}\nEnd Sub\n");
    let p = parse_clean(&src);
    let root = p.tree.root();
    let proc_ = root.child_node(NodeKind::ProcDecl).expect("proc");
    let block = proc_.child_node(NodeKind::Block).expect("block");
    let assign = block.child_node(NodeKind::AssignStmt).expect("assign");
    sexp_node(assign.child_nodes().nth(1).expect("rhs"))
}
