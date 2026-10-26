//! Diagnostics: lexer and parser findings plus pluggable semantic providers.

use vba_diagnostics::Severity as Sev;

use crate::document::FILE_ID;
use crate::types::{Diagnostic, Severity};
use crate::workspace::{SemanticInput, View};

fn convert_severity(s: Sev) -> Severity {
    match s {
        Sev::Error => Severity::Error,
        Sev::Warning => Severity::Warning,
        Sev::Info => Severity::Info,
        Sev::Hint => Severity::Hint,
    }
}

impl View<'_> {
    /// All diagnostics of the document: lexer and syntax diagnostics from
    /// the tolerant parse, then those of every registered
    /// [`SemanticDiagnostics`](crate::SemanticDiagnostics) provider,
    /// sorted by position.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut all: Vec<vba_diagnostics::Diagnostic> = self.analysis.parse.diagnostics.clone();
        let input = SemanticInput {
            file: FILE_ID,
            tree: self.tree(),
            profile: &self.config.profile,
        };
        for p in self.providers {
            all.extend(p.diagnose(&input));
        }
        let len = self.bytes().len() as u32;
        let mut out: Vec<Diagnostic> = all
            .into_iter()
            .map(|d| {
                let clamp =
                    |s: vba_source::Span| vba_source::Span::new(s.start.min(len), s.end.min(len));
                let mut message = d.message.clone();
                for n in &d.notes {
                    message.push_str("\nnote: ");
                    message.push_str(n);
                }
                Diagnostic {
                    range: self.range(clamp(d.span)),
                    severity: convert_severity(d.severity),
                    code: d.code.0.to_string(),
                    message,
                    related: d
                        .labels
                        .iter()
                        .map(|l| (self.range(clamp(l.span)), l.message.clone()))
                        .collect(),
                }
            })
            .collect();
        out.sort_by(|a, b| {
            (
                a.range.start,
                a.range.end,
                a.code.as_str(),
                a.message.as_str(),
            )
                .cmp(&(
                    b.range.start,
                    b.range.end,
                    b.code.as_str(),
                    b.message.as_str(),
                ))
        });
        out.dedup();
        out
    }
}
