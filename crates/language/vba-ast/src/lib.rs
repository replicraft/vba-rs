//! Typed abstract syntax tree for VBA, lowered from the `vba-syntax` CST.
//!
//! * [`Module`] holds [`Item`]s: declarations (separate types from
//!   executable statements) and procedures whose bodies are [`Block`]s of
//!   [`Stmt`]s. Every node carries a [`vba_source::Span`].
//! * [`Name`] compares case-insensitively and keeps the original spelling;
//!   [`Ident`] adds the type-declaration suffix and how the name was
//!   written.
//! * Line numbers and labels are [`StmtKind::Label`] statements; `GoTo`
//!   and friends carry a [`LabelRef`].
//! * Conditional compilation survives as [`Item::Directive`] /
//!   [`Item::Inactive`] markers (inactive text is never parsed).
//! * [`lower`] converts any CST, erroneous ones included, and reports
//!   error nodes as diagnostics; [`parse_module`] is the one-call helper.
//!
//! See `docs/language/syntax.md`.

mod ast;
mod lower;
mod name;

pub use ast::*;
pub use lower::{AST_BAD_LITERAL, AST_ERROR_NODE, AST_INCOMPLETE, Lowered, lower};
pub use name::{Ident, IdentKind, Name};

use vba_diagnostics::Diagnostic;
use vba_profile::Profile;
use vba_source::FileId;
use vba_syntax::{ParseOptions, SyntaxTree};

/// The result of [`parse_module`].
#[derive(Debug, Clone)]
pub struct ParsedModule {
    /// The lossless tree.
    pub tree: SyntaxTree,
    /// The typed AST.
    pub module: Module,
    /// Lexer, parser and lowering diagnostics, sorted.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parses `src` and lowers the tree.
pub fn parse_module(
    file: FileId,
    src: &[u8],
    profile: &Profile,
    opts: &ParseOptions,
) -> ParsedModule {
    let parsed = vba_syntax::parse(file, src, profile, opts);
    let lowered = lower(&parsed.tree, file);
    let mut all: vba_diagnostics::Diagnostics = parsed.diagnostics.into_iter().collect();
    all.extend(lowered.diagnostics);
    ParsedModule {
        tree: parsed.tree,
        module: lowered.module,
        diagnostics: all.into_sorted(),
    }
}
