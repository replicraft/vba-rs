//! The document store and the per-document request facade.

use indexmap::IndexMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use vba_lexer::TokenKind;
use vba_profile::Profile;
use vba_source::{FileId, SourceEncoding, Span};
use vba_syntax::{SyntaxToken, SyntaxTree};

use crate::document::{Analysis, Config, Document, TextChange};
use crate::project::{Project, SharedProvider};
use crate::text::{Position, Range, decode};
use crate::types::Error;

/// Input handed to a [`SemanticDiagnostics`] provider.
pub struct SemanticInput<'a> {
    /// The file id diagnostics must carry.
    pub file: FileId,
    /// The syntax tree of the current document version.
    pub tree: &'a SyntaxTree,
    /// The profile the document is analysed under.
    pub profile: &'a Profile,
}

/// A source of additional diagnostics, for example the semantic analyzer
/// that will come with `vba-sema`. Providers are consulted after the
/// syntactic diagnostics on every request for diagnostics; they receive the
/// shared tree and return diagnostics in the common
/// [`vba_diagnostics::Diagnostic`] form with byte spans.
pub trait SemanticDiagnostics: Send + Sync {
    /// Diagnostics for one document version.
    fn diagnose(&self, input: &SemanticInput<'_>) -> Vec<vba_diagnostics::Diagnostic>;
}

/// All open documents plus the configuration they are analysed with.
#[derive(Default)]
pub struct Workspace {
    config: Config,
    docs: IndexMap<String, Document>,
    providers: Vec<Box<dyn SemanticDiagnostics>>,
    types: Option<SharedProvider>,
    /// Bumped by every open, edit, close and configuration change; the
    /// project analysis is cached against it.
    generation: u64,
    project: Mutex<Option<(u64, Arc<Project>)>>,
    runs: AtomicUsize,
}

impl Workspace {
    /// An empty workspace with the default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Replaces the configuration; cached parses are dropped. The encoding
    /// applies to documents opened afterwards.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
        self.generation += 1;
        for d in self.docs.values_mut() {
            d.invalidate();
        }
    }

    /// Installs the embedding application's description of type libraries and
    /// host object models (`None` removes it). With a provider, library types
    /// resolve in the analysis, their members are completed and described,
    /// and calls to their members get signature help.
    pub fn set_type_provider(&mut self, provider: Option<SharedProvider>) {
        self.types = provider;
        self.generation += 1;
    }

    /// Registers an additional diagnostics provider.
    pub fn add_diagnostics_provider(&mut self, p: Box<dyn SemanticDiagnostics>) {
        self.providers.push(p);
    }

    /// Opens (or replaces) a document using the configured encoding.
    pub fn open(&mut self, uri: &str, text: &str, version: i32) {
        self.open_with_encoding(uri, text, version, self.config.encoding);
    }

    /// Opens (or replaces) a document with an explicit encoding.
    pub fn open_with_encoding(
        &mut self,
        uri: &str,
        text: &str,
        version: i32,
        encoding: SourceEncoding,
    ) {
        self.generation += 1;
        self.docs
            .insert(uri.to_string(), Document::new(text, version, encoding));
    }

    /// Applies edits to an open document.
    pub fn change(&mut self, uri: &str, version: i32, changes: &[TextChange]) -> Result<(), Error> {
        let d = self
            .docs
            .get_mut(uri)
            .ok_or_else(|| Error::NotOpen(uri.to_string()))?;
        d.apply_changes(version, changes);
        self.generation += 1;
        Ok(())
    }

    /// Closes a document; returns whether it was open.
    pub fn close(&mut self, uri: &str) -> bool {
        self.generation += 1;
        self.docs.shift_remove(uri).is_some()
    }

    /// How many times the project analysis has run. It runs at most once
    /// per workspace state (document set, texts, configuration) and only
    /// when a semantic feature asks for it.
    pub fn semantic_runs(&self) -> usize {
        self.runs.load(Ordering::Relaxed)
    }

    /// The cached analysis of all open documents as one project, built on
    /// demand. `None` when semantic analysis is disabled.
    pub(crate) fn project(&self) -> Option<Arc<Project>> {
        if !self.config.semantic {
            return None;
        }
        let mut slot = self.project.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((g, p)) = slot.as_ref()
            && *g == self.generation
        {
            return Some(p.clone());
        }
        let mut docs: Vec<(&str, &Document)> =
            self.docs.iter().map(|(u, d)| (u.as_str(), d)).collect();
        docs.sort_by(|a, b| a.0.cmp(b.0));
        let p = Arc::new(Project::build(&self.config, &docs, self.types.clone()));
        self.runs.fetch_add(1, Ordering::Relaxed);
        *slot = Some((self.generation, p.clone()));
        Some(p)
    }

    pub(crate) fn docs(&self) -> &IndexMap<String, Document> {
        &self.docs
    }

    /// The open document, if any.
    pub fn document(&self, uri: &str) -> Option<&Document> {
        self.docs.get(uri)
    }

    /// URIs of all open documents.
    pub fn open_uris(&self) -> Vec<&str> {
        self.docs.keys().map(String::as_str).collect()
    }

    /// The request facade for an open document.
    pub fn view(&self, uri: &str) -> Result<View<'_>, Error> {
        let doc = self
            .docs
            .get(uri)
            .ok_or_else(|| Error::NotOpen(uri.to_string()))?;
        Ok(View {
            doc,
            analysis: doc.analysis(&self.config),
            config: &self.config,
            providers: &self.providers,
            ws: self,
            uri: uri.to_string(),
        })
    }
}

/// Language features for one version of one document.
pub struct View<'a> {
    pub(crate) doc: &'a Document,
    pub(crate) analysis: Arc<Analysis>,
    pub(crate) config: &'a Config,
    pub(crate) providers: &'a [Box<dyn SemanticDiagnostics>],
    pub(crate) ws: &'a Workspace,
    pub(crate) uri: String,
}

impl<'a> View<'a> {
    /// The document.
    pub fn document(&self) -> &'a Document {
        self.doc
    }

    /// The cached analysis of this version.
    pub fn analysis(&self) -> &Arc<Analysis> {
        &self.analysis
    }

    /// The syntax tree.
    pub fn tree(&self) -> &SyntaxTree {
        self.analysis.tree()
    }

    /// The profile in force.
    pub fn profile(&self) -> &Profile {
        &self.config.profile
    }

    /// The buffer bytes.
    pub fn bytes(&self) -> &[u8] {
        self.doc.bytes()
    }

    /// Byte span to range.
    pub fn range(&self, span: Span) -> Range {
        self.doc.text_map().range(self.bytes(), span)
    }

    /// Range to byte span.
    pub fn span(&self, range: Range) -> Span {
        self.doc.text_map().span(self.bytes(), range)
    }

    /// Position to byte offset.
    pub fn offset(&self, pos: Position) -> u32 {
        self.doc.text_map().offset(self.bytes(), pos)
    }

    /// Byte offset to position.
    pub fn position(&self, offset: u32) -> Position {
        self.doc.text_map().position(self.bytes(), offset)
    }

    /// Decoded text of a span.
    pub fn text(&self, span: Span) -> String {
        let b = &self.bytes()[span.start as usize..span.end as usize];
        decode(b, self.doc.text_map().encoding())
    }

    /// Number of lines.
    pub fn line_count(&self) -> u32 {
        self.doc.text_map().line_count()
    }

    /// The syntactic scope index of this version.
    pub(crate) fn scopes(&self) -> &crate::scope::ScopeIndex {
        self.analysis.scopes(self.doc.text_map().encoding())
    }

    /// All tokens, trivia included, in source order.
    pub fn tokens(&self) -> Vec<SyntaxToken<'_>> {
        self.tree().root().descendant_tokens()
    }
}

/// Whether a token is part of a word (identifier, keyword, foreign name).
pub(crate) fn is_word(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_)
    )
}

/// Index of the token under `offset`. At a boundary between a word and a
/// non-word the word wins, so a cursor directly after a name selects it.
pub(crate) fn token_index_at(tokens: &[SyntaxToken<'_>], offset: u32) -> Option<usize> {
    if tokens.is_empty() {
        return None;
    }
    let i = tokens.partition_point(|t| t.span().end <= offset);
    let i = i.min(tokens.len() - 1);
    if i > 0
        && tokens[i].span().start == offset
        && is_word(tokens[i - 1].kind())
        && !is_word(tokens[i].kind())
    {
        return Some(i - 1);
    }
    Some(i)
}
