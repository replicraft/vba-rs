//! Selection ranges (expand/shrink selection).

use crate::text::Position;
use crate::types::SelectionRange;
use crate::workspace::{View, token_index_at};

impl View<'_> {
    /// For each position the chain of enclosing ranges, innermost first:
    /// the token, its enclosing syntax nodes (expression, statement, block,
    /// procedure, ...) and finally the whole document. Ranges strictly grow.
    pub fn selection_ranges(&self, positions: &[Position]) -> Vec<SelectionRange> {
        let tokens = self.tokens();
        let len = self.bytes().len() as u32;
        positions
            .iter()
            .map(|&p| {
                let off = self.offset(p);
                let mut spans = Vec::new();
                if let Some(i) = token_index_at(&tokens, off) {
                    let t = tokens[i];
                    if !t.kind().is_trivia() && t.kind() != vba_lexer::TokenKind::Newline {
                        spans.push(t.span());
                    }
                    let mut n = Some(t.parent());
                    while let Some(x) = n {
                        spans.push(x.span());
                        n = x.parent();
                    }
                }
                spans.push(vba_source::Span::new(0, len));
                let mut chain: Vec<crate::text::Range> = Vec::new();
                let mut last: Option<vba_source::Span> = None;
                for s in spans {
                    // Keep only spans that contain the offset and grow.
                    if s.start > off || s.end < off {
                        continue;
                    }
                    if last.is_some_and(|l| s.start >= l.start && s.end <= l.end) {
                        continue;
                    }
                    last = Some(s);
                    chain.push(self.range(s));
                }
                SelectionRange { chain }
            })
            .collect()
    }
}
