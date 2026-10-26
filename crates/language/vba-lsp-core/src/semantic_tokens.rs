//! Semantic tokens.
//!
//! Classification is lexical plus the syntactic scope index: a name gets a
//! class (function, variable, type, ...) only when it resolves to a
//! declaration in this file or to a standard-library function; unresolved
//! names are left to the client's grammar.

use std::collections::HashMap;

use vba_lexer::{Keyword, KeywordClass, TokenKind};
use vba_source::Span;
use vba_syntax::NodeKind;

use crate::scope::SymKind;
use crate::text::Range;
use crate::types::{SemanticToken, token_modifier as m, token_type as t};
use crate::workspace::View;

impl View<'_> {
    /// Semantic tokens for the whole document, sorted, one line per token.
    pub fn semantic_tokens(&self) -> Vec<SemanticToken> {
        let scopes = self.scopes();
        let occ: HashMap<u32, (u32, bool)> = scopes
            .occurrences
            .iter()
            .map(|o| (o.span.start, (o.symbol, o.is_decl)))
            .collect();
        let tokens = self.tokens();
        let mut out = Vec::new();
        for (i, tok) in tokens.iter().enumerate() {
            let parent = tok.parent().kind();
            let kind = tok.kind();
            let class: Option<(u32, u32)> = if parent == NodeKind::InactiveText {
                (!matches!(
                    kind,
                    TokenKind::Whitespace | TokenKind::Newline | TokenKind::LineContinuation
                ))
                .then_some((t::COMMENT, m::INACTIVE))
            } else {
                match kind {
                    TokenKind::Comment | TokenKind::RemComment => Some((t::COMMENT, 0)),
                    TokenKind::Keyword(k) => Some(keyword_class(k, parent)),
                    TokenKind::Ident | TokenKind::ForeignName => {
                        let prev_is_dot = tokens[..i]
                            .iter()
                            .rev()
                            .find(|p| !p.kind().is_trivia())
                            .is_some_and(|p| p.kind() == TokenKind::Dot);
                        self.ident_class(tok, parent, prev_is_dot, &occ)
                    }
                    k if k.is_number() => Some((t::NUMBER, 0)),
                    TokenKind::StringLiteral
                    | TokenKind::UnterminatedString
                    | TokenKind::DateLiteral => Some((t::STRING, 0)),
                    TokenKind::Plus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Backslash
                    | TokenKind::Caret
                    | TokenKind::Amp
                    | TokenKind::Eq
                    | TokenKind::Lt
                    | TokenKind::Gt
                    | TokenKind::Le
                    | TokenKind::Ge
                    | TokenKind::Ne
                    | TokenKind::ColonEq => Some((t::OPERATOR, 0)),
                    TokenKind::HashIf
                    | TokenKind::HashElseIf
                    | TokenKind::HashElse
                    | TokenKind::HashEndIf
                    | TokenKind::HashConst => Some((t::MACRO, 0)),
                    _ => None,
                }
            };
            if let Some((ty, mods)) = class {
                self.push_split(tok.span(), ty, mods, &mut out);
            }
        }
        out
    }

    /// Semantic tokens that intersect `range`.
    pub fn semantic_tokens_range(&self, range: Range) -> Vec<SemanticToken> {
        self.semantic_tokens()
            .into_iter()
            .filter(|s| {
                let start = crate::text::Position::new(s.line, s.start);
                let end = crate::text::Position::new(s.line, s.start + s.length);
                end > range.start && start < range.end
            })
            .collect()
    }

    fn ident_class(
        &self,
        tok: &vba_syntax::SyntaxToken<'_>,
        parent: NodeKind,
        prev_is_dot: bool,
        occ: &HashMap<u32, (u32, bool)>,
    ) -> Option<(u32, u32)> {
        let scopes = self.scopes();
        if let Some(&(sym, is_decl)) = occ.get(&tok.span().start) {
            let decl = if is_decl { m::DECLARATION } else { 0 };
            return Some(match scopes.symbols[sym as usize].kind {
                SymKind::Proc(_) | SymKind::Declare | SymKind::Event => (t::FUNCTION, decl),
                SymKind::Variable => (t::VARIABLE, decl),
                SymKind::Const => (t::VARIABLE, decl | m::READONLY),
                SymKind::Param => (t::PARAMETER, decl),
                SymKind::Type | SymKind::Enum => (t::TYPE, decl),
                SymKind::EnumMember => (t::ENUM_MEMBER, decl | m::READONLY),
                SymKind::Label => (t::LABEL, decl),
            });
        }
        match parent {
            NodeKind::ProcDecl
            | NodeKind::DeclareStmt
            | NodeKind::OptionStmt
            | NodeKind::ForStmt
            | NodeKind::OpenStmt
            | NodeKind::ExitStmt => return Some((t::KEYWORD, 0)),
            NodeKind::TypeMember => return Some((t::PROPERTY, m::DECLARATION)),
            NodeKind::Param => return Some((t::PARAMETER, m::DECLARATION)),
            NodeKind::TypeRef => return Some((t::TYPE, 0)),
            _ => {}
        }
        if prev_is_dot {
            return Some((t::PROPERTY, 0));
        }
        if vba_stdlib::lookup(&String::from_utf8_lossy(tok.text())).is_some() {
            return Some((t::FUNCTION, m::DEFAULT_LIBRARY));
        }
        None
    }

    fn push_split(&self, span: Span, ty: u32, mods: u32, out: &mut Vec<SemanticToken>) {
        let map = self.document().text_map();
        let bytes = self.bytes();
        let a = self.position(span.start);
        let b = self.position(span.end);
        for line in a.line..=b.line {
            let (ls, le) = map.line_content(bytes, line).unwrap_or((0, 0));
            let from = if line == a.line { a.character } else { 0 };
            let to = if line == b.line {
                b.character
            } else {
                self.position(le).character
            };
            let _ = ls;
            if to > from {
                out.push(SemanticToken {
                    line,
                    start: from,
                    length: to - from,
                    token_type: ty,
                    modifiers: mods,
                });
            }
        }
    }
}

fn keyword_class(k: Keyword, parent: NodeKind) -> (u32, u32) {
    match k.class() {
        KeywordClass::ReservedType => (t::TYPE, m::DEFAULT_LIBRARY),
        KeywordClass::Operator
            if matches!(
                k,
                Keyword::And
                    | Keyword::Eqv
                    | Keyword::Imp
                    | Keyword::Is
                    | Keyword::Like
                    | Keyword::Mod
                    | Keyword::Not
                    | Keyword::Or
                    | Keyword::Xor
            ) =>
        {
            (t::OPERATOR, 0)
        }
        KeywordClass::ReservedName | KeywordClass::SpecialForm => match (k, parent) {
            (Keyword::String | Keyword::Date, NodeKind::TypeRef) => (t::TYPE, m::DEFAULT_LIBRARY),
            (Keyword::Me | Keyword::Debug, _) => (t::KEYWORD, 0),
            (_, NodeKind::NameExpr | NodeKind::CallExpr) => (t::FUNCTION, m::DEFAULT_LIBRARY),
            _ => (t::KEYWORD, 0),
        },
        _ => (t::KEYWORD, 0),
    }
}

/// Encodes absolute tokens in the relative wire format: five integers per
/// token (delta line, delta start, length, type, modifiers).
pub fn encode_semantic_tokens(tokens: &[SemanticToken]) -> Vec<u32> {
    let mut out = Vec::with_capacity(tokens.len() * 5);
    let (mut pl, mut ps) = (0, 0);
    for s in tokens {
        let dl = s.line - pl;
        let ds = if dl == 0 { s.start - ps } else { s.start };
        out.extend([dl, ds, s.length, s.token_type, s.modifiers]);
        pl = s.line;
        ps = s.start;
    }
    out
}
