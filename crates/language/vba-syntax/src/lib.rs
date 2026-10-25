//! Lossless concrete syntax tree and error-tolerant parser for VBA
//! (MS-VBAL §4 module structure and §5 statements and expressions).
//!
//! * [`parse`] consumes `vba-lexer` tokens and builds a [`SyntaxTree`] in
//!   which every source byte (whitespace, comments, line continuations and
//!   text that failed to parse included) belongs to exactly one token, so
//!   the tree's text always equals the input.
//! * The parser is hand-written: recursive descent for declarations and
//!   statements, Pratt parsing for expressions.
//! * Syntax errors never abort: the offending logical line is wrapped in an
//!   [`NodeKind::Error`] node and parsing resumes at the next statement.
//!   [`ParseOptions::strict`] turns tolerated deviations into errors, and
//!   [`Parse::into_result`] fails on any error.
//! * Conditional compilation (`#If`) is evaluated with the profile's
//!   predefined constants; inactive branches are kept as
//!   [`NodeKind::InactiveText`] and are not parsed.
//!
//! See `docs/syntax.md` for node shapes, recovery and known gaps.

mod cc;
mod kind;
mod parser;
mod prec;
mod tree;

pub use kind::NodeKind;
pub use parser::{Parse, ParseOptions, parse};
pub use tree::{NodeId, SyntaxElement, SyntaxNode, SyntaxToken, SyntaxTree};
