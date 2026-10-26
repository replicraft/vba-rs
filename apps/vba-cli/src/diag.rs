//! Rendering diagnostics with source snippets.

use std::sync::atomic::{AtomicBool, Ordering};

use annotate_snippets::{AnnotationKind, Group, Level, Renderer, Snippet};
use vba_diagnostics::{Diagnostic, Severity};
use vba_source::{FileId, SourceEncoding, SourceFile, SourceMap};

/// A file's text as the renderer needs it: valid UTF-8, with a map from the
/// original byte offsets (the offsets in diagnostics) to offsets in `text`.
///
/// Source is bytes in a Windows code page. UTF-8 files are shown as they are;
/// any other file is shown as Latin-1 for display only, which widens every
/// byte of 0x80 or above to two UTF-8 bytes, so offsets shift accordingly.
struct Display {
    text: String,
    /// `true` when `text` has the same offsets as the original bytes.
    identity: bool,
    /// For a widened file: the original offsets of the bytes of 0x80 or above.
    high: Vec<usize>,
}

impl Display {
    fn new(file: &SourceFile) -> Self {
        let bytes = file.bytes();
        if file.encoding() == SourceEncoding::Utf8
            && let Ok(s) = std::str::from_utf8(bytes)
        {
            return Display {
                text: s.to_owned(),
                identity: true,
                high: Vec::new(),
            };
        }
        Display {
            text: bytes.iter().map(|&b| b as char).collect(),
            identity: false,
            high: bytes
                .iter()
                .enumerate()
                .filter(|(_, b)| **b >= 0x80)
                .map(|(i, _)| i)
                .collect(),
        }
    }

    /// Maps an original byte offset to an offset in `text`.
    fn offset(&self, original: u32) -> usize {
        let o = original as usize;
        if self.identity {
            return o.min(self.text.len());
        }
        let widened = self.high.partition_point(|&h| h < o);
        (o + widened).min(self.text.len())
    }

    fn range(&self, start: u32, end: u32) -> std::ops::Range<usize> {
        let s = self.offset(start);
        let e = self.offset(end).max(s);
        // Show an empty span as one character wide.
        if s == e && s < self.text.len() {
            let width = self.text[s..].chars().next().map_or(0, char::len_utf8);
            s..s + width
        } else {
            s..e
        }
    }
}

fn level(severity: Severity) -> Level<'static> {
    match severity {
        Severity::Error => Level::ERROR,
        Severity::Warning => Level::WARNING,
        Severity::Info => Level::INFO,
        Severity::Hint => Level::HELP,
    }
}

/// Renders one diagnostic with its source snippet.
fn render_one(d: &Diagnostic, sources: &SourceMap, renderer: &Renderer) -> String {
    let mut groups: Vec<Group<'_>> = Vec::new();
    let title = level(d.severity).primary_title(&d.message).id(d.code.0);
    let display = sources.get(d.file).map(|f| (f, Display::new(f)));
    match &display {
        Some((file, disp)) => {
            let mut snippet = Snippet::source(disp.text.as_str())
                .path(file.name())
                .fold(true)
                .annotation(AnnotationKind::Primary.span(disp.range(d.span.start, d.span.end)));
            for l in &d.labels {
                snippet = snippet.annotation(
                    AnnotationKind::Context
                        .span(disp.range(l.span.start, l.span.end))
                        .label(l.message.as_str()),
                );
            }
            groups.push(Group::with_title(title).element(snippet));
        }
        None => groups.push(Group::with_title(title)),
    }
    for n in &d.notes {
        groups.push(Group::with_title(Level::NOTE.secondary_title(n.as_str())));
    }
    renderer.render(&groups)
}

static COLOR: AtomicBool = AtomicBool::new(false);

/// Whether the diagnostics on standard output get ANSI colours.
pub fn set_color(on: bool) {
    COLOR.store(on, Ordering::Relaxed);
}

/// Renders every diagnostic, each followed by a blank line.
pub fn render_all(diags: &[Diagnostic], sources: &SourceMap) -> String {
    let renderer = if COLOR.load(Ordering::Relaxed) {
        Renderer::styled()
    } else {
        Renderer::plain()
    };
    let mut out = String::new();
    for d in diags {
        out.push_str(&render_one(d, sources, &renderer));
        out.push_str("\n\n");
    }
    out
}

/// A copy of `sources` whose files are named by `names` (parallel to the
/// files), so diagnostics point at paths rather than module names.
pub fn rename(sources: &SourceMap, names: &[String]) -> SourceMap {
    let mut out = SourceMap::new();
    for (i, name) in names.iter().enumerate() {
        let Some(f) = sources.get(FileId(i as u32)) else {
            break;
        };
        out.add(name.clone(), f.bytes().to_vec(), f.encoding());
    }
    out
}

/// The diagnostics as JSON values: code, severity, message, a 1-based
/// `line`/`column` range in `file`, labels and notes.
pub fn to_json(diags: &[Diagnostic], sources: &SourceMap) -> Vec<serde_json::Value> {
    use serde_json::json;
    let pos = |file, offset| {
        sources.get(file).map_or((0, 0), |f| {
            let lc = f.line_col(offset);
            (lc.line + 1, lc.col + 1)
        })
    };
    diags
        .iter()
        .map(|d| {
            let (line, column) = pos(d.file, d.span.start);
            let (end_line, end_column) = pos(d.file, d.span.end);
            let labels: Vec<_> = d
                .labels
                .iter()
                .map(|l| {
                    let (line, column) = pos(d.file, l.span.start);
                    json!({ "line": line, "column": column, "message": l.message })
                })
                .collect();
            json!({
                "code": d.code.0,
                "severity": d.severity.as_str(),
                "message": d.message,
                "file": sources.get(d.file).map(|f| f.name().to_owned()),
                "line": line,
                "column": column,
                "end_line": end_line,
                "end_column": end_column,
                "labels": labels,
                "notes": d.notes,
            })
        })
        .collect()
}
