//! Operator precedence (MS-VBAL §5.6, table of operator precedence).
//!
//! From loosest to tightest binding: `Imp`, `Eqv`, `Xor`, `Or`, `And`,
//! `Not` (prefix), comparison (`= <> < > <= >= Like Is`), `&`, `+ -`, `Mod`,
//! `\`, `* /`, unary `+ -`, `^`. All binary operators are left associative,
//! including `^` (`2^3^2` is 64). Unary minus binds looser than `^`
//! (`-2^2` is -4) but its operand may itself start with a unary operator
//! (`2^-2`).

use vba_lexer::{Keyword, TokenKind};

/// Binding power of the prefix `Not` operand: operators at least this
/// tight are part of the operand (comparisons and above).
pub(crate) const NOT_OPERAND_BP: u8 = 7;
/// Binding power of a unary `+`/`-` operand: only `^`.
pub(crate) const UNARY_OPERAND_BP: u8 = 14;

/// The binding power of a binary operator token, higher binds tighter.
pub(crate) fn binary_bp(k: TokenKind) -> Option<u8> {
    use TokenKind as T;
    Some(match k {
        T::Keyword(Keyword::Imp) => 1,
        T::Keyword(Keyword::Eqv) => 2,
        T::Keyword(Keyword::Xor) => 3,
        T::Keyword(Keyword::Or) => 4,
        T::Keyword(Keyword::And) => 5,
        T::Eq | T::Ne | T::Lt | T::Gt | T::Le | T::Ge => 7,
        T::Keyword(Keyword::Like | Keyword::Is) => 7,
        T::Amp => 8,
        T::Plus | T::Minus => 9,
        T::Keyword(Keyword::Mod) => 10,
        T::Backslash => 11,
        T::Star | T::Slash => 12,
        T::Caret => 14,
        _ => return None,
    })
}
