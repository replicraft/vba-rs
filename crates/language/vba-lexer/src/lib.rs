//! Lossless, hand-written lexer for VBA source (MS-VBAL §3).
//!
//! * Input is `&[u8]`: VBA source lives in a Windows code page and need not be
//!   valid UTF-8. Token text is therefore `&[u8]`, borrowed from the input
//!   without copying.
//! * Every input byte belongs to exactly one [`Token`]; whitespace, line
//!   continuations and comments are trivia tokens. Concatenating the token
//!   texts reproduces the input.
//! * Keywords match case-insensitively and are gated by the language
//!   generation of the selected profile.
//! * Malformed literals produce an error token plus a diagnostic and lexing
//!   continues.
//!
//! See `docs/language/lexer.md` for what is and is not implemented.

mod date;
mod keyword;
mod lexer;
mod token;

pub use keyword::{Keyword, KeywordClass, lookup_keyword};
pub use lexer::{LexOptions, LexOutput, Lexer, lex};
pub use token::{Token, TokenKind, TypeSuffix};
