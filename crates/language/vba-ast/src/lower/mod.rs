//! CST to AST lowering.
//!
//! [`lower`] accepts any tree produced by `vba-syntax`, including trees
//! with error nodes, and never panics. Error nodes become
//! [`Item::Error`] / [`StmtKind::Error`] / [`ExprKind::Error`] plus an
//! `AST001` diagnostic; constructs missing a mandatory part are completed
//! with placeholders and reported as `AST002`.

mod decl;
mod expr;
mod stmt;

use vba_diagnostics::{Code, Diagnostic};
use vba_lexer::{Keyword, TokenKind, TypeSuffix};
use vba_source::{FileId, Span};
use vba_syntax::{NodeKind, SyntaxNode, SyntaxToken, SyntaxTree};

use crate::ast::*;
use crate::name::{Ident, IdentKind, Name};

/// An unparsed region (CST error node) was lowered to an error node.
pub const AST_ERROR_NODE: Code = Code("AST001");
/// A construct lacks a mandatory part and was completed with a placeholder.
pub const AST_INCOMPLETE: Code = Code("AST002");
/// A literal could not be converted.
pub const AST_BAD_LITERAL: Code = Code("AST003");

/// The result of [`lower`].
#[derive(Debug, Clone, PartialEq)]
pub struct Lowered {
    /// The module.
    pub module: Module,
    /// Diagnostics for error nodes and incomplete constructs.
    pub diagnostics: Vec<Diagnostic>,
}

/// Lowers a syntax tree to a [`Module`].
pub fn lower(tree: &SyntaxTree, file: FileId) -> Lowered {
    let mut l = Lowerer {
        file,
        diags: Vec::new(),
        source: tree.source(),
    };
    let module = l.module(tree.root());
    for n in tree.root().descendants() {
        match n.kind() {
            NodeKind::Error => {
                l.report(AST_ERROR_NODE, n.span(), "source that could not be parsed")
            }
            NodeKind::ErrorExpr => {
                l.report(AST_ERROR_NODE, n.span(), "missing or malformed expression")
            }
            _ => {}
        }
    }
    Lowered {
        module,
        diagnostics: l.diags,
    }
}

pub(crate) struct Lowerer<'t> {
    pub(crate) file: FileId,
    pub(crate) diags: Vec<Diagnostic>,
    pub(crate) source: &'t [u8],
}

/// Significant (non-trivia) tokens that are direct children of `n`.
pub(crate) fn toks<'t>(n: SyntaxNode<'t>) -> Vec<SyntaxToken<'t>> {
    n.significant_tokens().collect()
}

/// Child nodes of `n` of the given kind.
pub(crate) fn nodes_of<'t>(n: SyntaxNode<'t>, kind: NodeKind) -> Vec<SyntaxNode<'t>> {
    n.child_nodes().filter(|c| c.kind() == kind).collect()
}

pub(crate) fn is_expr_kind(k: NodeKind) -> bool {
    matches!(
        k,
        NodeKind::LiteralExpr
            | NodeKind::NameExpr
            | NodeKind::ParenExpr
            | NodeKind::UnaryExpr
            | NodeKind::BinaryExpr
            | NodeKind::MemberExpr
            | NodeKind::DictExpr
            | NodeKind::CallExpr
            | NodeKind::NewExpr
            | NodeKind::AddressOfExpr
            | NodeKind::TypeOfIsExpr
            | NodeKind::FileNumberExpr
            | NodeKind::RangeExpr
            | NodeKind::ErrorExpr
    )
}

/// Child nodes of `n` that are expressions.
pub(crate) fn expr_nodes<'t>(n: SyntaxNode<'t>) -> Vec<SyntaxNode<'t>> {
    n.child_nodes().filter(|c| is_expr_kind(c.kind())).collect()
}

pub(crate) fn is_kw(t: &SyntaxToken<'_>, k: Keyword) -> bool {
    t.kind() == TokenKind::Keyword(k)
}

/// Builds an identifier from a name-like token.
pub(crate) fn ident(t: SyntaxToken<'_>) -> Ident {
    let text = t.text();
    let span = t.span();
    match t.kind() {
        TokenKind::ForeignName => {
            let inner = if text.len() >= 2 && text[text.len() - 1] == b']' {
                &text[1..text.len() - 1]
            } else {
                &text[1.min(text.len())..]
            };
            Ident {
                name: Name::new(inner),
                suffix: None,
                kind: IdentKind::Foreign,
                span,
            }
        }
        TokenKind::Keyword(k) => Ident {
            name: Name::new(text),
            suffix: None,
            kind: IdentKind::Keyword(k),
            span,
        },
        _ => {
            let (body, suffix) = match text.split_last() {
                Some((&last, rest)) if !rest.is_empty() => match TypeSuffix::from_byte(last) {
                    Some(s) => (rest, Some(s)),
                    None => (text, None),
                },
                _ => (text, None),
            };
            Ident {
                name: Name::new(body),
                suffix,
                kind: IdentKind::Plain,
                span,
            }
        }
    }
}

pub(crate) fn is_name_token(k: TokenKind) -> bool {
    matches!(k, TokenKind::Ident | TokenKind::ForeignName)
}

pub(crate) fn visibility_of(t: &SyntaxToken<'_>) -> Option<Visibility> {
    match t.kind() {
        TokenKind::Keyword(Keyword::Public) => Some(Visibility::Public),
        TokenKind::Keyword(Keyword::Private) => Some(Visibility::Private),
        TokenKind::Keyword(Keyword::Friend) => Some(Visibility::Friend),
        TokenKind::Keyword(Keyword::Global) => Some(Visibility::Global),
        _ => None,
    }
}

impl Lowerer<'_> {
    pub(crate) fn report(&mut self, code: Code, span: Span, msg: impl Into<String>) {
        self.diags
            .push(Diagnostic::error(code, self.file, span, msg));
    }

    fn module(&mut self, root: SyntaxNode<'_>) -> Module {
        let mut header: Option<ModuleHeader> = None;
        let mut items = Vec::new();
        for c in root.child_nodes() {
            match c.kind() {
                NodeKind::VersionLine => {
                    header.get_or_insert_with(ModuleHeader::default).version = Some(c.span());
                }
                NodeKind::HeaderBlock => {
                    let h = header.get_or_insert_with(ModuleHeader::default);
                    h.block = Some(c.span());
                    h.properties = self.header_properties(c);
                }
                _ => {
                    if let Some(item) = self.item(c) {
                        items.push(item);
                    }
                }
            }
        }
        Module {
            span: root.span(),
            header,
            items,
        }
    }

    /// `name = value` lines directly inside the outermost `BEGIN` block.
    fn header_properties(&mut self, block: SyntaxNode<'_>) -> Vec<HeaderProperty> {
        let mut out = Vec::new();
        let mut depth = 0i32;
        let mut line: Vec<SyntaxToken<'_>> = Vec::new();
        let tokens: Vec<_> = block.child_tokens().collect();
        let mut flush = |line: &mut Vec<SyntaxToken<'_>>, depth: &mut i32, source: &[u8]| {
            let first = line.first().copied();
            if let Some(f) = first {
                if f.kind() == TokenKind::Ident && f.text_eq("begin") {
                    *depth += 1;
                } else if is_kw(&f, Keyword::End) && line.len() == 1 {
                    *depth -= 1;
                } else if *depth == 1
                    && f.kind() == TokenKind::Ident
                    && line.get(1).is_some_and(|t| t.kind() == TokenKind::Eq)
                    && line.len() > 2
                {
                    let vstart = line[2].span().start as usize;
                    let vend = line[line.len() - 1].span().end as usize;
                    out.push(HeaderProperty {
                        name: Name::new(f.text()),
                        value: source[vstart..vend].to_vec(),
                        span: Span::new(f.span().start, vend as u32),
                    });
                }
            }
            line.clear();
        };
        for t in tokens {
            if t.kind() == TokenKind::Newline {
                flush(&mut line, &mut depth, self.source);
            } else if !t.kind().is_trivia() {
                line.push(t);
            }
        }
        flush(&mut line, &mut depth, self.source);
        out
    }

    pub(crate) fn directive(&mut self, n: SyntaxNode<'_>) -> Option<Directive> {
        let kind = match n.kind() {
            NodeKind::CcIf => DirectiveKind::If,
            NodeKind::CcElseIf => DirectiveKind::ElseIf,
            NodeKind::CcElse => DirectiveKind::Else,
            NodeKind::CcEndIf => DirectiveKind::EndIf,
            NodeKind::CcConst => DirectiveKind::Const,
            _ => return None,
        };
        Some(Directive {
            kind,
            span: n.span(),
            value: n.value(),
        })
    }
}
