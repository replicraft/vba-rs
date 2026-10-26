//! Expression lowering (MS-VBAL §5.6).

use vba_lexer::{Keyword, TokenKind, TypeSuffix};
use vba_source::Span;
use vba_syntax::{NodeKind, SyntaxNode, SyntaxToken};

use super::{
    AST_BAD_LITERAL, AST_INCOMPLETE, Lowerer, expr_nodes, ident, is_expr_kind, is_kw, toks,
};
use crate::ast::*;
use crate::name::{Ident, IdentKind, Name};

fn err_expr(span: Span) -> Expr {
    Expr {
        span,
        kind: ExprKind::Error,
    }
}

fn binary_op(k: TokenKind) -> Option<BinaryOp> {
    Some(match k {
        TokenKind::Plus => BinaryOp::Add,
        TokenKind::Minus => BinaryOp::Sub,
        TokenKind::Star => BinaryOp::Mul,
        TokenKind::Slash => BinaryOp::Div,
        TokenKind::Backslash => BinaryOp::IntDiv,
        TokenKind::Caret => BinaryOp::Pow,
        TokenKind::Amp => BinaryOp::Concat,
        TokenKind::Eq => BinaryOp::Eq,
        TokenKind::Ne => BinaryOp::Ne,
        TokenKind::Lt => BinaryOp::Lt,
        TokenKind::Gt => BinaryOp::Gt,
        TokenKind::Le => BinaryOp::Le,
        TokenKind::Ge => BinaryOp::Ge,
        TokenKind::Keyword(Keyword::Mod) => BinaryOp::Mod,
        TokenKind::Keyword(Keyword::Like) => BinaryOp::Like,
        TokenKind::Keyword(Keyword::Is) => BinaryOp::Is,
        TokenKind::Keyword(Keyword::And) => BinaryOp::And,
        TokenKind::Keyword(Keyword::Or) => BinaryOp::Or,
        TokenKind::Keyword(Keyword::Xor) => BinaryOp::Xor,
        TokenKind::Keyword(Keyword::Eqv) => BinaryOp::Eqv,
        TokenKind::Keyword(Keyword::Imp) => BinaryOp::Imp,
        _ => return None,
    })
}

/// Maps a comparison token (as used after `Case Is`).
pub(crate) fn comparison_op(k: TokenKind) -> Option<BinaryOp> {
    match binary_op(k)? {
        op @ (BinaryOp::Eq
        | BinaryOp::Ne
        | BinaryOp::Lt
        | BinaryOp::Gt
        | BinaryOp::Le
        | BinaryOp::Ge) => Some(op),
        _ => None,
    }
}

fn split_suffix(text: &[u8], allowed: &[u8]) -> (usize, Option<TypeSuffix>) {
    match text.last() {
        Some(b) if allowed.contains(b) && text.len() > 1 => {
            (text.len() - 1, TypeSuffix::from_byte(*b))
        }
        _ => (text.len(), None),
    }
}

impl Lowerer<'_> {
    /// Lowers the `nth` expression child of `n`; a missing child becomes
    /// [`ExprKind::Error`].
    pub(crate) fn expr_child(&mut self, n: SyntaxNode<'_>, nth: usize) -> Expr {
        match expr_nodes(n).get(nth) {
            Some(c) => self.expr(*c),
            None => {
                let at = Span::empty(n.span().end);
                self.report(AST_INCOMPLETE, n.span(), "expected an expression");
                err_expr(at)
            }
        }
    }

    pub(crate) fn expr_opt(&mut self, n: SyntaxNode<'_>, nth: usize) -> Option<Expr> {
        expr_nodes(n).get(nth).map(|c| self.expr(*c))
    }

    pub(crate) fn expr(&mut self, n: SyntaxNode<'_>) -> Expr {
        let span = n.span();
        let kind = match n.kind() {
            NodeKind::LiteralExpr => match toks(n).first() {
                Some(t) => self.literal(*t),
                None => ExprKind::Error,
            },
            NodeKind::NameExpr => match toks(n).first() {
                Some(t) => ExprKind::Name(ident(*t)),
                None => ExprKind::Error,
            },
            NodeKind::ParenExpr => ExprKind::Paren(Box::new(self.expr_child(n, 0))),
            NodeKind::UnaryExpr => {
                let op = match toks(n).first().map(|t| t.kind()) {
                    Some(TokenKind::Minus) => UnaryOp::Neg,
                    Some(TokenKind::Plus) => UnaryOp::Plus,
                    _ => UnaryOp::Not,
                };
                ExprKind::Unary {
                    op,
                    operand: Box::new(self.expr_child(n, 0)),
                }
            }
            NodeKind::BinaryExpr => {
                let op = toks(n)
                    .iter()
                    .find_map(|t| binary_op(t.kind()))
                    .unwrap_or(BinaryOp::Add);
                ExprKind::Binary {
                    op,
                    lhs: Box::new(self.expr_child(n, 0)),
                    rhs: Box::new(self.expr_child(n, 1)),
                }
            }
            NodeKind::MemberExpr | NodeKind::DictExpr => {
                let target = self.expr_opt(n, 0).map(Box::new);
                let name = match toks(n).last() {
                    Some(t) if !matches!(t.kind(), TokenKind::Dot | TokenKind::Bang) => ident(*t),
                    _ => {
                        self.report(AST_INCOMPLETE, span, "expected a member name");
                        Ident::missing(Span::empty(span.end))
                    }
                };
                if n.kind() == NodeKind::MemberExpr {
                    ExprKind::Member { target, name }
                } else {
                    ExprKind::Dict { target, name }
                }
            }
            NodeKind::CallExpr => {
                let callee = Box::new(self.expr_child(n, 0));
                let args = match n.child_node(NodeKind::ArgList) {
                    Some(l) => self.args(l),
                    None => Vec::new(),
                };
                ExprKind::Call { callee, args }
            }
            NodeKind::NewExpr => ExprKind::New(self.type_ref_child(n)),
            NodeKind::AddressOfExpr => ExprKind::AddressOf(Box::new(self.expr_child(n, 0))),
            NodeKind::TypeOfIsExpr => ExprKind::TypeOfIs {
                expr: Box::new(self.expr_child(n, 0)),
                ty: self.type_ref_child(n),
            },
            NodeKind::FileNumberExpr => ExprKind::FileNumber(Box::new(self.expr_child(n, 0))),
            // `To end` without a start stands for `1 To end` (MS-VBAL 5.4.5.4).
            NodeKind::RangeExpr if expr_nodes(n).len() == 1 => ExprKind::Range {
                lower: Box::new(Expr {
                    span: Span::empty(span.start),
                    kind: ExprKind::Literal(Literal::Integer {
                        value: Some(1),
                        radix: Radix::Decimal,
                        suffix: None,
                    }),
                }),
                upper: Box::new(self.expr_child(n, 0)),
            },
            NodeKind::RangeExpr => ExprKind::Range {
                lower: Box::new(self.expr_child(n, 0)),
                upper: Box::new(self.expr_child(n, 1)),
            },
            NodeKind::ErrorExpr => ExprKind::Error,
            _ => {
                self.report(AST_INCOMPLETE, span, "expected an expression");
                ExprKind::Error
            }
        };
        Expr { span, kind }
    }

    /// Arguments of an `ArgList` node.
    pub(crate) fn args(&mut self, list: SyntaxNode<'_>) -> Vec<Arg> {
        list.child_nodes()
            .filter(|a| a.kind() == NodeKind::Arg)
            .map(|a| {
                let t = toks(a);
                let name = match (t.first(), t.get(1)) {
                    (Some(n), Some(c)) if c.kind() == TokenKind::ColonEq => Some(ident(*n)),
                    _ => None,
                };
                let value = a
                    .child_nodes()
                    .find(|c| is_expr_kind(c.kind()))
                    .map(|c| self.expr(c));
                Arg {
                    span: a.span(),
                    name,
                    value,
                }
            })
            .collect()
    }

    fn literal(&mut self, t: SyntaxToken<'_>) -> ExprKind {
        let text = t.text();
        match t.kind() {
            TokenKind::IntegerLiteral => {
                let (end, suffix) = split_suffix(text, b"%&^");
                let value = std::str::from_utf8(&text[..end])
                    .ok()
                    .and_then(|s| s.parse().ok());
                ExprKind::Literal(Literal::Integer {
                    value,
                    radix: Radix::Decimal,
                    suffix,
                })
            }
            TokenKind::HexLiteral | TokenKind::OctalLiteral => {
                let hex = t.kind() == TokenKind::HexLiteral;
                let (end, suffix) = split_suffix(text, b"%&^");
                let body = &text[..end];
                // `&H` / `&O` / `&` prefix.
                let digits = match body.get(1) {
                    Some(b'h' | b'H' | b'o' | b'O') => &body[2..],
                    _ => &body[1.min(body.len())..],
                };
                let radix = if hex { 16 } else { 8 };
                let value = std::str::from_utf8(digits)
                    .ok()
                    .and_then(|s| u64::from_str_radix(s, radix).ok());
                ExprKind::Literal(Literal::Integer {
                    value,
                    radix: if hex { Radix::Hex } else { Radix::Octal },
                    suffix,
                })
            }
            TokenKind::FloatLiteral => {
                let (end, suffix) = split_suffix(text, b"!#@");
                let s: String = String::from_utf8_lossy(&text[..end])
                    .chars()
                    .map(|c| if c == 'd' || c == 'D' { 'e' } else { c })
                    .collect();
                let value = match s.parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        self.report(AST_BAD_LITERAL, t.span(), "invalid floating point literal");
                        0.0
                    }
                };
                ExprKind::Literal(Literal::Float {
                    value,
                    text: s,
                    suffix,
                })
            }
            TokenKind::StringLiteral => {
                let inner = if text.len() >= 2 && text[text.len() - 1] == b'"' {
                    &text[1..text.len() - 1]
                } else {
                    &text[1.min(text.len())..]
                };
                let mut out = Vec::with_capacity(inner.len());
                let mut i = 0;
                while i < inner.len() {
                    out.push(inner[i]);
                    if inner[i] == b'"' && inner.get(i + 1) == Some(&b'"') {
                        i += 1;
                    }
                    i += 1;
                }
                ExprKind::Literal(Literal::String(out))
            }
            TokenKind::DateLiteral => {
                let inner = if text.len() >= 2 {
                    &text[1..text.len() - 1]
                } else {
                    text
                };
                ExprKind::Literal(Literal::Date(inner.to_vec()))
            }
            TokenKind::Keyword(Keyword::True) => ExprKind::Literal(Literal::True),
            TokenKind::Keyword(Keyword::False) => ExprKind::Literal(Literal::False),
            TokenKind::Keyword(Keyword::Nothing) => ExprKind::Literal(Literal::Nothing),
            TokenKind::Keyword(Keyword::Empty) => ExprKind::Literal(Literal::Empty),
            TokenKind::Keyword(Keyword::Null) => ExprKind::Literal(Literal::Null),
            _ => {
                self.report(AST_BAD_LITERAL, t.span(), "unexpected literal token");
                // A keyword used as a name keeps its spelling.
                ExprKind::Name(Ident {
                    name: Name::new(text),
                    suffix: None,
                    kind: IdentKind::Plain,
                    span: t.span(),
                })
            }
        }
    }

    // ---- types ----------------------------------------------------------

    pub(crate) fn type_ref_child(&mut self, n: SyntaxNode<'_>) -> TypeRef {
        match n.child_node(NodeKind::TypeRef) {
            Some(t) => self.type_ref(t),
            None => {
                self.report(AST_INCOMPLETE, n.span(), "expected a type name");
                TypeRef {
                    span: Span::empty(n.span().end),
                    kind: TypeRefKind::Missing,
                }
            }
        }
    }

    pub(crate) fn type_ref(&mut self, n: SyntaxNode<'_>) -> TypeRef {
        let t = toks(n);
        let span = n.span();
        let kind = match t.first() {
            None => {
                self.report(AST_INCOMPLETE, span, "expected a type name");
                TypeRefKind::Missing
            }
            Some(first) => match first.kind() {
                TokenKind::Keyword(k) => match builtin_type(k) {
                    Some(b) => TypeRefKind::Builtin(b),
                    None => TypeRefKind::Named(vec![ident(*first)]),
                },
                _ => TypeRefKind::Named(
                    t.iter()
                        .filter(|x| x.kind() != TokenKind::Dot)
                        .map(|x| ident(*x))
                        .collect(),
                ),
            },
        };
        TypeRef { span, kind }
    }

    pub(crate) fn as_clause(&mut self, n: SyntaxNode<'_>) -> AsClause {
        AsClause {
            span: n.span(),
            is_new: toks(n).iter().any(|t| is_kw(t, Keyword::New)),
            ty: self.type_ref_child(n),
            fixed_length: self.expr_opt(n, 0),
            is_array: toks(n).iter().any(|t| t.kind() == TokenKind::LParen),
        }
    }

    pub(crate) fn as_clause_opt(&mut self, n: SyntaxNode<'_>) -> Option<AsClause> {
        n.child_node(NodeKind::AsClause).map(|c| self.as_clause(c))
    }

    /// Array bounds from an `ArgList` of `Arg`s.
    pub(crate) fn bounds(&mut self, list: SyntaxNode<'_>) -> Vec<Bound> {
        let mut out = Vec::new();
        for a in list.child_nodes().filter(|a| a.kind() == NodeKind::Arg) {
            let span = a.span();
            let value = a.child_nodes().find(|c| is_expr_kind(c.kind()));
            let (lower, upper) = match value {
                Some(v) if v.kind() == NodeKind::RangeExpr => {
                    let l = self.expr_child(v, 0);
                    let u = self.expr_child(v, 1);
                    (Some(l), u)
                }
                Some(v) => (None, self.expr(v)),
                None => {
                    self.report(AST_INCOMPLETE, span, "expected an array bound");
                    (None, err_expr(span))
                }
            };
            out.push(Bound { span, lower, upper });
        }
        out
    }
}

fn builtin_type(k: Keyword) -> Option<BuiltinType> {
    Some(match k {
        Keyword::Boolean => BuiltinType::Boolean,
        Keyword::Byte => BuiltinType::Byte,
        Keyword::Currency => BuiltinType::Currency,
        Keyword::Date => BuiltinType::Date,
        Keyword::Double => BuiltinType::Double,
        Keyword::Integer => BuiltinType::Integer,
        Keyword::Long => BuiltinType::Long,
        Keyword::LongLong => BuiltinType::LongLong,
        Keyword::LongPtr => BuiltinType::LongPtr,
        Keyword::Single => BuiltinType::Single,
        Keyword::String => BuiltinType::String,
        Keyword::Variant => BuiltinType::Variant,
        Keyword::Any => BuiltinType::Any,
        _ => return None,
    })
}
