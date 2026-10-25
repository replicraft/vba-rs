//! Snapshot tests of tree shape (run with `UPDATE_EXPECT=1` to refresh).

mod common;

use common::*;
use expect_test::{Expect, expect};

fn check(src: &str, expect: Expect) {
    let p = parse_str(src);
    let mut out = p.tree.debug_dump();
    for l in diag_lines(&p) {
        out.push_str("DIAG ");
        out.push_str(&l);
        out.push('\n');
    }
    expect.assert_eq(&out);
}

#[test]
fn small_procedure() {
    check(
        "Sub A(ByVal x As Long)\n    x = x + 1 ' inc\nEnd Sub\n",
        expect![[r#"
            Module@0..51
              ProcDecl@0..50
                Keyword(Sub)@0..3 "Sub"
                Whitespace@3..4 " "
                Ident@4..5 "A"
                ParamList@5..22
                  LParen@5..6 "("
                  Param@6..21
                    Keyword(ByVal)@6..11 "ByVal"
                    Whitespace@11..12 " "
                    Ident@12..13 "x"
                    Whitespace@13..14 " "
                    AsClause@14..21
                      Keyword(As)@14..16 "As"
                      Whitespace@16..17 " "
                      TypeRef@17..21
                        Keyword(Long)@17..21 "Long"
                  RParen@21..22 ")"
                Block@22..43
                  Newline@22..23 "\n"
                  Whitespace@23..27 "    "
                  AssignStmt@27..36
                    NameExpr@27..28
                      Ident@27..28 "x"
                    Whitespace@28..29 " "
                    Eq@29..30 "="
                    Whitespace@30..31 " "
                    BinaryExpr@31..36
                      NameExpr@31..32
                        Ident@31..32 "x"
                      Whitespace@32..33 " "
                      Plus@33..34 "+"
                      Whitespace@34..35 " "
                      LiteralExpr@35..36
                        IntegerLiteral@35..36 "1"
                  Whitespace@36..37 " "
                  Comment@37..42 "' inc"
                  Newline@42..43 "\n"
                Keyword(End)@43..46 "End"
                Whitespace@46..47 " "
                Keyword(Sub)@47..50 "Sub"
              Newline@50..51 "\n"
        "#]],
    );
}

#[test]
fn single_line_if() {
    check(
        "Sub A()\nIf a Then b = 1 Else c = 2\nEnd Sub",
        expect![[r#"
        Module@0..42
          ProcDecl@0..42
            Keyword(Sub)@0..3 "Sub"
            Whitespace@3..4 " "
            Ident@4..5 "A"
            ParamList@5..7
              LParen@5..6 "("
              RParen@6..7 ")"
            Block@7..35
              Newline@7..8 "\n"
              IfLineStmt@8..34
                Keyword(If)@8..10 "If"
                Whitespace@10..11 " "
                NameExpr@11..12
                  Ident@11..12 "a"
                Whitespace@12..13 " "
                Keyword(Then)@13..17 "Then"
                Whitespace@17..18 " "
                Block@18..23
                  AssignStmt@18..23
                    NameExpr@18..19
                      Ident@18..19 "b"
                    Whitespace@19..20 " "
                    Eq@20..21 "="
                    Whitespace@21..22 " "
                    LiteralExpr@22..23
                      IntegerLiteral@22..23 "1"
                Whitespace@23..24 " "
                Keyword(Else)@24..28 "Else"
                Whitespace@28..29 " "
                Block@29..34
                  AssignStmt@29..34
                    NameExpr@29..30
                      Ident@29..30 "c"
                    Whitespace@30..31 " "
                    Eq@31..32 "="
                    Whitespace@32..33 " "
                    LiteralExpr@33..34
                      IntegerLiteral@33..34 "2"
              Newline@34..35 "\n"
            Keyword(End)@35..38 "End"
            Whitespace@38..39 " "
            Keyword(Sub)@39..42 "Sub"
    "#]],
    );
}

#[test]
fn recovery_keeps_following_statement() {
    check(
        "Sub A()\nx = )\ny = 2\nEnd Sub",
        expect![[r#"
        Module@0..27
          ProcDecl@0..27
            Keyword(Sub)@0..3 "Sub"
            Whitespace@3..4 " "
            Ident@4..5 "A"
            ParamList@5..7
              LParen@5..6 "("
              RParen@6..7 ")"
            Block@7..20
              Newline@7..8 "\n"
              AssignStmt@8..12
                NameExpr@8..9
                  Ident@8..9 "x"
                Whitespace@9..10 " "
                Eq@10..11 "="
                Whitespace@11..12 " "
                ErrorExpr@12..12
              Error@12..13
                RParen@12..13 ")"
              Newline@13..14 "\n"
              AssignStmt@14..19
                NameExpr@14..15
                  Ident@14..15 "y"
                Whitespace@15..16 " "
                Eq@16..17 "="
                Whitespace@17..18 " "
                LiteralExpr@18..19
                  IntegerLiteral@18..19 "2"
              Newline@19..20 "\n"
            Keyword(End)@20..23 "End"
            Whitespace@23..24 " "
            Keyword(Sub)@24..27 "Sub"
        DIAG SYN002 error: expected an expression, found `)`
    "#]],
    );
}

#[test]
fn conditional_compilation() {
    check(
        "#If Win64 Then\nDim a\n#Else\nDim b ' skipped\n#End If\n",
        expect![[r##"
            Module@0..51
              CcIf@0..14 value=true
                HashIf@0..3 "#If"
                Whitespace@3..4 " "
                Ident@4..9 "Win64"
                Whitespace@9..10 " "
                Keyword(Then)@10..14 "Then"
              Newline@14..15 "\n"
              VarDeclStmt@15..20
                Keyword(Dim)@15..18 "Dim"
                Whitespace@18..19 " "
                VarDeclarator@19..20
                  Ident@19..20 "a"
              Newline@20..21 "\n"
              CcElse@21..26
                HashElse@21..26 "#Else"
              Newline@26..27 "\n"
              InactiveText@27..43
                Keyword(Dim)@27..30 "Dim"
                Whitespace@30..31 " "
                Ident@31..32 "b"
                Whitespace@32..33 " "
                Comment@33..42 "' skipped"
                Newline@42..43 "\n"
              CcEndIf@43..50
                HashEndIf@43..50 "#End If"
              Newline@50..51 "\n"
        "##]],
    );
}

#[test]
fn exported_class_header() {
    check(
        "VERSION 1.0 CLASS\nBEGIN\n  MultiUse = -1\nEND\nAttribute VB_Name = \"C\"\n",
        expect![[r#"
            Module@0..68
              VersionLine@0..17
                Ident@0..7 "VERSION"
                Whitespace@7..8 " "
                FloatLiteral@8..11 "1.0"
                Whitespace@11..12 " "
                Ident@12..17 "CLASS"
              Newline@17..18 "\n"
              HeaderBlock@18..43
                Ident@18..23 "BEGIN"
                Newline@23..24 "\n"
                Whitespace@24..26 "  "
                Ident@26..34 "MultiUse"
                Whitespace@34..35 " "
                Eq@35..36 "="
                Whitespace@36..37 " "
                Minus@37..38 "-"
                IntegerLiteral@38..39 "1"
                Newline@39..40 "\n"
                Keyword(End)@40..43 "END"
              Newline@43..44 "\n"
              AttributeStmt@44..67
                Keyword(Attribute)@44..53 "Attribute"
                Whitespace@53..54 " "
                Keyword(VbAttribute)@54..61 "VB_Name"
                Whitespace@61..62 " "
                Eq@62..63 "="
                Whitespace@63..64 " "
                LiteralExpr@64..67
                  StringLiteral@64..67 "\"C\""
              Newline@67..68 "\n"
        "#]],
    );
}
