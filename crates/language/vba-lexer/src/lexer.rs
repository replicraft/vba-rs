use vba_diagnostics::{Code, Diagnostic};
use vba_profile::{Feature, LanguageGeneration, Profile};
use vba_source::{FileId, Span};

use crate::date::{is_date_byte, is_date_or_time, is_wsc};
use crate::{Keyword, KeywordClass, Token, TokenKind, lookup_keyword};

// Diagnostic codes.
const LEX_UNTERMINATED_STRING: Code = Code("LEX001");
const LEX_BAD_DATE: Code = Code("LEX002");
const LEX_UNEXPECTED_BYTE: Code = Code("LEX003");
const LEX_BAD_FOREIGN_NAME: Code = Code("LEX004");
const LEX_NON_ASCII_IDENTIFIER: Code = Code("LEX005");

// ---- byte classes ---------------------------------------------------------

const ID_START: u8 = 1;
const ID_CONT: u8 = 2;
const DIGIT: u8 = 4;
const HEX: u8 = 8;

/// Byte class table, indexed by byte. Bytes of 0x80 and above are treated as
/// identifier characters (single-byte or multi-byte code-page letters).
const CLASS: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut i = 0usize;
    while i < 256 {
        let b = i as u8;
        let mut c = 0;
        if b.is_ascii_alphabetic() || b >= 0x80 {
            c |= ID_START | ID_CONT;
        }
        if b.is_ascii_digit() {
            c |= ID_CONT | DIGIT | HEX;
        }
        if b == b'_' {
            c |= ID_CONT;
        }
        if matches!(b, b'a'..=b'f' | b'A'..=b'F') {
            c |= HEX;
        }
        t[i] = c;
        i += 1;
    }
    t
};

#[inline]
fn is(b: u8, class: u8) -> bool {
    CLASS[b as usize] & class != 0
}

#[inline]
fn is_eol(b: u8) -> bool {
    b == b'\r' || b == b'\n'
}

/// Bytes that make a directly preceding `&`, `^` or `!` an operator rather
/// than a type suffix.
#[inline]
fn starts_operand(b: u8) -> bool {
    is(b, ID_CONT) || matches!(b, b'(' | b'"' | b'.' | b'[' | b'#')
}

// ---- options --------------------------------------------------------------

/// Lexer configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LexOptions {
    /// Language generation; decides which words are reserved.
    pub generation: LanguageGeneration,
    /// Strict mode additionally reports deviations from the specification
    /// that tolerant mode (used by editors) silently accepts:
    /// non-ASCII identifier characters.
    pub strict: bool,
}

impl LexOptions {
    /// Tolerant options for `generation`.
    pub const fn new(generation: LanguageGeneration) -> Self {
        LexOptions {
            generation,
            strict: false,
        }
    }

    /// Tolerant options for a target profile.
    pub const fn for_profile(profile: &Profile) -> Self {
        Self::new(profile.generation)
    }

    /// The same options with strict checking enabled.
    #[must_use]
    pub const fn strict(mut self) -> Self {
        self.strict = true;
        self
    }
}

impl Default for LexOptions {
    fn default() -> Self {
        Self::new(LanguageGeneration::Vba7_1)
    }
}

// ---- lexer ----------------------------------------------------------------

/// A lazy, lossless lexer over VBA source bytes.
///
/// Iterating yields tokens whose spans tile the input without gaps, so the
/// concatenated token texts equal the input. Malformed input never stops the
/// lexer: it yields an error token ([`TokenKind::is_error`]), records a
/// [`Diagnostic`], and continues. Diagnostics are available through
/// [`Lexer::diagnostics`] once the iteration is done.
///
/// The lexer is context free. Where the grammar needs context, it applies the
/// heuristics documented in `docs/language/lexer.md` (whether a `&` starts a radix
/// literal, whether `&`, `^` or `!` directly after a name is a type suffix).
#[derive(Debug)]
pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    file: FileId,
    opts: LexOptions,
    /// Whether the previous significant token can end an operand.
    prev_operand: bool,
    /// Whether the previous significant token was `.`.
    prev_dot: bool,
    /// Whether the previous significant token names a statement that takes a
    /// file number (`Print`, `Write`, `Get`, `Width`...): a following `#`
    /// followed by a file number is not a date literal.
    prev_file_stmt: bool,
    /// Whether the next name is in a declaring position (statement start,
    /// after a declaration keyword, or after the comma of a declaration list),
    /// where a directly following `&` is a type suffix even before `(`.
    decl_name_next: bool,
    /// Whether the current statement began with a declaration keyword.
    in_decl_stmt: bool,
    /// Whether the previous significant token was the word `Property`.
    prev_property: bool,
    diags: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    /// Creates a lexer for `src`.
    ///
    /// # Panics
    ///
    /// Panics if `src` is longer than `u32::MAX` bytes (spans are 32-bit).
    pub fn new(file: FileId, src: &'a [u8], opts: LexOptions) -> Self {
        assert!(u32::try_from(src.len()).is_ok(), "source larger than 4 GiB");
        Lexer {
            src,
            pos: 0,
            file,
            opts,
            prev_operand: false,
            prev_dot: false,
            prev_file_stmt: false,
            decl_name_next: true,
            in_decl_stmt: false,
            prev_property: false,
            diags: Vec::new(),
        }
    }

    /// Diagnostics reported so far, in source order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diags
    }

    /// Consumes the lexer, returning its diagnostics.
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diags
    }

    fn byte(&self, i: usize) -> Option<u8> {
        self.src.get(i).copied()
    }

    fn report(&mut self, code: Code, start: usize, end: usize, msg: impl Into<String>) {
        self.diags.push(Diagnostic::error(
            code,
            self.file,
            Span::new(start as u32, end as u32),
            msg,
        ));
    }

    fn after_terminator(&self, i: usize) -> usize {
        if self.src[i] == b'\r' && self.byte(i + 1) == Some(b'\n') {
            i + 2
        } else {
            i + 1
        }
    }

    fn skip_digits(&self, mut i: usize) -> usize {
        while i < self.src.len() && is(self.src[i], DIGIT) {
            i += 1;
        }
        i
    }

    fn scan(&mut self, start: usize) -> TokenKind {
        let b = self.src[start];
        let next = self.byte(start + 1);
        let one = |this: &mut Self, k: TokenKind| {
            this.pos = start + 1;
            k
        };
        let two = |this: &mut Self, k: TokenKind| {
            this.pos = start + 2;
            k
        };
        match b {
            b' ' | b'\t' | 0x19 => self.whitespace(start),
            b'\r' | b'\n' => {
                self.pos = self.after_terminator(start);
                TokenKind::Newline
            }
            b'\'' => {
                self.pos = self.comment_end(start + 1, start + 1);
                TokenKind::Comment
            }
            b'"' => self.string(start),
            b'0'..=b'9' => self.number(start),
            b'.' => match next {
                Some(d) if is(d, DIGIT) && !self.prev_operand => self.number(start),
                _ => one(self, TokenKind::Dot),
            },
            b'&' => self.ampersand(start),
            b'#' => self.hash(start),
            b'[' => self.foreign_name(start),
            b':' if next == Some(b'=') => two(self, TokenKind::ColonEq),
            b':' => one(self, TokenKind::Colon),
            b'<' => match next {
                Some(b'=') => two(self, TokenKind::Le),
                Some(b'>') => two(self, TokenKind::Ne),
                _ => one(self, TokenKind::Lt),
            },
            b'>' if next == Some(b'=') => two(self, TokenKind::Ge),
            // `><` is the same operator as `<>` (MS-VBAL 5.6.9.5.2).
            b'>' if next == Some(b'<') => two(self, TokenKind::Ne),
            b'>' => one(self, TokenKind::Gt),
            b'+' => one(self, TokenKind::Plus),
            b'-' => one(self, TokenKind::Minus),
            b'*' => one(self, TokenKind::Star),
            b'/' => one(self, TokenKind::Slash),
            b'\\' => one(self, TokenKind::Backslash),
            b'^' => one(self, TokenKind::Caret),
            // `=<` and `=>` are the same operators as `<=` and `>=`
            // (MS-VBAL 5.6.9.5.5, 5.6.9.5.6).
            b'=' if next == Some(b'<') => two(self, TokenKind::Le),
            b'=' if next == Some(b'>') => two(self, TokenKind::Ge),
            b'=' => one(self, TokenKind::Eq),
            b',' => one(self, TokenKind::Comma),
            b';' => one(self, TokenKind::Semicolon),
            b'!' => one(self, TokenKind::Bang),
            b'(' => one(self, TokenKind::LParen),
            b')' => one(self, TokenKind::RParen),
            b'?' => one(self, TokenKind::Question),
            c if is(c, ID_START) => self.word(start),
            c => {
                self.pos = start + 1;
                let shown = if c.is_ascii_graphic() {
                    format!("`{}`", c as char)
                } else {
                    format!("byte 0x{c:02X}")
                };
                self.report(
                    LEX_UNEXPECTED_BYTE,
                    start,
                    start + 1,
                    format!("unexpected character {shown}"),
                );
                TokenKind::Unknown
            }
        }
    }

    /// `1*WSC`, or `1*WSC "_" line-terminator` (§3.2.2).
    fn whitespace(&mut self, start: usize) -> TokenKind {
        let mut i = start;
        while i < self.src.len() && is_wsc(self.src[i]) {
            i += 1;
        }
        if self.byte(i) == Some(b'_') {
            let j = i + 1;
            if j >= self.src.len() {
                self.pos = j;
                return TokenKind::LineContinuation;
            }
            if is_eol(self.src[j]) {
                self.pos = self.after_terminator(j);
                return TokenKind::LineContinuation;
            }
        }
        self.pos = i;
        TokenKind::Whitespace
    }

    /// End of a comment body (§3.3.1 `comment-body`). `i` is where scanning
    /// starts and `line_start` the first byte after the comment introducer; a
    /// `WSC "_"` directly before a line terminator continues the comment on
    /// the next physical line.
    fn comment_end(&self, mut i: usize, mut line_start: usize) -> usize {
        let s = self.src;
        loop {
            while i < s.len() && !is_eol(s[i]) {
                i += 1;
            }
            if i >= s.len() {
                return i;
            }
            if i >= line_start + 2 && s[i - 1] == b'_' && is_wsc(s[i - 2]) {
                i = self.after_terminator(i);
                line_start = i;
                continue;
            }
            return i;
        }
    }

    /// `STRING` (§3.3.4): doubled quotes escape; ends at the closing quote,
    /// or, when unterminated, before the line terminator or before a
    /// trailing line continuation.
    fn string(&mut self, start: usize) -> TokenKind {
        let s = self.src;
        let mut i = start + 1;
        loop {
            match s.get(i) {
                Some(b'"') => {
                    if s.get(i + 1) == Some(&b'"') {
                        i += 2;
                    } else {
                        self.pos = i + 1;
                        return TokenKind::StringLiteral;
                    }
                }
                Some(&c) if is_eol(c) => break,
                Some(_) => i += 1,
                None => break,
            }
        }
        // Unterminated. A trailing `WSC "_"` is left for the continuation.
        let mut end = i;
        if end >= start + 3 && s[end - 1] == b'_' && is_wsc(s[end - 2]) {
            end -= 1;
            while end > start + 1 && is_wsc(s[end - 1]) {
                end -= 1;
            }
        }
        self.pos = end;
        self.report(
            LEX_UNTERMINATED_STRING,
            start,
            end,
            "unterminated string literal",
        );
        TokenKind::UnterminatedString
    }

    /// Optional `%`, `&` or `^` after an integer-like literal.
    fn integer_suffix(&self, i: usize) -> usize {
        match self.byte(i) {
            Some(b'%') => i + 1,
            Some(b'&') if !self.byte(i + 1).is_some_and(starts_operand) => i + 1,
            Some(b'^')
                if self.opts.generation.supports(Feature::LongLong)
                    && !self
                        .byte(i + 1)
                        .is_some_and(|b| starts_operand(b) || b == b'-' || b == b'+') =>
            {
                i + 1
            }
            _ => i,
        }
    }

    /// `INTEGER` and `FLOAT` decimal forms (§3.3.2).
    fn number(&mut self, start: usize) -> TokenKind {
        let s = self.src;
        let mut i = start;
        let mut float = false;
        if s[i] == b'.' {
            i = self.skip_digits(i + 1);
            float = true;
        } else {
            i = self.skip_digits(i);
            if self.byte(i) == Some(b'.') {
                i = self.skip_digits(i + 1);
                float = true;
            }
        }
        if matches!(self.byte(i), Some(b'D' | b'd' | b'E' | b'e')) {
            let mut j = i + 1;
            if matches!(self.byte(j), Some(b'+' | b'-')) {
                j += 1;
            }
            if self.byte(j).is_some_and(|b| is(b, DIGIT)) {
                i = self.skip_digits(j);
                float = true;
            }
        }
        if float {
            if matches!(self.byte(i), Some(b'!' | b'#' | b'@')) {
                i += 1;
            }
        } else {
            match self.byte(i) {
                Some(b'!' | b'#' | b'@') => {
                    i += 1;
                    float = true;
                }
                _ => i = self.integer_suffix(i),
            }
        }
        self.pos = i;
        if float {
            TokenKind::FloatLiteral
        } else {
            TokenKind::IntegerLiteral
        }
    }

    /// `&H...`, `&O...`, `&777` radix literals, otherwise the `&` operator.
    /// A radix literal is only recognised where no operand precedes.
    fn ampersand(&mut self, start: usize) -> TokenKind {
        self.pos = start + 1;
        if self.prev_operand {
            return TokenKind::Amp;
        }
        let digits_from = match self.byte(start + 1) {
            Some(b'h' | b'H') if self.byte(start + 2).is_some_and(|b| is(b, HEX)) => {
                let mut i = start + 2;
                while self.byte(i).is_some_and(|b| is(b, HEX)) {
                    i += 1;
                }
                self.pos = self.integer_suffix(i);
                return TokenKind::HexLiteral;
            }
            Some(b'o' | b'O') => start + 2,
            Some(b'0'..=b'7') => start + 1,
            _ => return TokenKind::Amp,
        };
        let mut i = digits_from;
        while matches!(self.byte(i), Some(b'0'..=b'7')) {
            i += 1;
        }
        if i == digits_from {
            return TokenKind::Amp;
        }
        self.pos = self.integer_suffix(i);
        TokenKind::OctalLiteral
    }

    /// Conditional compilation directives (§4), `DATE` literals (§3.3.3) and
    /// the plain `#` (file number marker).
    fn hash(&mut self, start: usize) -> TokenKind {
        let s = self.src;
        // Directive keywords.
        let mut j = start + 1;
        while matches!(self.byte(j), Some(b' ' | b'\t')) {
            j += 1;
        }
        let word_len = |from: usize| {
            s[from.min(s.len())..]
                .iter()
                .take_while(|b| b.is_ascii_alphabetic())
                .count()
        };
        let n = word_len(j);
        if n > 0 {
            let boundary = |end: usize| !s.get(end).is_some_and(|&b| is(b, ID_CONT));
            let mut lower = [0u8; 8];
            if n <= lower.len() {
                for (d, c) in lower.iter_mut().zip(&s[j..j + n]) {
                    *d = c.to_ascii_lowercase();
                }
                let end = j + n;
                let kind = match &lower[..n] {
                    b"if" => Some(TokenKind::HashIf),
                    b"elseif" => Some(TokenKind::HashElseIf),
                    b"else" => Some(TokenKind::HashElse),
                    b"const" => Some(TokenKind::HashConst),
                    b"endif" => Some(TokenKind::HashEndIf),
                    _ => None,
                };
                if let Some(k) = kind
                    && boundary(end)
                {
                    self.pos = end;
                    return k;
                }
                if &lower[..n] == b"end" {
                    let mut k = end;
                    while matches!(self.byte(k), Some(b' ' | b'\t')) {
                        k += 1;
                    }
                    if k > end
                        && word_len(k) == 2
                        && s[k..k + 2].eq_ignore_ascii_case(b"if")
                        && boundary(k + 2)
                    {
                        self.pos = k + 2;
                        return TokenKind::HashEndIf;
                    }
                }
            }
        }
        // `Print #1, 5#`: after a file statement, `#` followed by a file number
        // (digits or a name) and then a separator is the file-number marker,
        // even where the text up to a later `#` would read as a date
        // (`#1, 5#`). `Debug.Print #3/4#` is still a date literal.
        if self.prev_file_stmt {
            let mut k = start + 1;
            while self.byte(k).is_some_and(|b| is(b, ID_CONT)) {
                k += 1;
            }
            if k > start + 1 {
                while matches!(self.byte(k), Some(b' ' | b'\t')) {
                    k += 1;
                }
                if matches!(
                    self.byte(k),
                    None | Some(b',' | b';' | b'\r' | b'\n' | b':' | b'\'' | b')')
                ) {
                    self.pos = start + 1;
                    return TokenKind::Hash;
                }
            }
        }
        // Date literal.
        let mut i = start + 1;
        while i < s.len() && is_date_byte(s[i]) {
            i += 1;
        }
        let content = &s[start + 1..i];
        let looks_like_date =
            content.first().is_some_and(u8::is_ascii_digit) && content.contains(&b'/');
        if s.get(i) == Some(&b'#') {
            if is_date_or_time(content) {
                self.pos = i + 1;
                return TokenKind::DateLiteral;
            }
            if looks_like_date {
                self.pos = i + 1;
                self.report(LEX_BAD_DATE, start, i + 1, "malformed date literal");
                return TokenKind::BadDate;
            }
        } else if looks_like_date {
            let mut end = i;
            while end > start + 1 && is_wsc(s[end - 1]) {
                end -= 1;
            }
            self.pos = end;
            self.report(LEX_BAD_DATE, start, end, "unterminated date literal");
            return TokenKind::BadDate;
        }
        self.pos = start + 1;
        TokenKind::Hash
    }

    /// `FOREIGN-NAME = "[" 1*non-line-termination-character "]"` (§3.3.5.3).
    fn foreign_name(&mut self, start: usize) -> TokenKind {
        let s = self.src;
        let mut i = start + 1;
        while i < s.len() && !is_eol(s[i]) && s[i] != b']' {
            i += 1;
        }
        if s.get(i) == Some(&b']') {
            if i > start + 1 {
                self.pos = i + 1;
                return TokenKind::ForeignName;
            }
            self.pos = i + 1;
            self.report(LEX_BAD_FOREIGN_NAME, start, i + 1, "empty foreign name");
        } else {
            self.pos = i;
            self.report(
                LEX_BAD_FOREIGN_NAME,
                start,
                i,
                "unterminated foreign name; expected `]`",
            );
        }
        TokenKind::BadForeignName
    }

    /// Identifiers, keywords, `Rem` comments and type suffixes.
    fn word(&mut self, start: usize) -> TokenKind {
        let s = self.src;
        let mut i = start + 1;
        let mut ascii = is_ascii_byte(s[start]);
        while i < s.len() && is(s[i], ID_CONT) {
            ascii &= is_ascii_byte(s[i]);
            i += 1;
        }
        let text = &s[start..i];
        if !ascii && self.opts.strict {
            self.report(
                LEX_NON_ASCII_IDENTIFIER,
                start,
                i,
                "identifier contains non-ASCII characters; MS-VBAL §3.3.5 requires a Latin-identifier",
            );
        }
        if text.len() == 3 && text.eq_ignore_ascii_case(b"rem") && !self.prev_dot {
            self.pos = self.comment_end(i, i);
            return TokenKind::RemComment;
        }
        if ascii && let Some(k) = lookup_keyword(text, self.opts.generation) {
            let typed_function = matches!(
                k,
                Keyword::Date | Keyword::String | Keyword::Input | Keyword::InputB
            );
            if typed_function && self.byte(i) == Some(b'$') {
                self.pos = i + 1;
                return TokenKind::Ident;
            }
            self.pos = i;
            return TokenKind::Keyword(k);
        }
        self.pos = match self.byte(i) {
            Some(b'%' | b'#' | b'@' | b'$') => i + 1,
            Some(b'&')
                if !self.byte(i + 1).is_some_and(starts_operand)
                    || (self.decl_name_next && self.byte(i + 1) == Some(b'(')) =>
            {
                i + 1
            }
            Some(b'!')
                if !self
                    .byte(i + 1)
                    .is_some_and(|b| starts_operand(b) && b != b'#') =>
            {
                i + 1
            }
            Some(b'^')
                if self.opts.generation.supports(Feature::LongLong)
                    && !self
                        .byte(i + 1)
                        .is_some_and(|b| starts_operand(b) || b == b'-' || b == b'+') =>
            {
                i + 1
            }
            _ => i,
        };
        TokenKind::Ident
    }

    /// Tracks whether the next name is in a declaring position.
    fn update_decl_state(&mut self, kind: TokenKind, text: &[u8]) {
        let was_property = self.prev_property;
        self.prev_property = false;
        self.decl_name_next = match kind {
            TokenKind::Newline | TokenKind::Colon => {
                self.in_decl_stmt = false;
                true
            }
            TokenKind::Comma => self.in_decl_stmt,
            TokenKind::Keyword(k) => {
                if matches!(
                    k,
                    Keyword::Dim
                        | Keyword::Static
                        | Keyword::Const
                        | Keyword::Public
                        | Keyword::Private
                        | Keyword::Global
                        | Keyword::Friend
                ) {
                    self.in_decl_stmt = true;
                }
                matches!(
                    k,
                    Keyword::Dim
                        | Keyword::Static
                        | Keyword::Const
                        | Keyword::Public
                        | Keyword::Private
                        | Keyword::Global
                        | Keyword::Friend
                        | Keyword::Sub
                        | Keyword::Function
                        | Keyword::ByVal
                        | Keyword::ByRef
                        | Keyword::Optional
                        | Keyword::ParamArray
                        | Keyword::WithEvents
                        | Keyword::Shared
                ) || (was_property && matches!(k, Keyword::Get | Keyword::Let | Keyword::Set))
            }
            TokenKind::Ident => {
                self.prev_property = text.eq_ignore_ascii_case(b"property");
                false
            }
            _ => false,
        };
    }

    fn update_state(&mut self, kind: TokenKind) {
        if kind.is_trivia() {
            return;
        }
        self.prev_dot = kind == TokenKind::Dot;
        self.prev_operand = match kind {
            TokenKind::Ident
            | TokenKind::ForeignName
            | TokenKind::IntegerLiteral
            | TokenKind::HexLiteral
            | TokenKind::OctalLiteral
            | TokenKind::FloatLiteral
            | TokenKind::StringLiteral
            | TokenKind::DateLiteral
            | TokenKind::RParen => true,
            TokenKind::Keyword(k) => k == Keyword::Me || k.class() == KeywordClass::Literal,
            _ => false,
        };
    }
}

#[inline]
fn is_ascii_byte(b: u8) -> bool {
    b < 0x80
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        if self.pos >= self.src.len() {
            return None;
        }
        let start = self.pos;
        let kind = self.scan(start);
        if self.pos <= start {
            // Defensive: every token consumes at least one byte.
            self.pos = start + 1;
        }
        self.update_state(kind);
        if !kind.is_trivia() {
            self.update_decl_state(kind, &self.src[start..self.pos]);
        }
        self.prev_file_stmt = if kind.is_trivia() {
            self.prev_file_stmt
        } else {
            match kind {
                TokenKind::Keyword(k) => matches!(
                    k,
                    Keyword::Print
                        | Keyword::Write
                        | Keyword::Input
                        | Keyword::Get
                        | Keyword::Put
                        | Keyword::Seek
                        | Keyword::Open
                        | Keyword::Close
                        | Keyword::Lock
                        | Keyword::Unlock
                        | Keyword::As
                ),
                TokenKind::Ident => {
                    let t = &self.src[start..self.pos];
                    t.eq_ignore_ascii_case(b"width") || t.eq_ignore_ascii_case(b"reset")
                }
                _ => false,
            }
        };
        Some(Token {
            kind,
            span: Span::new(start as u32, self.pos as u32),
            text: &self.src[start..self.pos],
        })
    }
}

/// The result of [`lex`].
#[derive(Debug, Clone)]
pub struct LexOutput<'a> {
    /// All tokens including trivia; spans tile the input.
    pub tokens: Vec<Token<'a>>,
    /// Diagnostics in source order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Lexes `src` completely.
pub fn lex<'a>(file: FileId, src: &'a [u8], opts: LexOptions) -> LexOutput<'a> {
    let mut lexer = Lexer::new(file, src, opts);
    let tokens: Vec<Token<'a>> = lexer.by_ref().collect();
    LexOutput {
        tokens,
        diagnostics: lexer.into_diagnostics(),
    }
}
