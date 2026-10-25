use vba_source::Span;

use crate::Keyword;

/// The kind of a lexical token or trivia run.
///
/// Every byte of the input belongs to exactly one token. Trivia kinds
/// ([`TokenKind::is_trivia`]) are tokens too, so concatenating all token texts
/// reproduces the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // ---- trivia -------------------------------------------------------
    /// A run of spaces, tabs and 0x19 (MS-VBAL §3.2.2 `WSC`, byte-encodable
    /// members only).
    Whitespace,
    /// `1*WSC "_" line-terminator` (§3.2.2 `line-continuation`).
    LineContinuation,
    /// A `'` comment up to, not including, the line terminator (§3.3.1
    /// `comment-body`, continuing over `line-continuation`).
    Comment,
    /// A `Rem` comment (same body rules as [`TokenKind::Comment`]).
    RemComment,

    // ---- structure ----------------------------------------------------
    /// A line terminator: CRLF, CR or LF (§3.2.1).
    Newline,
    /// `:` statement separator.
    Colon,

    // ---- names and words ---------------------------------------------
    /// An identifier (§3.3.5), including an optional type suffix
    /// (`TYPED-NAME`, §3.3.5.3).
    Ident,
    /// A bracketed foreign name `[...]` (§3.3.5.3 `FOREIGN-NAME`).
    ForeignName,
    /// A reserved identifier available in the selected language generation.
    Keyword(Keyword),

    // ---- literals -----------------------------------------------------
    /// Decimal `INTEGER` literal with optional `%`, `&` or `^` suffix.
    IntegerLiteral,
    /// `&H` hexadecimal literal with optional suffix.
    HexLiteral,
    /// `&O` or `&` octal literal with optional suffix.
    OctalLiteral,
    /// `FLOAT` literal: fraction and/or exponent, or a decimal literal with a
    /// `!`, `#` or `@` suffix.
    FloatLiteral,
    /// `STRING` literal including both quotes (§3.3.4).
    StringLiteral,
    /// `DATE` literal including both `#` (§3.3.3).
    DateLiteral,

    // ---- conditional compilation (§4) --------------------------------
    /// `#If`
    HashIf,
    /// `#ElseIf`
    HashElseIf,
    /// `#Else`
    HashElse,
    /// `#End If` (also `#EndIf`)
    HashEndIf,
    /// `#Const`
    HashConst,

    // ---- operators and punctuation (§3.3.1 special-token) ------------
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `\`
    Backslash,
    /// `^`
    Caret,
    /// `&`
    Amp,
    /// `=`
    Eq,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `<>`
    Ne,
    /// `:=` named argument assignment.
    ColonEq,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
    /// `.`
    Dot,
    /// `!` dictionary access operator.
    Bang,
    /// `#` (file-number marker).
    Hash,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `?`
    Question,

    // ---- errors ------------------------------------------------------
    /// A `"` string that reaches the end of the line without a closing quote.
    UnterminatedString,
    /// A `#` date literal that is unterminated or malformed.
    BadDate,
    /// A `[` foreign name that is unterminated or empty.
    BadForeignName,
    /// A byte that cannot start any token.
    Unknown,
}

impl TokenKind {
    /// Whether the token carries no syntax: whitespace, continuation, comment.
    pub const fn is_trivia(self) -> bool {
        matches!(
            self,
            TokenKind::Whitespace
                | TokenKind::LineContinuation
                | TokenKind::Comment
                | TokenKind::RemComment
        )
    }

    /// Whether the token is an error token (a diagnostic was reported).
    pub const fn is_error(self) -> bool {
        matches!(
            self,
            TokenKind::UnterminatedString
                | TokenKind::BadDate
                | TokenKind::BadForeignName
                | TokenKind::Unknown
        )
    }

    /// Whether the token is a numeric literal.
    pub const fn is_number(self) -> bool {
        matches!(
            self,
            TokenKind::IntegerLiteral
                | TokenKind::HexLiteral
                | TokenKind::OctalLiteral
                | TokenKind::FloatLiteral
        )
    }
}

/// A type-declaration suffix (§3.3.5.3 `type-suffix`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeSuffix {
    /// `%` Integer
    Integer,
    /// `&` Long
    Long,
    /// `^` LongLong
    LongLong,
    /// `!` Single
    Single,
    /// `#` Double
    Double,
    /// `@` Currency
    Currency,
    /// `$` String
    String,
}

impl TypeSuffix {
    /// Maps a suffix byte.
    pub const fn from_byte(b: u8) -> Option<TypeSuffix> {
        Some(match b {
            b'%' => TypeSuffix::Integer,
            b'&' => TypeSuffix::Long,
            b'^' => TypeSuffix::LongLong,
            b'!' => TypeSuffix::Single,
            b'#' => TypeSuffix::Double,
            b'@' => TypeSuffix::Currency,
            b'$' => TypeSuffix::String,
            _ => return None,
        })
    }
}

/// A token: its kind, byte span and the exact source bytes (zero-copy).
///
/// Text is bytes, not `str`: VBA source is stored in a Windows code page and
/// may not be valid UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token<'a> {
    /// What the token is.
    pub kind: TokenKind,
    /// Byte range in the source.
    pub span: Span,
    /// The source bytes of the token, exactly as written.
    pub text: &'a [u8],
}

impl Token<'_> {
    /// The type suffix, for identifiers and numeric literals that carry one.
    pub fn type_suffix(&self) -> Option<TypeSuffix> {
        let last = *self.text.last()?;
        match self.kind {
            TokenKind::Ident => {
                // `$` etc. cannot otherwise end an identifier.
                TypeSuffix::from_byte(last)
            }
            TokenKind::IntegerLiteral | TokenKind::HexLiteral | TokenKind::OctalLiteral => {
                match last {
                    b'%' | b'&' | b'^' => TypeSuffix::from_byte(last),
                    _ => None,
                }
            }
            TokenKind::FloatLiteral => match last {
                b'!' | b'#' | b'@' => TypeSuffix::from_byte(last),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether the token is trivia.
    pub fn is_trivia(&self) -> bool {
        self.kind.is_trivia()
    }
}
