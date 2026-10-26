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
//! Positions are LSP positions (zero-based line, UTF-16 column); see
//! [`text`]. Name resolution is syntactic only (see `docs/lsp.md`).

mod builtin;
mod completion;
mod diagnostics;
mod document;
mod folding;
mod format;
mod hover;
mod navigation;
mod scope;
mod selection;
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
pub use workspace::{SemanticDiagnostics, SemanticInput, View, Workspace};
