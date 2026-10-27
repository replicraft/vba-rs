# Lexer (`vba-lexer`)

A hand-written, lossless lexer for MS-VBAL section 3. It uses a 256-entry byte
class table for identifier/digit/hex tests, scans with plain byte loops, and
returns zero-copy token slices. It has no lexer-generator dependency.

## Interface

```rust
let out = vba_lexer::lex(file_id, bytes, LexOptions::new(generation));
// out.tokens: Vec<Token { kind, span, text: &[u8] }>, out.diagnostics
```

`Lexer` is also a lazy `Iterator<Item = Token>`.

## Design decisions

- **Input and token text are bytes.** VBA source is stored in a Windows code
  page. Decoding at lex time would be lossy and would change offsets, so the
  lexer works on `&[u8]` and token text is `&[u8]`. Bytes of 0x80 and above are
  treated as identifier characters outside strings and comments (tolerant
  mode). Consequently NBSP/DBCS whitespace and Unicode Zs from the grammar's
  `WSC` are not recognised; only tab, space and 0x19 are.
- **Lossless.** Every byte belongs to exactly one token; whitespace,
  continuations and comments are trivia tokens (`TokenKind::is_trivia`).
  Concatenating token texts reproduces the input, which is checked by property
  tests over random bytes, token soups and adversarial inputs.
- **Error tolerance.** Malformed literals yield an error token
  (`UnterminatedString`, `BadDate`, `BadForeignName`, `Unknown`) plus a
  diagnostic (`LEX001`-`LEX004`) and lexing continues. `LexOptions::strict()`
  additionally reports non-ASCII identifier characters (`LEX005`); callers
  treat any error diagnostic as failure in strict compilation.
- **Keywords are gated by `LanguageGeneration`** (from `vba-profile`):
  `LongPtr`, `LongLong`, `PtrSafe`, `CLngPtr`, `CLngLng`, `DefLngPtr` and
  `DefLngLng` are identifiers before VBA 7.0; `Implements`, `Event`,
  `RaiseEvent`, `WithEvents` and `AddressOf` are identifiers in the Access 95
  profile. Keyword matching is case-insensitive and the original spelling is
  kept in `text`.
- **Context-free with documented heuristics.** The lexer does not know the
  statement being parsed, so:
  - `&H`, `&O` and `&777` start a radix literal only when the previous
    significant token cannot end an operand; otherwise `&` is the operator.
  - A `&`, `^` or `!` directly after a name or number is a type suffix unless
    the next byte starts an operand (`a&b`, `x^2`, `rs!Name` are operators).
    `^` is only a suffix where `LongLong` exists (VBA 7+). The exception is a
    declaring position (statement start, after `Dim`/`Const`/`Static`/`Public`/
    `Private`/`Sub`/`Function`/`Property Get`/`ByVal`/`ByRef`/`Optional`, or after
    a comma of a declaration), where `a&(` is a typed name followed by bounds.
  - `%`, `#`, `@`, `$` always attach to a non-keyword identifier. Keywords
    take no suffix except `Date$`, `String$`, `Input$`, `InputB$`, which become
    identifiers; this keeps `Print#1` as `Print`, `#`, `1`.
  - `#` starts a date only if the text up to the next `#` matches the spec's
    `date-or-time` shape, so `Print #1, x` stays a file-number marker.
  - `Rem` is a comment unless directly after `.`.

## Implemented (spec sections)

| Section | Production | Notes |
| --- | --- | --- |
| 3.2.1 | line terminators | CR, LF, CRLF as `Newline`; U+2028/2029 too when the source is valid UTF-8 |
| 3.2.2 | `line-continuation`, `WSC` | `1*WSC "_" terminator`, also at EOF; Unicode `Zs` spaces (U+00A0, U+1680, U+2000-U+200A, U+202F, U+205F, U+3000) count as `WSC` in UTF-8 sources |
| 3.3.1 | `special-token`, `comment-body`, `EOS` | `Colon`, `Newline`; comments continue over `WSC "_"` line ends, for `'` and `Rem` |
| 3.3.2 | `INTEGER`, `FLOAT` | decimal, `&H`, `&O`, `&`-octal, `%` `&` `^` `!` `#` `@` suffixes, `D`/`E` exponents |
| 3.3.3 | `DATE` | full `date-or-time` shape incl. month names and am/pm |
| 3.3.4 | `STRING` | doubled quotes; unterminated string is an error token |
| 3.3.5 | `lex-identifier` | Latin identifiers; bytes >= 0x80 accepted (flagged in strict mode); non-Latin forms of 3.3.5.1 are not implemented |
| 3.3.5.2 | `reserved-identifier` | all listed words plus `Rem`; `PtrSafe` as contextual keyword |
| 3.3.5.3 | `FOREIGN-NAME`, `TYPED-NAME` | `[...]`, type suffix accessor `Token::type_suffix` |
| 4 | `#If`, `#ElseIf`, `#Else`, `#End If`, `#Const` | directive tokens; `#EndIf` and whitespace after `#` are also accepted |

Operators and punctuation: `+ - * / \ ^ & = < > <= >= <> := , ; . ! # ( ) : ?`.

## Not implemented

- Unicode line terminators and space characters in sources that are not valid
  UTF-8 (the lexer is byte-based there; code-page text keeps its bytes). Literal
  range and date validity are checked in `vba-sema`/`vba-value`; see
  `docs/language/spec-coverage.md`.
- `Attribute` lines are lexed as ordinary tokens; there is no module-header
  handling.
- Non-Latin identifier grammars (CP932/936/949/950) beyond accepting high
  bytes.
- The `PtrSafe` word is reserved from VBA 7.0 by this implementation; the
  specification does not list it as a reserved identifier, so whether VBA7
  permits it as a variable name is unverified.
- Logical-line assembly, `#If` evaluation, and `Rem`/`:` statement-position
  rules belong to later stages.
- Benchmarks.
- Lexing is only tested against the specification and synthetic inputs, not
  against a corpus of real VBA modules; compatibility with the real VBE lexer
  is therefore unmeasured.
