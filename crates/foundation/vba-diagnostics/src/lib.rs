//! Diagnostics shared by all vba-rs components.
//!
//! A [`Diagnostic`] has a severity, a stable code, a primary span, a message
//! and optional secondary labels and notes. [`Diagnostics`] collects them and
//! sorts them deterministically; [`render_plain`] formats them as plain text.

mod render;

pub use render::render_plain;
use vba_source::{FileId, Span};

/// How serious a diagnostic is. Ordered from most to least severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    /// The program is invalid.
    Error,
    /// Suspicious but accepted.
    Warning,
    /// Informational.
    Info,
    /// Editor hint.
    Hint,
}

impl Severity {
    /// Lower-case name used in rendered output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        }
    }
}

/// A stable diagnostic code such as `LEX001`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Code(pub &'static str);

/// A secondary span with an explanatory message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// Span in the same file as the diagnostic.
    pub span: Span,
    /// Text shown next to the span.
    pub message: String,
}

/// One problem found in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Severity.
    pub severity: Severity,
    /// Stable code.
    pub code: Code,
    /// File the span refers to.
    pub file: FileId,
    /// Primary span.
    pub span: Span,
    /// Human readable message.
    pub message: String,
    /// Secondary labels.
    pub labels: Vec<Label>,
    /// Free-form notes.
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Creates a diagnostic without labels or notes.
    pub fn new(
        severity: Severity,
        code: Code,
        file: FileId,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Diagnostic {
            severity,
            code,
            file,
            span,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Creates an error.
    pub fn error(code: Code, file: FileId, span: Span, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, file, span, message)
    }

    /// Creates a warning.
    pub fn warning(code: Code, file: FileId, span: Span, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, file, span, message)
    }

    /// Adds a secondary label.
    #[must_use]
    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
        });
        self
    }

    /// Adds a note.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// The key defining the deterministic order: file, position, severity
    /// (errors first), code, then message.
    fn sort_key(&self) -> (FileId, u32, u32, Severity, Code, &str) {
        (
            self.file,
            self.span.start,
            self.span.end,
            self.severity,
            self.code,
            &self.message,
        )
    }
}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key()
            .cmp(&other.sort_key())
            .then_with(|| self.labels.len().cmp(&other.labels.len()))
            .then_with(|| self.notes.cmp(&other.notes))
    }
}

/// A collection of diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a diagnostic.
    pub fn push(&mut self, d: Diagnostic) {
        self.items.push(d);
    }

    /// Moves all diagnostics of `other` into `self`.
    pub fn extend(&mut self, other: impl IntoIterator<Item = Diagnostic>) {
        self.items.extend(other);
    }

    /// Whether any diagnostic has [`Severity::Error`].
    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    /// Number of diagnostics.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are no diagnostics.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Diagnostics in insertion order.
    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.items
    }

    /// Sorts deterministically (see [`Diagnostic`] ordering) and removes exact
    /// duplicates.
    pub fn sort_and_dedup(&mut self) {
        self.items.sort();
        self.items.dedup();
    }

    /// Consumes the collection, returning the diagnostics in sorted order.
    pub fn into_sorted(mut self) -> Vec<Diagnostic> {
        self.sort_and_dedup();
        self.items
    }
}

impl FromIterator<Diagnostic> for Diagnostics {
    fn from_iter<T: IntoIterator<Item = Diagnostic>>(iter: T) -> Self {
        Diagnostics {
            items: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(sev: Severity, start: u32, msg: &str) -> Diagnostic {
        Diagnostic::new(
            sev,
            Code("T001"),
            FileId(0),
            Span::new(start, start + 1),
            msg,
        )
    }

    #[test]
    fn ordering_is_deterministic() {
        let a = vec![
            d(Severity::Warning, 5, "b"),
            d(Severity::Error, 5, "z"),
            d(Severity::Error, 1, "a"),
            d(Severity::Error, 5, "z"),
        ];
        let mut rev = a.clone();
        rev.reverse();
        let x = a.into_iter().collect::<Diagnostics>().into_sorted();
        let y = rev.into_iter().collect::<Diagnostics>().into_sorted();
        assert_eq!(x, y);
        assert_eq!(x.len(), 3);
        assert_eq!(x[0].span.start, 1);
        assert_eq!(x[1].severity, Severity::Error);
        assert_eq!(x[2].severity, Severity::Warning);
    }

    #[test]
    fn has_errors() {
        let mut ds = Diagnostics::new();
        ds.push(d(Severity::Warning, 0, "w"));
        assert!(!ds.has_errors());
        ds.push(d(Severity::Error, 0, "e"));
        assert!(ds.has_errors());
    }
}
