//! Conversions between `vba-lsp-core` types and `lsp-types`.

use lsp_types as lsp;
use vba_lsp_core as core;

pub fn position(p: lsp::Position) -> core::Position {
    core::Position::new(p.line, p.character)
}

pub fn to_position(p: core::Position) -> lsp::Position {
    lsp::Position::new(p.line, p.character)
}

pub fn range(r: lsp::Range) -> core::Range {
    core::Range::new(position(r.start), position(r.end))
}

pub fn to_range(r: core::Range) -> lsp::Range {
    lsp::Range::new(to_position(r.start), to_position(r.end))
}

/// A location of the workspace; `None` when its URI cannot be represented.
pub fn location(l: core::Location) -> Option<lsp::Location> {
    Some(lsp::Location::new(l.uri.parse().ok()?, to_range(l.range)))
}

/// A multi-document rename as a `WorkspaceEdit`.
#[allow(clippy::mutable_key_type)] // `WorkspaceEdit::changes` is keyed by `Uri`
pub fn workspace_edit(files: Vec<core::FileEdit>) -> lsp::WorkspaceEdit {
    let changes = files
        .into_iter()
        .filter_map(|f| {
            let uri = f.uri.parse::<lsp::Uri>().ok()?;
            Some((uri, f.edits.into_iter().map(text_edit).collect()))
        })
        .collect();
    lsp::WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }
}

/// A workspace symbol as `SymbolInformation`.
#[allow(deprecated)] // `deprecated` is a required field of SymbolInformation
pub fn workspace_symbol(s: core::WorkspaceSymbol) -> Option<lsp::SymbolInformation> {
    Some(lsp::SymbolInformation {
        name: s.name,
        kind: symbol_kind(s.kind),
        tags: None,
        deprecated: None,
        location: location(s.location)?,
        container_name: (!s.container.is_empty()).then_some(s.container),
    })
}

pub fn text_edit(e: core::TextEdit) -> lsp::TextEdit {
    lsp::TextEdit::new(to_range(e.range), e.new_text)
}

pub fn diagnostic(uri: &lsp::Uri, d: core::Diagnostic) -> lsp::Diagnostic {
    lsp::Diagnostic {
        range: to_range(d.range),
        severity: Some(match d.severity {
            core::Severity::Error => lsp::DiagnosticSeverity::ERROR,
            core::Severity::Warning => lsp::DiagnosticSeverity::WARNING,
            core::Severity::Info => lsp::DiagnosticSeverity::INFORMATION,
            core::Severity::Hint => lsp::DiagnosticSeverity::HINT,
        }),
        code: Some(lsp::NumberOrString::String(d.code)),
        source: Some("vba".to_string()),
        message: d.message,
        related_information: (!d.related.is_empty()).then(|| {
            d.related
                .into_iter()
                .map(|(r, m)| lsp::DiagnosticRelatedInformation {
                    location: lsp::Location::new(uri.clone(), to_range(r)),
                    message: m,
                })
                .collect()
        }),
        ..Default::default()
    }
}

fn symbol_kind(k: core::SymbolKind) -> lsp::SymbolKind {
    match k {
        core::SymbolKind::Method => lsp::SymbolKind::METHOD,
        core::SymbolKind::Function => lsp::SymbolKind::FUNCTION,
        core::SymbolKind::Property => lsp::SymbolKind::PROPERTY,
        core::SymbolKind::Field => lsp::SymbolKind::FIELD,
        core::SymbolKind::Variable => lsp::SymbolKind::VARIABLE,
        core::SymbolKind::Constant => lsp::SymbolKind::CONSTANT,
        core::SymbolKind::Enum => lsp::SymbolKind::ENUM,
        core::SymbolKind::EnumMember => lsp::SymbolKind::ENUM_MEMBER,
        core::SymbolKind::Struct => lsp::SymbolKind::STRUCT,
        core::SymbolKind::Event => lsp::SymbolKind::EVENT,
        core::SymbolKind::Module => lsp::SymbolKind::MODULE,
        core::SymbolKind::Class => lsp::SymbolKind::CLASS,
    }
}

#[allow(deprecated)] // `deprecated` is a required field of DocumentSymbol
pub fn document_symbol(s: core::DocumentSymbol) -> lsp::DocumentSymbol {
    lsp::DocumentSymbol {
        name: s.name,
        detail: (!s.detail.is_empty()).then_some(s.detail),
        kind: symbol_kind(s.kind),
        tags: None,
        deprecated: None,
        range: to_range(s.range),
        selection_range: to_range(s.selection_range),
        children: (!s.children.is_empty())
            .then(|| s.children.into_iter().map(document_symbol).collect()),
    }
}

/// Flattens the outline for clients without hierarchical symbol support.
#[allow(deprecated)] // `deprecated` is a required field of SymbolInformation
pub fn flat_symbols(
    uri: &lsp::Uri,
    syms: Vec<core::DocumentSymbol>,
    container: Option<&str>,
    out: &mut Vec<lsp::SymbolInformation>,
) {
    for s in syms {
        out.push(lsp::SymbolInformation {
            name: s.name.clone(),
            kind: symbol_kind(s.kind),
            tags: None,
            deprecated: None,
            location: lsp::Location::new(uri.clone(), to_range(s.range)),
            container_name: container.map(str::to_string),
        });
        flat_symbols(uri, s.children, Some(&s.name), out);
    }
}

pub fn folding(f: core::FoldingRange) -> lsp::FoldingRange {
    lsp::FoldingRange {
        start_line: f.start_line,
        start_character: None,
        end_line: f.end_line,
        end_character: None,
        kind: Some(match f.kind {
            core::FoldingKind::Comment => lsp::FoldingRangeKind::Comment,
            core::FoldingKind::Region => lsp::FoldingRangeKind::Region,
        }),
        collapsed_text: None,
    }
}

pub fn legend() -> lsp::SemanticTokensLegend {
    lsp::SemanticTokensLegend {
        token_types: core::TOKEN_TYPES
            .iter()
            .map(|t| lsp::SemanticTokenType::new(t))
            .collect(),
        token_modifiers: core::TOKEN_MODIFIERS
            .iter()
            .map(|t| lsp::SemanticTokenModifier::new(t))
            .collect(),
    }
}

pub fn semantic_tokens(tokens: &[core::SemanticToken]) -> lsp::SemanticTokens {
    let data = core::encode_semantic_tokens(tokens);
    lsp::SemanticTokens {
        result_id: None,
        data: data
            .as_chunks::<5>()
            .0
            .iter()
            .map(|c| lsp::SemanticToken {
                delta_line: c[0],
                delta_start: c[1],
                length: c[2],
                token_type: c[3],
                token_modifiers_bitset: c[4],
            })
            .collect(),
    }
}

pub fn hover(h: core::Hover) -> lsp::Hover {
    lsp::Hover {
        contents: lsp::HoverContents::Markup(lsp::MarkupContent {
            kind: lsp::MarkupKind::Markdown,
            value: h.contents,
        }),
        range: Some(to_range(h.range)),
    }
}

pub fn completions(c: core::Completions) -> lsp::CompletionList {
    let replace = to_range(c.replace);
    lsp::CompletionList {
        is_incomplete: false,
        items: c
            .items
            .into_iter()
            .map(|i| lsp::CompletionItem {
                kind: Some(match i.kind {
                    core::CompletionKind::Keyword => lsp::CompletionItemKind::KEYWORD,
                    core::CompletionKind::Function => lsp::CompletionItemKind::FUNCTION,
                    core::CompletionKind::Procedure => lsp::CompletionItemKind::METHOD,
                    core::CompletionKind::Variable => lsp::CompletionItemKind::VARIABLE,
                    core::CompletionKind::Constant => lsp::CompletionItemKind::CONSTANT,
                    core::CompletionKind::Type => lsp::CompletionItemKind::CLASS,
                    core::CompletionKind::EnumMember => lsp::CompletionItemKind::ENUM_MEMBER,
                    core::CompletionKind::Module => lsp::CompletionItemKind::MODULE,
                    core::CompletionKind::Property => lsp::CompletionItemKind::PROPERTY,
                    core::CompletionKind::Field => lsp::CompletionItemKind::FIELD,
                }),
                detail: i.detail,
                text_edit: Some(lsp::CompletionTextEdit::Edit(lsp::TextEdit::new(
                    replace,
                    i.label.clone(),
                ))),
                label: i.label,
                ..Default::default()
            })
            .collect(),
    }
}

pub fn signature_help(h: core::SignatureHelp) -> lsp::SignatureHelp {
    lsp::SignatureHelp {
        signatures: h
            .signatures
            .into_iter()
            .map(|s| lsp::SignatureInformation {
                label: s.label,
                documentation: s.documentation.map(|d| {
                    lsp::Documentation::MarkupContent(lsp::MarkupContent {
                        kind: lsp::MarkupKind::Markdown,
                        value: d,
                    })
                }),
                parameters: Some(
                    s.parameters
                        .into_iter()
                        .map(|p| lsp::ParameterInformation {
                            label: lsp::ParameterLabel::LabelOffsets([p.start, p.end]),
                            documentation: None,
                        })
                        .collect(),
                ),
                active_parameter: h.active_parameter,
            })
            .collect(),
        active_signature: Some(0),
        active_parameter: h.active_parameter,
    }
}

pub fn selection_range(s: core::SelectionRange) -> lsp::SelectionRange {
    let mut parent: Option<Box<lsp::SelectionRange>> = None;
    for r in s.chain.into_iter().rev() {
        parent = Some(Box::new(lsp::SelectionRange {
            range: to_range(r),
            parent,
        }));
    }
    match parent {
        Some(b) => *b,
        None => lsp::SelectionRange {
            range: lsp::Range::default(),
            parent: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_chain_nests_innermost_first() {
        let r = |a, b| core::Range::new(core::Position::new(0, a), core::Position::new(0, b));
        let s = selection_range(core::SelectionRange {
            chain: vec![r(2, 3), r(1, 4), r(0, 5)],
        });
        assert_eq!(s.range, to_range(r(2, 3)));
        let p = s.parent.unwrap();
        assert_eq!(p.range, to_range(r(1, 4)));
        assert_eq!(p.parent.unwrap().range, to_range(r(0, 5)));
    }

    #[test]
    fn semantic_tokens_are_chunked() {
        let t = core::SemanticToken {
            line: 1,
            start: 2,
            length: 3,
            token_type: 4,
            modifiers: 1,
        };
        let out = semantic_tokens(&[t, t]);
        assert_eq!(out.data.len(), 2);
        assert_eq!(out.data[0].delta_line, 1);
        assert_eq!(out.data[1].delta_line, 0);
        assert_eq!(out.data[1].delta_start, 0);
    }
}
