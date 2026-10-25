//! Source text storage for VBA modules.
//!
//! VBA source is stored in a Windows code page, not UTF-8, so this crate keeps
//! the original bytes untouched together with the declared encoding. Positions
//! are byte offsets ([`Span`]); [`LineIndex`] maps them to line and column.
//! Lines end with CR, LF or CRLF (MS-VBAL §3.2.1 physical line grammar).

mod line_index;
mod source;
mod span;

pub use line_index::{LineCol, LineIndex};
pub use source::{FileId, SourceEncoding, SourceFile, SourceMap};
pub use span::Span;
