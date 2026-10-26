//! Folding ranges.

use vba_lexer::TokenKind;
use vba_syntax::NodeKind;

use crate::types::{FoldingKind, FoldingRange};
use crate::workspace::View;

impl View<'_> {
    /// Folding ranges for procedures, types, enums, block statements,
    /// `Case` clauses, conditional-compilation branches and runs of two or
    /// more comment lines.
    pub fn folding_ranges(&self) -> Vec<FoldingRange> {
        let mut out = Vec::new();
        let region = |s: u32, e: u32, out: &mut Vec<FoldingRange>| {
            if e > s {
                out.push(FoldingRange {
                    start_line: s,
                    end_line: e,
                    kind: FoldingKind::Region,
                });
            }
        };
        let root = self.tree().root();
        let mut cc_open: Vec<u32> = Vec::new();
        for n in root.descendants() {
            let line_of = |off: u32| self.position(off).line;
            // A node whose span ends with a line terminator ends at column
            // 0 of the next line; fold up to the last line with content.
            let end_line_of = |sp: vba_source::Span| {
                let p = self.position(sp.end);
                let start = line_of(sp.start);
                if p.character == 0 && p.line > start {
                    p.line - 1
                } else {
                    p.line
                }
            };
            match n.kind() {
                NodeKind::ProcDecl
                | NodeKind::TypeStmt
                | NodeKind::EnumStmt
                | NodeKind::IfStmt
                | NodeKind::SelectStmt
                | NodeKind::CaseClause
                | NodeKind::ForStmt
                | NodeKind::ForEachStmt
                | NodeKind::DoStmt
                | NodeKind::WhileStmt
                | NodeKind::WithStmt => {
                    let sp = n.span();
                    region(line_of(sp.start), end_line_of(sp), &mut out);
                }
                NodeKind::CcIf => cc_open.push(line_of(n.span().start)),
                NodeKind::CcElseIf | NodeKind::CcElse | NodeKind::CcEndIf => {
                    let here = line_of(n.span().start);
                    if let Some(start) = cc_open.pop() {
                        region(start, here.saturating_sub(1), &mut out);
                    }
                    if n.kind() != NodeKind::CcEndIf {
                        cc_open.push(here);
                    }
                }
                _ => {}
            }
        }
        out.extend(self.comment_runs());
        out.sort_by_key(|r| (r.start_line, r.end_line, r.kind == FoldingKind::Comment));
        out.dedup();
        out
    }

    fn comment_runs(&self) -> Vec<FoldingRange> {
        let tokens = self.tokens();
        let mut lines: Vec<u32> = Vec::new();
        for (i, t) in tokens.iter().enumerate() {
            if !matches!(t.kind(), TokenKind::Comment | TokenKind::RemComment) {
                continue;
            }
            let mut j = i;
            if j > 0 && tokens[j - 1].kind() == TokenKind::Whitespace {
                j -= 1;
            }
            let at_line_start = j == 0 || tokens[j - 1].kind() == TokenKind::Newline;
            if at_line_start {
                let l = self.position(t.span().start).line;
                if lines.last() != Some(&l) {
                    lines.push(l);
                }
            }
        }
        let mut out = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            let mut j = i;
            while j + 1 < lines.len() && lines[j + 1] == lines[j] + 1 {
                j += 1;
            }
            if j > i {
                out.push(FoldingRange {
                    start_line: lines[i],
                    end_line: lines[j],
                    kind: FoldingKind::Comment,
                });
            }
            i = j + 1;
        }
        out
    }
}
