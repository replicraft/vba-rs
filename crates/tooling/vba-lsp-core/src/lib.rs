//! Editor-agnostic language intelligence for VBA.
//!
//! The crate has no protocol transport and no async runtime. A
//! [`Workspace`] holds open documents (byte buffers in a declared encoding,
//! edited incrementally) and a per-version parse cache built on the one
//! shared parser, `vba-syntax`, in tolerant mode so every feature works on
//! broken code. [`Workspace::view`] returns a [`View`] with the features:
//! diagnostics, document symbols, folding, semantic tokens, hover,
//! completion, signature help, definition, references, rename, selection
//! ranges and formatting.
//!
//! The open documents are also analysed together as one `vba-sema` project
//! (cached per workspace state): `SEM###` diagnostics, cross-module
//! definition, references and rename ([`Workspace::definition`] and
//! friends), type-aware hover and member completion, and
//! [`Workspace::workspace_symbols`]. Names the project does not bind fall
//! back to a syntactic walk of the one document.
//!
//! Positions are LSP positions (zero-based line, UTF-16 column); see
//! [`text`]. See `docs/lsp.md`.

mod builtin;
mod completion;
mod describe;
mod diagnostics;
mod document;
mod folding;
mod format;
mod hover;
mod navigation;
mod project;
mod resolve;
mod scope;
mod selection;
mod semantic;
mod semantic_tokens;
mod signature;
mod symbols;
pub mod text;
mod types;
mod util;
mod workspace;

pub use document::{Analysis, Config, Document, FILE_ID, TextChange, parse_encoding};
pub use format::apply_edits;
pub use semantic_tokens::encode_semantic_tokens;
pub use text::{Position, Range, TextMap};
pub use types::*;
pub use vba_project::ModuleKind;
pub use workspace::{SemanticDiagnostics, SemanticInput, View, Workspace};
