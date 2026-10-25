//! Lowering never panics and produces in-range, nested spans for any input.

use proptest::prelude::*;
use vba_ast::*;
use vba_profile::find_profile;
use vba_source::{FileId, Span};
use vba_syntax::ParseOptions;

fn within(inner: Span, outer: Span, what: &str) {
    assert!(
        outer.start <= inner.start && inner.end <= outer.end,
        "{what}: {inner:?} not inside {outer:?}"
    );
}

fn check_expr(e: &Expr, outer: Span) {
    within(e.span, outer, "expr");
    match &e.kind {
        ExprKind::Paren(x) | ExprKind::AddressOf(x) | ExprKind::FileNumber(x) => {
            check_expr(x, e.span)
        }
        ExprKind::Unary { operand, .. } => check_expr(operand, e.span),
        ExprKind::Binary { lhs, rhs, .. } => {
            check_expr(lhs, e.span);
            check_expr(rhs, e.span);
        }
        ExprKind::Range { lower, upper } => {
            check_expr(lower, e.span);
            check_expr(upper, e.span);
        }
        ExprKind::Member { target, .. } | ExprKind::Dict { target, .. } => {
            if let Some(t) = target {
                check_expr(t, e.span);
            }
        }
        ExprKind::Call { callee, args } => {
            check_expr(callee, e.span);
            for a in args {
                if let Some(v) = &a.value {
                    check_expr(v, e.span);
                }
            }
        }
        ExprKind::TypeOfIs { expr, .. } => check_expr(expr, e.span),
        _ => {}
    }
}

fn check_block(b: &Block, outer: Span) {
    for s in &b.stmts {
        within(s.span, outer, "stmt");
        check_stmt(s);
    }
}

fn check_stmt(s: &Stmt) {
    let sp = s.span;
    match &s.kind {
        StmtKind::Assign { target, value, .. } | StmtKind::Set { target, value } => {
            check_expr(target, sp);
            check_expr(value, sp);
        }
        StmtKind::If(i) => {
            check_expr(&i.cond, sp);
            check_block(&i.then_body, sp);
            for e in &i.else_ifs {
                within(e.span, sp, "elseif");
                check_expr(&e.cond, e.span);
                check_block(&e.body, e.span);
            }
            if let Some(b) = &i.else_body {
                check_block(b, sp);
            }
        }
        StmtKind::Select(sel) => {
            check_expr(&sel.subject, sp);
            for c in &sel.cases {
                within(c.span, sp, "case");
                check_block(&c.body, c.span);
            }
        }
        StmtKind::For(f) => {
            check_expr(&f.counter, sp);
            check_expr(&f.start, sp);
            check_expr(&f.end, sp);
            check_block(&f.body, sp);
        }
        StmtKind::ForEach(f) => {
            check_expr(&f.item, sp);
            check_expr(&f.collection, sp);
            check_block(&f.body, sp);
        }
        StmtKind::Do(d) => check_block(&d.body, sp),
        StmtKind::While { cond, body } | StmtKind::With { object: cond, body } => {
            check_expr(cond, sp);
            check_block(body, sp);
        }
        StmtKind::Call { callee, args, .. } => {
            check_expr(callee, sp);
            for a in args {
                if let Some(v) = &a.value {
                    check_expr(v, sp);
                }
            }
        }
        _ => {}
    }
}

fn check(src: &[u8]) {
    for id in [
        "vba7.1-excel-win64",
        "vba6-excel-win32",
        "access95-research",
    ] {
        let profile = find_profile(id).unwrap();
        let p = parse_module(FileId(0), src, profile, &ParseOptions::tolerant());
        let whole = Span::new(0, src.len() as u32);
        within(p.module.span, whole, "module");
        for item in &p.module.items {
            within(item.span(), whole, "item");
            if let Item::Proc(pr) = item {
                check_block(&pr.body, pr.span);
            }
        }
        for d in &p.diagnostics {
            within(d.span, whole, "diagnostic");
        }
        // Syntax errors imply errors in the combined diagnostics.
        let has_error_node = p
            .tree
            .root()
            .descendants()
            .iter()
            .any(|n| n.kind() == vba_syntax::NodeKind::Error);
        if has_error_node {
            assert!(p.diagnostics.iter().any(|d| d.code.0 == "AST001"));
        }
    }
}

const FRAGMENTS: &[&str] = &[
    " ",
    "\n",
    ":",
    ",",
    ";",
    ".",
    "!",
    "#",
    "(",
    ")",
    "=",
    "<>",
    ":=",
    "+",
    "-",
    "*",
    "'",
    "\"s\"",
    "\"",
    "12",
    "1.5",
    "&HFF",
    "x",
    "foo",
    "a(1)",
    "Sub",
    "End Sub",
    "Function",
    "End Function",
    "Property Get",
    "Dim",
    "As",
    "Integer",
    "Long",
    "New",
    "Set",
    "Call",
    "If",
    "Then",
    "Else",
    "ElseIf",
    "End If",
    "Select Case",
    "Case",
    "Case Else",
    "Is",
    "End Select",
    "For",
    "To",
    "Step",
    "Each",
    "In",
    "Next",
    "Do",
    "Loop",
    "While",
    "Until",
    "Wend",
    "With",
    "End With",
    "GoTo",
    "On Error",
    "Resume",
    "Exit",
    "ReDim",
    "Preserve",
    "Open",
    "Close",
    "Print",
    "Input",
    "Line Input",
    "Get",
    "Put",
    "Name",
    "Type",
    "End Type",
    "Enum",
    "End Enum",
    "Declare",
    "Lib",
    "Option",
    "Explicit",
    "Attribute",
    "Public",
    "Private",
    "Const",
    "ByVal",
    "Optional",
    "And",
    "Or",
    "Not",
    "Like",
    "TypeOf",
    "AddressOf",
    "Me",
    "#If",
    "#Else",
    "#End If",
    "Win64",
    "VERSION 1.0 CLASS\n",
    "BEGIN\n",
    "END\n",
    "~",
    "{",
];

fn soup() -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::sample::select(FRAGMENTS), 0..60).prop_map(|v| v.concat())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..300)) {
        check(&bytes);
    }

    #[test]
    fn token_soup(s in soup()) {
        check(s.as_bytes());
    }

    #[test]
    fn soup_inside_a_procedure(s in soup()) {
        check(format!("Sub T()\n{s}\nEnd Sub\n").as_bytes());
    }
}
