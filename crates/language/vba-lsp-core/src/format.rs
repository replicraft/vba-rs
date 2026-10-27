//! The formatter: re-indents block structure and, when asked, normalizes
//! the case of reserved words ([`FormatOptions::keyword_case`]).
//!
//! Only the leading whitespace of a line and, with that option, the letter
//! case of reserved words may change. Token text, blank
//! lines, trailing whitespace, line continuations (the lines after a
//! continuation keep their indentation), line numbers, inactive
//! conditional-compilation text and exported-module headers are left
//! exactly as written. The indentation of a line is the nesting depth of
//! its first token in the syntax tree, so formatting is idempotent.

use vba_lexer::{KeywordClass, TokenKind};
use vba_syntax::{NodeKind, SyntaxToken};

use crate::text::{Position, Range};
use crate::types::{FormatError, FormatLimits, FormatOptions, TextEdit};
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
    /// Edits that re-indent the whole document. No budget is applied; use
    /// [`try_format`](View::try_format) for source that is not trusted.
    pub fn format(&self, opts: FormatOptions) -> Vec<TextEdit> {
        self.format_lines(0, u32::MAX, opts, None)
            .unwrap_or_default()
    }

    /// [`format`](View::format) within budgets: refused, before any work,
    /// when the document or the indentation would exceed `limits`.
    pub fn try_format(
        &self,
        opts: FormatOptions,
        limits: &FormatLimits,
    ) -> Result<Vec<TextEdit>, FormatError> {
        self.check_size(limits)?;
        self.format_lines(0, u32::MAX, opts, limits.max_indent_width)
    }

    /// [`format_range`](View::format_range) within budgets, see
    /// [`try_format`](View::try_format).
    pub fn try_format_range(
        &self,
        range: Range,
        opts: FormatOptions,
        limits: &FormatLimits,
    ) -> Result<Vec<TextEdit>, FormatError> {
        self.check_size(limits)?;
        let (first, last) = Self::range_lines(range);
        self.format_lines(first, last, opts, limits.max_indent_width)
    }

    fn check_size(&self, limits: &FormatLimits) -> Result<(), FormatError> {
        if let Some(limit) = limits.max_source_bytes {
            let actual = self.bytes().len() as u64;
            if actual > limit {
                return Err(FormatError::TooLarge { limit, actual });
            }
        }
        Ok(())
    }

    /// Edits that re-indent the lines touched by `range`.
    pub fn format_range(&self, range: Range, opts: FormatOptions) -> Vec<TextEdit> {
        let (first, last) = Self::range_lines(range);
        self.format_lines(first, last, opts, None)
            .unwrap_or_default()
    }

    /// The first and last line a range touches.
    fn range_lines(range: Range) -> (u32, u32) {
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
        (a.line, last)
    }

    fn format_lines(
        &self,
        first: u32,
        last: u32,
        opts: FormatOptions,
        max_indent_width: Option<u64>,
    ) -> Result<Vec<TextEdit>, FormatError> {
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
                // Indentation made of anything but blanks and tabs (a Unicode
                // space) is left as written.
                let (s0, e0) = (t0.span().start as usize, t0.span().end as usize);
                if bytes
                    .get(s0..e0)
                    .is_some_and(|w| w.iter().any(|b| !matches!(b, b' ' | b'\t')))
                {
                    lines.push(skip);
                    continue;
                }
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
        // The depth of the nearest code line before and after each line, in
        // two linear passes (a scan per comment line is quadratic in a file
        // of comments).
        let mut prev_code: Vec<Option<u32>> = Vec::with_capacity(lines.len());
        let mut seen = None;
        for d in &code_depth {
            prev_code.push(seen);
            seen = d.or(seen);
        }
        let mut next_code: Vec<Option<u32>> = vec![None; lines.len()];
        let mut following = None;
        for (i, d) in code_depth.iter().enumerate().rev() {
            next_code[i] = following;
            following = d.or(following);
        }
        let mut want: Vec<Option<u32>> = code_depth.clone();
        for (i, l) in lines.iter().enumerate() {
            if !matches!(l.kind, LineKind::Comment) {
                continue;
            }
            let (prev, next) = (prev_code[i], next_code[i]);
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
            if let Some(limit) = max_indent_width {
                let unit = if opts.insert_spaces {
                    u64::from(opts.tab_size)
                } else {
                    1
                };
                let width = unit.saturating_mul(u64::from(depth));
                if width > limit {
                    return Err(FormatError::IndentTooWide {
                        limit,
                        wanted: width,
                    });
                }
            }
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
        if opts.keyword_case {
            self.keyword_edits(&tokens, first, last, &mut out);
            out.sort_by_key(|e| (e.range.start.line, e.range.start.character));
        }
        Ok(out)
    }

    /// Edits that spell reserved words canonically. An edit that starts where
    /// the indent edit of its line ends is merged into it so that edits
    /// never touch.
    fn keyword_edits(
        &self,
        tokens: &[SyntaxToken<'_>],
        first: u32,
        last: u32,
        out: &mut Vec<TextEdit>,
    ) {
        // Edits by line and end column, to find the one that ends where a
        // keyword starts without scanning all of them.
        let mut ends: std::collections::HashMap<(u32, u32), usize> = out
            .iter()
            .enumerate()
            .map(|(i, e)| ((e.range.start.line, e.range.end.character), i))
            .collect();
        for (k, t) in tokens.iter().enumerate() {
            let TokenKind::Keyword(kw) = t.kind() else {
                continue;
            };
            if matches!(
                kw.class(),
                KeywordClass::ImplementationUse | KeywordClass::FutureReserved
            ) {
                continue;
            }
            let canonical = kw.canonical();
            if t.text() == canonical.as_bytes()
                || !t.text().eq_ignore_ascii_case(canonical.as_bytes())
            {
                continue;
            }
            if matches!(
                t.parent().kind(),
                NodeKind::InactiveText | NodeKind::HeaderBlock | NodeKind::VersionLine
            ) {
                continue;
            }
            let prev = tokens[..k].iter().rev().find(|p| {
                !matches!(
                    p.kind(),
                    TokenKind::Whitespace | TokenKind::LineContinuation
                )
            });
            if prev.is_some_and(|p| matches!(p.kind(), TokenKind::Dot | TokenKind::Bang)) {
                continue;
            }
            let range = self.range(t.span());
            if range.start.line < first || range.start.line > last {
                continue;
            }
            let merge = ends
                .remove(&(range.start.line, range.start.character))
                .filter(|&i| out[i].range.end == range.start);
            match merge {
                Some(i) => {
                    let e = &mut out[i];
                    e.range.end = range.end;
                    e.new_text.push_str(canonical);
                    ends.insert((range.end.line, range.end.character), i);
                }
                None => {
                    ends.insert((range.end.line, range.end.character), out.len());
                    out.push(TextEdit {
                        range,
                        new_text: canonical.to_owned(),
                    });
                }
            }
        }
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
