//! Expression precedence, associativity and the grammar's ambiguities
//! (MS-VBAL §5.6 and §5.4).

mod common;

use common::*;

/// Fully parenthesised rendering of an expression S-expression, keeping
/// only the operator structure: `(B l op r)`.
fn bin(src: &str) -> String {
    // Render BinaryExpr/UnaryExpr/Call nesting compactly using the tree.
    use vba_syntax::{NodeKind, SyntaxNode};
    fn go(n: SyntaxNode<'_>) -> String {
        match n.kind() {
            NodeKind::BinaryExpr => {
                let mut it = n.child_nodes();
                let l = go(it.next().unwrap());
                let op = n
                    .significant_tokens()
                    .next()
                    .map(|t| String::from_utf8_lossy(t.text()).to_string())
                    .unwrap();
                let r = go(it.next().unwrap());
                format!("({l} {op} {r})")
            }
            NodeKind::UnaryExpr => {
                let op = n
                    .significant_tokens()
                    .next()
                    .map(|t| String::from_utf8_lossy(t.text()).to_string())
                    .unwrap();
                format!("({op} {})", go(n.child_nodes().next().unwrap()))
            }
            NodeKind::ParenExpr => format!("[{}]", go(n.child_nodes().next().unwrap())),
            NodeKind::CallExpr => {
                let mut it = n.child_nodes();
                let callee = go(it.next().unwrap());
                let args: Vec<String> = it
                    .next()
                    .unwrap()
                    .child_nodes()
                    .map(|a| a.child_nodes().next().map(go).unwrap_or_default())
                    .collect();
                format!("{callee}({})", args.join(","))
            }
            NodeKind::MemberExpr => {
                let name = n.significant_tokens().last().unwrap();
                match n.child_nodes().next() {
                    Some(l) => format!("{}.{}", go(l), String::from_utf8_lossy(name.text())),
                    None => format!(".{}", String::from_utf8_lossy(name.text())),
                }
            }
            _ => String::from_utf8_lossy(n.text()).trim().to_string(),
        }
    }
    let full = format!("Sub T()\nx = {src}\nEnd Sub\n");
    let p = parse_clean(&full);
    let root = p.tree.root();
    let proc_ = root.child_node(vba_syntax::NodeKind::ProcDecl).unwrap();
    let block = proc_.child_node(vba_syntax::NodeKind::Block).unwrap();
    let assign = block.child_node(vba_syntax::NodeKind::AssignStmt).unwrap();
    go(assign.child_nodes().nth(1).unwrap())
}

#[test]
fn precedence_table() {
    let cases: &[(&str, &str)] = &[
        // arithmetic
        ("a + b * c", "(a + (b * c))"),
        ("a * b + c", "((a * b) + c)"),
        ("a - b - c", "((a - b) - c)"),
        ("a / b * c", "((a / b) * c)"),
        ("a * b \\ c", "((a * b) \\ c)"),
        ("a \\ b * c", "(a \\ (b * c))"),
        ("a \\ b Mod c", "((a \\ b) Mod c)"),
        ("a Mod b \\ c", "(a Mod (b \\ c))"),
        ("a + b Mod c", "(a + (b Mod c))"),
        ("a Mod b + c", "((a Mod b) + c)"),
        ("a & b + c", "(a & (b + c))"),
        ("a + b & c", "((a + b) & c)"),
        ("a & b = c", "((a & b) = c)"),
        ("a = b & c", "(a = (b & c))"),
        // exponent: left associative, tighter than unary minus
        ("a ^ b ^ c", "((a ^ b) ^ c)"),
        ("-a ^ b", "(- (a ^ b))"),
        ("a ^ -b", "(a ^ (- b))"),
        ("-a * b", "((- a) * b)"),
        ("- -a", "(- (- a))"),
        ("a * -b ^ c", "(a * (- (b ^ c)))"),
        ("2 ^ 3 * 4", "((2 ^ 3) * 4)"),
        ("a - -b", "(a - (- b))"),
        // comparison
        ("a < b + c", "(a < (b + c))"),
        ("a = b = c", "((a = b) = c)"),
        ("a <> b", "(a <> b)"),
        ("a <= b", "(a <= b)"),
        ("a Like b & c", "(a Like (b & c))"),
        ("a Is b", "(a Is b)"),
        ("a Is b And c Is d", "((a Is b) And (c Is d))"),
        // logical
        ("a And b Or c", "((a And b) Or c)"),
        ("a Or b And c", "(a Or (b And c))"),
        ("a Or b Xor c", "((a Or b) Xor c)"),
        ("a Xor b Or c", "(a Xor (b Or c))"),
        ("a Xor b Eqv c", "((a Xor b) Eqv c)"),
        ("a Eqv b Imp c", "((a Eqv b) Imp c)"),
        ("a Imp b Eqv c", "(a Imp (b Eqv c))"),
        ("a Imp b Imp c", "((a Imp b) Imp c)"),
        ("a And b And c", "((a And b) And c)"),
        // Not
        ("Not a", "(Not a)"),
        ("Not a = b", "(Not (a = b))"),
        ("Not a And b", "((Not a) And b)"),
        ("a And Not b", "(a And (Not b))"),
        ("Not Not a", "(Not (Not a))"),
        ("Not a Or b", "((Not a) Or b)"),
        ("Not a + b", "(Not (a + b))"),
        ("a = Not b", "(a = (Not b))"),
        ("-a = b", "((- a) = b)"),
        // grouping and postfix
        ("(a + b) * c", "([(a + b)] * c)"),
        ("a(1) + b(2)", "(a(1) + b(2))"),
        ("a.b + c.d", "(a.b + c.d)"),
        ("a(1).b(2)", "a(1).b(2)"),
        (".a + .b", "(.a + .b)"),
        ("-a.b", "(- a.b)"),
        ("Foo(a + b, c * d)", "Foo((a + b),(c * d))"),
    ];
    for (src, want) in cases {
        assert_eq!(&bin(src), want, "for {src}");
    }
}

#[test]
fn call_vs_index_share_one_node() {
    // The syntax cannot tell `a(1)` the call from `a(1)` the index.
    assert_eq!(
        expr("a(1)"),
        "(CallExpr (NameExpr a) (ArgList ( (Arg (LiteralExpr 1)) )))"
    );
    assert_eq!(
        expr("a(1)(2)"),
        "(CallExpr (CallExpr (NameExpr a) (ArgList ( (Arg (LiteralExpr 1)) ))) (ArgList ( (Arg (LiteralExpr 2)) )))"
    );
}

#[test]
fn print_hash_is_a_file_number() {
    assert_eq!(
        stmts("Print #1, x"),
        "(PrintStmt Print (FileNumberExpr # (LiteralExpr 1)) , (OutputList (NameExpr x)))"
    );
    assert_eq!(
        stmts("Print #n, a; b"),
        "(PrintStmt Print (FileNumberExpr # (NameExpr n)) , (OutputList (NameExpr a) ; (NameExpr b)))"
    );
}

#[test]
fn comparison_vs_assignment() {
    assert_eq!(
        stmts("x = y = z"),
        "(AssignStmt (NameExpr x) = (BinaryExpr (NameExpr y) = (NameExpr z)))"
    );
}

#[test]
fn single_line_if_else_binds_to_nearest_if() {
    assert_eq!(
        stmts("If a Then b = 1 Else c = 2"),
        "(IfLineStmt If (NameExpr a) Then (Block (AssignStmt (NameExpr b) = (LiteralExpr 1))) Else (Block (AssignStmt (NameExpr c) = (LiteralExpr 2))))"
    );
    // The dangling Else belongs to the inner If.
    assert_eq!(
        stmts("If a Then If b Then c = 1 Else d = 2"),
        "(IfLineStmt If (NameExpr a) Then (Block (IfLineStmt If (NameExpr b) Then (Block (AssignStmt (NameExpr c) = (LiteralExpr 1))) Else (Block (AssignStmt (NameExpr d) = (LiteralExpr 2))))))"
    );
    // Colon-separated statements stay in the Then/Else part.
    assert_eq!(
        stmts("If a Then b = 1: c = 2 Else d = 3"),
        "(IfLineStmt If (NameExpr a) Then (Block (AssignStmt (NameExpr b) = (LiteralExpr 1)) : (AssignStmt (NameExpr c) = (LiteralExpr 2))) Else (Block (AssignStmt (NameExpr d) = (LiteralExpr 3))))"
    );
}

#[test]
fn comment_after_then_makes_a_block_if() {
    let s = stmts("If a Then ' note\nb = 1\nEnd If");
    assert!(s.starts_with("(IfStmt "), "{s}");
}

#[test]
fn else_if_is_nested_unlike_elseif() {
    // `ElseIf` is a clause of the same If.
    let a = stmts("If a Then\nx = 1\nElseIf b Then\nx = 2\nEnd If");
    assert!(a.contains("(ElseIfClause "), "{a}");
    assert_eq!(a.matches("IfStmt").count(), 1, "{a}");
    // `Else If` opens a second, nested block If that needs its own End If.
    let src = "Sub T()\nIf a Then\nx = 1\nElse If b Then\nx = 2\nEnd If\nEnd If\nEnd Sub\n";
    let p = parse_str(src);
    assert!(!p.has_errors(), "{:?}", diag_lines(&p));
    let b = sexp(&p);
    assert!(b.contains("(ElseClause Else (Block (IfStmt "), "{b}");
    assert_eq!(b.matches("(IfStmt").count(), 2, "{b}");
}

#[test]
fn dim_with_bounds() {
    assert_eq!(
        stmts("Dim a(1 To 3) As Integer"),
        "(VarDeclStmt Dim (VarDeclarator a (ArgList ( (Arg (RangeExpr (LiteralExpr 1) To (LiteralExpr 3))) )) (AsClause As (TypeRef Integer))))"
    );
}

#[test]
fn redim_preserve() {
    assert_eq!(
        stmts("ReDim Preserve a(1 To n) As Long"),
        "(ReDimStmt ReDim Preserve (ReDimItem (CallExpr (NameExpr a) (ArgList ( (Arg (RangeExpr (LiteralExpr 1) To (NameExpr n))) ))) (AsClause As (TypeRef Long))))"
    );
}

#[test]
fn const_statement() {
    assert_eq!(
        stmts("Const A As Long = 1, B = 2"),
        "(ConstStmt Const (ConstItem A (AsClause As (TypeRef Long)) = (LiteralExpr 1)) , (ConstItem B = (LiteralExpr 2)))"
    );
}

#[test]
fn labels_versus_colons() {
    // `lbl:` at line start is a label; `a: b` later on is a separator.
    assert_eq!(
        stmts("lbl:\nx = 1"),
        "(LabelStmt lbl :) (AssignStmt (NameExpr x) = (LiteralExpr 1))"
    );
    assert_eq!(
        stmts("x = 1: y = 2"),
        "(AssignStmt (NameExpr x) = (LiteralExpr 1)) (AssignStmt (NameExpr y) = (LiteralExpr 2))"
    );
    assert_eq!(
        stmts("10 x = 1"),
        "(LabelStmt 10) (AssignStmt (NameExpr x) = (LiteralExpr 1))"
    );
    // `Foo:=1` is a named argument, not a label.
    assert_eq!(
        stmts("Foo a:=1"),
        "(CallStmt (NameExpr Foo) (ArgList (Arg a := (LiteralExpr 1))))"
    );
}

#[test]
fn implicit_call_with_parenthesised_first_argument() {
    assert_eq!(
        stmts("Foo (a), b"),
        "(CallStmt (NameExpr Foo) (ArgList (Arg (ParenExpr ( (NameExpr a) ))) , (Arg (NameExpr b))))"
    );
    assert_eq!(
        stmts("Foo(a)"),
        "(CallStmt (CallExpr (NameExpr Foo) (ArgList ( (Arg (NameExpr a)) ))))"
    );
}

#[test]
fn bang_and_dot_access() {
    assert_eq!(expr("rs!Name"), "(DictExpr (NameExpr rs) ! Name)");
    assert_eq!(
        expr("a.b!c"),
        "(DictExpr (MemberExpr (NameExpr a) . b) ! c)"
    );
    assert_eq!(expr(".x"), "(MemberExpr . x)");
    assert_eq!(expr("!x"), "(DictExpr ! x)");
}

#[test]
fn named_arguments() {
    assert_eq!(
        stmts("Foo a:=1, b:=2"),
        "(CallStmt (NameExpr Foo) (ArgList (Arg a := (LiteralExpr 1)) , (Arg b := (LiteralExpr 2))))"
    );
    assert_eq!(
        expr("F(x:=1)"),
        "(CallExpr (NameExpr F) (ArgList ( (Arg x := (LiteralExpr 1)) )))"
    );
}

#[test]
fn skipped_arguments() {
    assert_eq!(
        expr("F(, 2)"),
        "(CallExpr (NameExpr F) (ArgList ( (Arg) , (Arg (LiteralExpr 2)) )))"
    );
    assert_eq!(
        expr("F(1, , 3)"),
        "(CallExpr (NameExpr F) (ArgList ( (Arg (LiteralExpr 1)) , (Arg) , (Arg (LiteralExpr 3)) )))"
    );
}

#[test]
fn new_addressof_typeof() {
    assert_eq!(expr("New Foo.Bar"), "(NewExpr New (TypeRef Foo . Bar))");
    assert_eq!(
        expr("AddressOf Foo"),
        "(AddressOfExpr AddressOf (NameExpr Foo))"
    );
    assert_eq!(
        expr("TypeOf x Is Foo"),
        "(TypeOfIsExpr TypeOf (NameExpr x) Is (TypeRef Foo))"
    );
    assert_eq!(
        expr("TypeOf x Is Foo And y"),
        "(BinaryExpr (TypeOfIsExpr TypeOf (NameExpr x) Is (TypeRef Foo)) And (NameExpr y))"
    );
}

#[test]
fn for_next_with_multiple_counters_closes_both_loops() {
    let s = stmts("For i = 1 To 2\nFor j = 1 To 2\nNext j, i");
    assert_eq!(s.matches("ForStmt").count(), 2, "{s}");
}

#[test]
fn select_case_items() {
    let s = stmts("Select Case x\nCase 1, 2 To 3, Is >= 4\nCase Else\nEnd Select");
    assert!(s.contains("(CaseClause Case (LiteralExpr 1) , (RangeExpr (LiteralExpr 2) To (LiteralExpr 3)) , (CaseIsItem Is >= (LiteralExpr 4))"), "{s}");
    assert!(s.contains("(CaseClause Case Else"), "{s}");
}
