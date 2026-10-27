//! Expressions (MS-VBAL §5.6): a Pratt parser over the precedence table in
//! `crate::prec`, plus postfix member/call/dictionary access.

use vba_lexer::{Keyword, KeywordClass, TokenKind};

use super::{EXPECTED, Parser};
use crate::NodeKind;
use crate::prec::{NOT_OPERAND_BP, UNARY_OPERAND_BP, binary_bp};

/// Keywords that are also names usable as the head of an expression
/// (built-in functions, `Me`, `Debug`, literals, ...).
pub(crate) fn kw_primary(k: Keyword) -> bool {
    matches!(
        k.class(),
        KeywordClass::Literal | KeywordClass::ReservedName | KeywordClass::SpecialForm
    ) || matches!(
        k,
        Keyword::Input | Keyword::Seek | Keyword::Spc | Keyword::Tab
    )
}

/// Whether a token can start an expression.
pub(crate) fn starts_expr(k: TokenKind) -> bool {
    use TokenKind as T;
    match k {
        T::Ident
        | T::ForeignName
        | T::IntegerLiteral
        | T::HexLiteral
        | T::OctalLiteral
        | T::FloatLiteral
        | T::StringLiteral
        | T::DateLiteral
        | T::LParen
        | T::Minus
        | T::Plus
        | T::Dot
        | T::Bang
        | T::Hash => true,
        T::Keyword(k) => {
            kw_primary(k)
                || matches!(
                    k,
                    Keyword::Not | Keyword::New | Keyword::AddressOf | Keyword::TypeOf
                )
        }
        _ => false,
    }
}

impl Parser<'_> {
    pub(crate) fn at_expr_start(&self) -> bool {
        self.cur().is_some_and(starts_expr)
    }

    /// Parses an expression whose binary operators bind at least as tightly
    /// as `min` (0 parses everything).
    pub(crate) fn parse_expr(&mut self, min: u8) {
        if self.depth >= super::MAX_DEPTH {
            self.too_deep();
            return;
        }
        self.depth += 1;
        self.parse_expr_inner(min);
        self.depth -= 1;
    }

    fn parse_expr_inner(&mut self, min: u8) {
        let cp = self.cp();
        self.parse_prefix();
        let mut chain = 0u32;
        while let Some(k) = self.cur() {
            let Some(bp) = binary_bp(k) else { break };
            if bp < min {
                break;
            }
            chain += 1;
            if chain > super::MAX_CHAIN {
                self.chain_too_long();
                break;
            }
            self.start_at(cp, NodeKind::BinaryExpr);
            self.bump();
            self.parse_expr(bp + 1);
            self.finish();
        }
    }

    fn parse_prefix(&mut self) {
        match self.cur() {
            Some(TokenKind::Minus | TokenKind::Plus) => {
                self.start(NodeKind::UnaryExpr);
                self.bump();
                self.parse_expr(UNARY_OPERAND_BP);
                self.finish();
            }
            Some(TokenKind::Keyword(Keyword::Not)) => {
                self.start(NodeKind::UnaryExpr);
                self.bump();
                self.parse_expr(NOT_OPERAND_BP);
                self.finish();
            }
            Some(TokenKind::Keyword(Keyword::AddressOf)) => {
                self.start(NodeKind::AddressOfExpr);
                self.bump();
                self.parse_postfix(true);
                self.finish();
            }
            Some(TokenKind::Keyword(Keyword::TypeOf)) => {
                self.start(NodeKind::TypeOfIsExpr);
                self.bump();
                // Tighter than `Is` so the operand stops before it.
                self.parse_expr(8);
                if self.expect_kw(Keyword::Is, "`Is`") {
                    self.parse_type_ref();
                }
                self.finish();
            }
            Some(TokenKind::Keyword(Keyword::New)) => {
                self.start(NodeKind::NewExpr);
                self.bump();
                self.parse_type_ref();
                self.finish();
            }
            _ => self.parse_postfix(true),
        }
    }

    /// A primary expression followed by `.name`, `!name` and `(args)`.
    /// With `spaced_paren` false a `(` preceded by whitespace is not a call
    /// (statement-level `Foo (a), b` is an implicit call whose first
    /// argument is parenthesised) and a `.name` preceded by whitespace is an
    /// argument (`MsgBox .Name`, `Err.Raise .Number`), not a member.
    pub(crate) fn parse_postfix(&mut self, spaced_paren: bool) {
        let cp = self.cp();
        self.parse_primary();
        let mut chain = 0u32;
        loop {
            if matches!(
                self.cur(),
                Some(TokenKind::Dot | TokenKind::Bang | TokenKind::LParen)
            ) {
                chain += 1;
                if chain > super::MAX_CHAIN {
                    self.chain_too_long();
                    break;
                }
            }
            match self.cur() {
                Some(TokenKind::Dot) if !spaced_paren && self.at_spaced_with_member() => {
                    // Statement level: `MsgBox .Name` passes `.Name`.
                    break;
                }
                Some(TokenKind::Dot) => {
                    self.start_at(cp, NodeKind::MemberExpr);
                    self.bump();
                    self.member_name();
                    self.finish();
                }
                Some(TokenKind::Bang) => {
                    self.start_at(cp, NodeKind::DictExpr);
                    self.bump();
                    self.member_name();
                    self.finish();
                }
                Some(TokenKind::LParen) => {
                    if !spaced_paren && self.space_before() {
                        break;
                    }
                    self.start_at(cp, NodeKind::CallExpr);
                    self.parse_arg_list_paren();
                    self.finish();
                }
                _ => break,
            }
        }
    }

    fn member_name(&mut self) {
        if matches!(
            self.cur(),
            Some(TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_))
        ) {
            self.bump();
        } else {
            let d = self.describe_cur();
            self.error(EXPECTED, format!("expected a member name, found {d}"));
        }
    }

    fn parse_primary(&mut self) {
        use TokenKind as T;
        match self.cur() {
            Some(
                T::IntegerLiteral
                | T::HexLiteral
                | T::OctalLiteral
                | T::FloatLiteral
                | T::StringLiteral
                | T::DateLiteral
                | T::Keyword(
                    Keyword::True
                    | Keyword::False
                    | Keyword::Nothing
                    | Keyword::Empty
                    | Keyword::Null,
                ),
            ) => {
                self.start(NodeKind::LiteralExpr);
                self.bump();
                self.finish();
            }
            Some(T::Ident | T::ForeignName) => {
                self.start(NodeKind::NameExpr);
                self.bump();
                self.finish();
            }
            Some(T::Keyword(k)) if kw_primary(k) => {
                self.start(NodeKind::NameExpr);
                self.bump();
                self.finish();
            }
            Some(T::LParen) => {
                self.start(NodeKind::ParenExpr);
                self.bump();
                self.parse_expr(0);
                self.expect(T::RParen, "`)`");
                self.finish();
            }
            Some(T::Dot) => {
                self.start(NodeKind::MemberExpr);
                self.bump();
                self.member_name();
                self.finish();
            }
            Some(T::Bang) => {
                self.start(NodeKind::DictExpr);
                self.bump();
                self.member_name();
                self.finish();
            }
            Some(T::Hash) => {
                self.start(NodeKind::FileNumberExpr);
                self.bump();
                self.parse_postfix(true);
                self.finish();
            }
            Some(k) if k.is_error() => {
                // The lexer already reported it.
                self.start(NodeKind::ErrorExpr);
                self.bump();
                self.finish();
            }
            _ => {
                let d = self.describe_cur();
                self.error(EXPECTED, format!("expected an expression, found {d}"));
                self.start(NodeKind::ErrorExpr);
                self.finish();
            }
        }
    }

    /// `( [Arg] {, [Arg]} )`.
    pub(crate) fn parse_arg_list_paren(&mut self) {
        self.start(NodeKind::ArgList);
        self.bump();
        if !self.at(TokenKind::RParen) {
            loop {
                self.parse_arg();
                if self.at(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "`)`");
        self.finish();
    }

    /// Arguments without parentheses: `[Arg] {, [Arg]}`.
    pub(crate) fn parse_arg_list_bare(&mut self) {
        self.start(NodeKind::ArgList);
        loop {
            self.parse_arg();
            if self.at(TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.finish();
    }

    /// One argument slot: `[name :=] [expr [To expr]]`; may be empty.
    pub(crate) fn parse_arg(&mut self) {
        self.start(NodeKind::Arg);
        if matches!(
            self.cur(),
            Some(TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_))
        ) && self.kind(1) == Some(TokenKind::ColonEq)
        {
            self.bump();
            self.bump();
        }
        if self.at_expr_start() {
            self.parse_range_or_expr();
        }
        self.finish();
    }

    /// `expr` or `expr To expr`.
    pub(crate) fn parse_range_or_expr(&mut self) {
        let cp = self.cp();
        self.parse_expr(0);
        if self.at_kw(Keyword::To) {
            self.start_at(cp, NodeKind::RangeExpr);
            self.bump();
            self.parse_expr(0);
            self.finish();
        }
    }
}
