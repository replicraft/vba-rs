//! The formatter: re-indents block structure and nothing else.
//!
//! Only the leading whitespace of a line may change. Token text, blank
//! lines, trailing whitespace, line continuations (the lines after a
//! continuation keep their indentation), line numbers, inactive
//! conditional-compilation text and exported-module headers are left
//! exactly as written. The indentation of a line is the nesting depth of
//! its first token in the syntax tree, so formatting is idempotent.

use vba_lexer::TokenKind;
use vba_syntax::NodeKind;

use crate::text::{Position, Range};
use crate::types::{FormatOptions, TextEdit};
use crate::workspace::View;

enum LineKind {
    Skip,
    Code(u32),
    Comment,
}

struct Line {
    kind: LineKind,
    /// Byte range of the existing leading whitespace.
    lead: (u32, u32),
}

impl View<'_> {
    /// Edits that re-indent the whole document.
    pub fn format(&self, opts: FormatOptions) -> Vec<TextEdit> {
        self.format_lines(0, u32::MAX, opts)
    }

    /// Edits that re-indent the lines touched by `range`.
    pub fn format_range(&self, range: Range, opts: FormatOptions) -> Vec<TextEdit> {
        let (a, b) = if range.start <= range.end {
            (range.start, range.end)
        } else {
            (range.end, range.start)
        };
        // A range ending at column 0 does not include that line.
        let last = if b.character == 0 && b.line > a.line {
            b.line - 1
        } else {
            b.line
        };
        self.format_lines(a.line, last, opts)
    }

    fn format_lines(&self, first: u32, last: u32, opts: FormatOptions) -> Vec<TextEdit> {
        let tokens = self.tokens();
        let map = self.document().text_map();
        let bytes = self.bytes();
        let mut lines: Vec<Line> = Vec::new();
        for l in 0..map.line_count() {
            let (ls, _) = map.line_content(bytes, l).unwrap_or((0, 0));
            let skip = Line {
                kind: LineKind::Skip,
                lead: (ls, ls),
            };
            let k = tokens.partition_point(|t| t.span().start < ls);
            let Some(t0) = tokens.get(k).filter(|t| t.span().start == ls) else {
                lines.push(skip);
                continue;
            };
            if k > 0 && tokens[k - 1].kind() == TokenKind::LineContinuation {
                lines.push(skip);
                continue;
            }
            let (lead, next) = if t0.kind() == TokenKind::Whitespace {
                ((ls, t0.span().end), tokens.get(k + 1))
            } else {
                ((ls, ls), Some(t0))
            };
            let Some(next) = next else {
                lines.push(Line {
                    kind: LineKind::Skip,
                    lead,
                });
                continue;
            };
            let parent = next.parent();
            let kind = match next.kind() {
                // Leading blanks can change how an error token lexes (a
                // lone `_` after blanks is a line continuation).
                k if k.is_error() => LineKind::Skip,
                TokenKind::Newline | TokenKind::LineContinuation => LineKind::Skip,
                _ if matches!(
                    parent.kind(),
                    NodeKind::InactiveText | NodeKind::HeaderBlock | NodeKind::VersionLine
                ) =>
                {
                    LineKind::Skip
                }
                TokenKind::Comment | TokenKind::RemComment => LineKind::Comment,
                TokenKind::HashIf
                | TokenKind::HashElseIf
                | TokenKind::HashElse
                | TokenKind::HashEndIf
                | TokenKind::HashConst => LineKind::Code(0),
                _ if parent.kind() == NodeKind::LabelStmt => {
                    if next.kind() == TokenKind::Ident {
                        LineKind::Code(0)
                    } else {
                        LineKind::Skip
                    }
                }
                _ => {
                    let mut depth = 0;
                    let mut member = false;
                    let mut cur = Some(parent);
                    while let Some(n) = cur {
                        match n.kind() {
                            NodeKind::Block | NodeKind::CaseClause => depth += 1,
                            NodeKind::TypeMember | NodeKind::EnumMember => member = true,
                            _ => {}
                        }
                        cur = n.parent();
                    }
                    LineKind::Code(depth + u32::from(member))
                }
            };
            lines.push(Line { kind, lead });
        }

        // Comment-only lines take the depth of the next code line, unless
        // that line closes a block, in which case they stay with the body.
        let code_depth: Vec<Option<u32>> = lines
            .iter()
            .map(|l| match l.kind {
                LineKind::Code(d) => Some(d),
                _ => None,
            })
            .collect();
        let mut want: Vec<Option<u32>> = code_depth.clone();
        for (i, l) in lines.iter().enumerate() {
            if !matches!(l.kind, LineKind::Comment) {
                continue;
            }
            let prev = code_depth[..i].iter().rev().find_map(|d| *d);
            let next = code_depth[i + 1..].iter().find_map(|d| *d);
            want[i] = match (prev, next) {
                (Some(p), Some(n)) => Some(if n >= p { n } else { p }),
                (Some(d), None) | (None, Some(d)) => Some(d),
                (None, None) => Some(0),
            };
        }

        let mut out = Vec::new();
        for (i, l) in lines.iter().enumerate() {
            let line = i as u32;
            if line < first || line > last {
                continue;
            }
            let Some(depth) = want[i] else { continue };
            let wanted = if opts.insert_spaces {
                " ".repeat((depth * opts.tab_size) as usize)
            } else {
                "\t".repeat(depth as usize)
            };
            let have = &bytes[l.lead.0 as usize..l.lead.1 as usize];
            if have == wanted.as_bytes() {
                continue;
            }
            out.push(TextEdit {
                range: Range::new(
                    Position::new(line, 0),
                    Position::new(line, l.lead.1 - l.lead.0),
                ),
                new_text: wanted,
            });
        }
        out
    }
}

/// Applies edits (non-overlapping, any order) to a string. Used by tests and
/// by callers that want the formatted text.
pub fn apply_edits(text: &str, edits: &[TextEdit]) -> String {
    let mut ws = crate::Workspace::new();
    ws.open("mem:apply", text, 0);
    let doc = ws.document("mem:apply").expect("just opened");
    let mut spans: Vec<(vba_source::Span, &str)> = edits
        .iter()
        .map(|e| {
            (
                doc.text_map().span(doc.bytes(), e.range),
                e.new_text.as_str(),
            )
        })
        .collect();
    spans.sort_by_key(|(s, _)| std::cmp::Reverse(s.start));
    let mut bytes = doc.bytes().to_vec();
    for (s, t) in spans {
        bytes.splice(s.start as usize..s.end as usize, t.bytes());
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
