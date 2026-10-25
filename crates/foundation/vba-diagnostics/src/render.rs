use std::fmt::Write;

use vba_source::SourceMap;

use crate::Diagnostic;

/// Renders a diagnostic as plain text (no colors):
///
/// ```text
/// error[LEX001]: unterminated string literal
///  --> mod.bas:3:9
///   |
/// 3 | x = "abc
///   |     ^^^^
/// ```
///
/// Source lines are shown for display only (see
/// [`vba_source::SourceFile::line_text_lossy`]). Spans are underlined on the
/// first line they touch; columns are byte columns, shown 1-based.
pub fn render_plain(d: &Diagnostic, sources: &SourceMap) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}[{}]: {}", d.severity.as_str(), d.code.0, d.message);
    let Some(file) = sources.get(d.file) else {
        for n in &d.notes {
            let _ = writeln!(out, "  = note: {n}");
        }
        return out;
    };
    let lc = file.line_col(d.span.start);
    let _ = writeln!(out, " --> {}:{}:{}", file.name(), lc.line + 1, lc.col + 1);
    let spans = std::iter::once((d.span, None))
        .chain(d.labels.iter().map(|l| (l.span, Some(l.message.as_str()))));
    let mut rendered: Vec<(u32, u32, u32, Option<&str>)> = spans
        .map(|(s, m)| {
            let a = file.line_col(s.start);
            let width = if file.line_col(s.end).line == a.line {
                s.len().max(1)
            } else {
                file.lines()
                    .line_range(a.line)
                    .map_or(1, |(ls, le)| (le - ls).saturating_sub(a.col).max(1))
            };
            (a.line, a.col, width, m)
        })
        .collect();
    rendered.sort_by_key(|&(line, col, ..)| (line, col));
    let gutter = rendered
        .iter()
        .map(|r| (r.0 + 1).to_string().len())
        .max()
        .unwrap_or(1);
    let _ = writeln!(out, "{:gutter$} |", "");
    let mut last_line = None;
    for (line, col, width, msg) in rendered {
        if last_line != Some(line) {
            let text = file.line_text_lossy(line).unwrap_or_default();
            let _ = writeln!(out, "{:>gutter$} | {}", line + 1, text);
            last_line = Some(line);
        }
        let _ = write!(
            out,
            "{:gutter$} | {}{}",
            "",
            " ".repeat(col as usize),
            "^".repeat(width as usize)
        );
        if let Some(m) = msg {
            let _ = write!(out, " {m}");
        }
        out.push('\n');
    }
    for n in &d.notes {
        let _ = writeln!(out, "{:gutter$} = note: {n}", "");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Code;
    use vba_source::{SourceEncoding, Span};

    #[test]
    fn renders() {
        let mut map = SourceMap::new();
        let f = map.add(
            "m.bas",
            b"Dim a\r\nx = \"abc\r\n".to_vec(),
            SourceEncoding::Unknown,
        );
        let d = Diagnostic::error(Code("LEX001"), f, Span::new(11, 15), "unterminated string")
            .with_label(Span::new(9, 10), "assigned here")
            .with_note("strings end at the end of the line");
        let s = render_plain(&d, &map);
        let expected = "\
error[LEX001]: unterminated string
 --> m.bas:2:5
  |
2 | x = \"abc
  |   ^ assigned here
  |     ^^^^
  = note: strings end at the end of the line
";
        assert_eq!(s, expected);
    }
}
