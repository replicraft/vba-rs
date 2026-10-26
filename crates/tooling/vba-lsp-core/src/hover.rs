//! Hover for in-file declarations, keywords and standard-library functions.

use vba_lexer::TokenKind;

use crate::builtin::{keyword_doc, std_doc, std_signature};
use crate::text::Position;
use crate::types::Hover;
use crate::util::qualified_name;
use crate::workspace::{View, token_index_at};

impl View<'_> {
    /// Hover text for the token under the cursor: the declaration of an
    /// in-file name, a description of a keyword, or the signature of a
    /// library function. `None` over whitespace, punctuation and names that
    /// do not resolve.
    pub fn hover(&self, pos: Position) -> Option<Hover> {
        let off = self.offset(pos);
        let tokens = self.tokens();
        let i = token_index_at(&tokens, off)?;
        let tok = tokens[i];
        let range = self.range(tok.span());
        let fence = |code: &str, doc: Option<String>| {
            let mut s = format!("```vba\n{code}\n```");
            if let Some(d) = doc {
                s.push_str("\n\n");
                s.push_str(&d);
            }
            Some(Hover { contents: s, range })
        };

        if let Some(h) = self.semantic_hover(&tokens, i, range) {
            return Some(h);
        }

        if let Some(o) = self.scopes().occurrence_at(tok.span().start) {
            let sym = &self.scopes().symbols[o.symbol as usize];
            if o.span.start == tok.span().start {
                let mut detail = sym.detail.clone();
                if let Some(c) = &sym.container {
                    detail = format!("{detail}  ({c})");
                }
                return fence(&detail, None);
            }
        }

        match tok.kind() {
            TokenKind::Keyword(k) => {
                let word = String::from_utf8_lossy(tok.text()).into_owned();
                let in_type = tok.parent().kind() == vba_syntax::NodeKind::TypeRef;
                if let Some(info) = self.std_function(&tokens, i).filter(|_| !in_type) {
                    let sig = std_signature(info);
                    return fence(&sig.label, Some(std_doc(info)));
                }
                Some(Hover {
                    contents: keyword_doc(&word, k),
                    range,
                })
            }
            TokenKind::Ident => {
                let info = self.std_function(&tokens, i)?;
                let sig = std_signature(info);
                fence(&sig.label, Some(std_doc(info)))
            }
            _ => None,
        }
    }

    /// The library function a word refers to, if it is not an in-file name,
    /// not a member of something else and is called like a function.
    pub(crate) fn std_function(
        &self,
        tokens: &[vba_syntax::SyntaxToken<'_>],
        i: usize,
    ) -> Option<&'static vba_stdlib::FunctionInfo> {
        let (name, leading_dot) = qualified_name(tokens, i);
        if leading_dot {
            return None;
        }
        if !name.contains('.')
            && self
                .scopes()
                .lookup_at(
                    tokens[i].span().start,
                    &crate::util::normalize(name.as_bytes()),
                )
                .is_some()
        {
            return None;
        }
        vba_stdlib::lookup(&name).map(|r| r.info)
    }
}
