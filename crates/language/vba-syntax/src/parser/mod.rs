//! The parser: cursor, tree-building helpers, diagnostics and module level.

mod decl;
mod expr;
mod stmt;

use bitflags::bitflags;
use vba_diagnostics::{Code, Diagnostic};
use vba_lexer::{Keyword, LexOptions, Token, TokenKind, lex};
use vba_profile::Profile;
use vba_source::{FileId, Span};

use crate::cc::{self, CcInfo, is_directive};
use crate::tree::{Builder, Checkpoint};
use crate::{NodeKind, SyntaxTree};

pub(crate) const UNEXPECTED: Code = Code("SYN001");
pub(crate) const EXPECTED: Code = Code("SYN002");
pub(crate) const MISSING_CLOSER: Code = Code("SYN003");
pub(crate) const MISPLACED: Code = Code("SYN004");
pub(crate) const DEVIATION: Code = Code("SYN005");
pub(crate) const VERSION_GATE: Code = Code("SYN010");

bitflags! {
    /// Block terminators, as a set per open construct.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) struct Term: u32 {
        const ENDIF = 1;
        const ELSE = 1 << 1;
        const ELSEIF = 1 << 2;
        const CASE = 1 << 3;
        const ENDSELECT = 1 << 4;
        const NEXT = 1 << 5;
        const LOOP = 1 << 6;
        const WEND = 1 << 7;
        const ENDWITH = 1 << 8;
        const ENDSUB = 1 << 9;
        const ENDFUNCTION = 1 << 10;
        const ENDPROPERTY = 1 << 11;
        const ENDTYPE = 1 << 12;
        const ENDENUM = 1 << 13;
        /// A procedure header at the start of a line: ends every open construct.
        const PROCSTART = 1 << 14;
        const ENDPROC = Self::ENDSUB.bits() | Self::ENDFUNCTION.bits() | Self::ENDPROPERTY.bits();
    }
}

/// Maximum nesting of blocks, statements and expressions; deeper input is
/// reported instead of risking stack exhaustion.
pub(crate) const MAX_DEPTH: u32 = 200;

/// Maximum number of operators, member accesses and call suffixes chained
/// onto one operand (`a + b + c + ...`, `a.b.c...`, `f(1)(2)...`). The tree
/// of such a chain is as deep as it is long and every later phase walks it
/// recursively, so a longer chain is reported instead of risking stack
/// exhaustion downstream. VBA itself cannot hold anything near this on a
/// logical line.
pub(crate) const MAX_CHAIN: u32 = 512;

/// Parser configuration.
#[derive(Debug, Clone, Default)]
pub struct ParseOptions {
    /// Strict mode: the lexer reports non-ASCII identifiers and the
    /// deviations that VBE tolerates (statements after a block `Else`,
    /// indented labels) are errors instead of warnings.
    /// [`Parse::into_result`] fails on any error either way.
    pub strict: bool,
    /// Extra project-level conditional compilation constants
    /// (`name`, value), consulted before the profile's predefined ones.
    pub constants: Vec<(String, i64)>,
}

impl ParseOptions {
    /// Tolerant options (for editors).
    pub fn tolerant() -> Self {
        Self::default()
    }

    /// Strict options.
    pub fn strict() -> Self {
        ParseOptions {
            strict: true,
            ..Self::default()
        }
    }

    /// Adds a project-level conditional compilation constant.
    #[must_use]
    pub fn with_constant(mut self, name: impl Into<String>, value: i64) -> Self {
        self.constants.push((name.into(), value));
        self
    }
}

/// The result of parsing: a tree that always covers the whole input, plus
/// diagnostics (lexer, conditional compilation and parser).
#[derive(Debug, Clone)]
pub struct Parse {
    /// The lossless tree.
    pub tree: SyntaxTree,
    /// Diagnostics in deterministic order.
    pub diagnostics: Vec<Diagnostic>,
}

impl Parse {
    /// Whether any diagnostic is an error.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == vba_diagnostics::Severity::Error)
    }

    /// The tree if there are no errors, else the diagnostics.
    pub fn into_result(self) -> Result<SyntaxTree, Vec<Diagnostic>> {
        if self.has_errors() {
            Err(self.diagnostics)
        } else {
            Ok(self.tree)
        }
    }
}

/// Parses a VBA module. Never panics; the tree's text always equals `src`.
pub fn parse(file: FileId, src: &[u8], profile: &Profile, opts: &ParseOptions) -> Parse {
    let mut lopts = LexOptions::for_profile(profile);
    if opts.strict {
        lopts = lopts.strict();
    }
    let lexed = lex(file, src, lopts);
    let cc = cc::analyze(file, &lexed.tokens, profile, &opts.constants);
    let sig: Vec<usize> = lexed
        .tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.kind.is_trivia())
        .map(|(i, _)| i)
        .collect();
    let mut p = Parser {
        toks: lexed.tokens,
        sig,
        si: 0,
        raw: 0,
        b: Builder::new(src),
        file,
        profile,
        strict: opts.strict,
        diags: Vec::new(),
        cc,
        frames: Vec::new(),
        pending_closes: 0,
        depth: 0,
        suppress: Vec::new(),
        last_err: None,
        end_offset: src.len() as u32,
        root: None,
    };
    p.parse_module();

    let mut suppress = p.suppress;
    for (&s, &e) in &p.cc.regions {
        if e > s {
            suppress.push(Span::new(p.toks[s].span.start, p.toks[e - 1].span.end));
        }
    }
    let mut diagnostics: Vec<Diagnostic> = lexed
        .diagnostics
        .into_iter()
        .filter(|d| {
            !suppress
                .iter()
                .any(|s| s.start <= d.span.start && d.span.end <= s.end)
        })
        .collect();
    diagnostics.extend(p.cc.diagnostics);
    diagnostics.extend(p.diags);
    let mut ds: vba_diagnostics::Diagnostics = diagnostics.into_iter().collect();
    ds.sort_and_dedup();
    let diagnostics = ds.into_sorted();
    let root = p.root.expect("root");
    Parse {
        tree: p.b.build(root),
        diagnostics,
    }
}

pub(crate) struct Parser<'a> {
    pub(crate) toks: Vec<Token<'a>>,
    /// Indices into `toks` of the non-trivia tokens.
    pub(crate) sig: Vec<usize>,
    /// Cursor into `sig`.
    pub(crate) si: usize,
    /// Next raw token not yet emitted into the tree.
    pub(crate) raw: usize,
    pub(crate) b: Builder,
    pub(crate) file: FileId,
    pub(crate) profile: &'a Profile,
    pub(crate) strict: bool,
    pub(crate) diags: Vec<Diagnostic>,
    pub(crate) cc: CcInfo,
    /// Terminator sets of the open constructs, innermost last.
    pub(crate) frames: Vec<Term>,
    /// Number of enclosing `For` loops closed by the same `Next i, j, ...`.
    pub(crate) pending_closes: usize,
    pub(crate) depth: u32,
    /// Spans whose lexer diagnostics are dropped (exported headers).
    pub(crate) suppress: Vec<Span>,
    last_err: Option<usize>,
    end_offset: u32,
    root: Option<crate::tree::NodeId>,
}

impl<'a> Parser<'a> {
    // ---- cursor ------------------------------------------------------

    pub(crate) fn kind(&self, n: usize) -> Option<TokenKind> {
        self.sig.get(self.si + n).map(|&i| self.toks[i].kind)
    }

    pub(crate) fn tok(&self, n: usize) -> Option<Token<'a>> {
        self.sig.get(self.si + n).map(|&i| self.toks[i])
    }

    pub(crate) fn cur(&self) -> Option<TokenKind> {
        self.kind(0)
    }

    pub(crate) fn at(&self, k: TokenKind) -> bool {
        self.cur() == Some(k)
    }

    pub(crate) fn at_kw(&self, k: Keyword) -> bool {
        self.cur() == Some(TokenKind::Keyword(k))
    }

    pub(crate) fn kw_at(&self, n: usize, k: Keyword) -> bool {
        self.kind(n) == Some(TokenKind::Keyword(k))
    }

    /// Whether the `n`th token is an identifier spelled `word`.
    pub(crate) fn word_at(&self, n: usize, word: &str) -> bool {
        self.tok(n).is_some_and(|t| {
            t.kind == TokenKind::Ident && t.text.eq_ignore_ascii_case(word.as_bytes())
        })
    }

    /// End of statement: newline, colon or end of input.
    pub(crate) fn at_eos(&self) -> bool {
        matches!(
            self.cur(),
            None | Some(TokenKind::Newline | TokenKind::Colon)
        )
    }

    /// Whether the current token is the first on its physical line.
    pub(crate) fn at_line_start(&self) -> bool {
        self.si == 0 || self.toks[self.sig[self.si - 1]].kind == TokenKind::Newline
    }

    /// Whether trivia directly precedes the current token.
    pub(crate) fn space_before(&self) -> bool {
        match self.sig.get(self.si) {
            Some(&i) => i > 0 && self.toks[i - 1].kind.is_trivia(),
            None => false,
        }
    }

    // ---- tree building -----------------------------------------------

    fn emit_trivia(&mut self) {
        let target = self.sig.get(self.si).copied().unwrap_or(self.toks.len());
        while self.raw < target {
            let t = self.toks[self.raw];
            self.b.token(t.kind, t.span);
            self.raw += 1;
        }
    }

    pub(crate) fn bump(&mut self) {
        if self.si >= self.sig.len() {
            return;
        }
        self.emit_trivia();
        let t = self.toks[self.sig[self.si]];
        self.b.token(t.kind, t.span);
        self.raw = self.sig[self.si] + 1;
        self.si += 1;
    }

    pub(crate) fn eat(&mut self, k: TokenKind) -> bool {
        if self.at(k) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(crate) fn eat_kw(&mut self, k: Keyword) -> bool {
        self.eat(TokenKind::Keyword(k))
    }

    pub(crate) fn start(&mut self, kind: NodeKind) {
        self.emit_trivia();
        self.b.start_node(kind);
    }

    pub(crate) fn finish(&mut self) {
        self.b.finish_node();
    }

    pub(crate) fn cp(&mut self) -> Checkpoint {
        self.emit_trivia();
        self.b.checkpoint()
    }

    pub(crate) fn start_at(&mut self, cp: Checkpoint, kind: NodeKind) {
        self.b.start_node_at(cp, kind);
    }

    // ---- diagnostics -------------------------------------------------

    pub(crate) fn cur_span(&self) -> Span {
        match self.tok(0) {
            Some(t) => t.span,
            None => Span::empty(self.end_offset),
        }
    }

    pub(crate) fn error(&mut self, code: Code, msg: impl Into<String>) {
        // Several constructs can end at the same token, each missing its closer.
        if self.last_err == Some(self.si) && code != MISSING_CLOSER {
            return;
        }
        self.last_err = Some(self.si);
        let span = self.cur_span();
        self.diags
            .push(Diagnostic::error(code, self.file, span, msg));
    }

    /// A deviation VBE tolerates: warning when tolerant, error when strict.
    pub(crate) fn soft(&mut self, code: Code, span: Span, msg: impl Into<String>) {
        let d = if self.strict {
            Diagnostic::error(code, self.file, span, msg)
        } else {
            Diagnostic::warning(code, self.file, span, msg)
        };
        self.diags.push(d);
    }

    pub(crate) fn describe_cur(&self) -> String {
        match self.tok(0) {
            Some(t) if t.kind == TokenKind::Newline => "end of line".into(),
            Some(t) => format!("`{}`", String::from_utf8_lossy(t.text)),
            None => "end of input".into(),
        }
    }

    pub(crate) fn expect(&mut self, k: TokenKind, what: &str) -> bool {
        if self.eat(k) {
            true
        } else {
            let d = self.describe_cur();
            self.error(EXPECTED, format!("expected {what}, found {d}"));
            false
        }
    }

    pub(crate) fn expect_kw(&mut self, k: Keyword, what: &str) -> bool {
        self.expect(TokenKind::Keyword(k), what)
    }

    // ---- conditional compilation ---------------------------------------

    /// Handles an inactive region or an effective directive at the cursor.
    /// Returns whether anything was consumed.
    pub(crate) fn cc_item(&mut self) -> bool {
        if let Some(&end) = self.cc.regions.get(&self.raw) {
            self.start(NodeKind::InactiveText);
            while self.raw < end {
                let t = self.toks[self.raw];
                self.b.token(t.kind, t.span);
                self.raw += 1;
            }
            self.finish();
            self.si = self.sig.partition_point(|&i| i < end);
            return true;
        }
        let Some(&idx) = self.sig.get(self.si) else {
            return false;
        };
        let k = self.toks[idx].kind;
        if !is_directive(k) {
            return false;
        }
        let Some(&value) = self.cc.directives.get(&idx) else {
            return false;
        };
        let kind = match k {
            TokenKind::HashIf => NodeKind::CcIf,
            TokenKind::HashElseIf => NodeKind::CcElseIf,
            TokenKind::HashElse => NodeKind::CcElse,
            TokenKind::HashEndIf => NodeKind::CcEndIf,
            _ => NodeKind::CcConst,
        };
        self.start(kind);
        while !matches!(self.cur(), None | Some(TokenKind::Newline)) {
            self.bump();
        }
        self.b.finish_node();
        let id = self.last_node_id();
        self.b.set_value(id, value);
        true
    }

    fn last_node_id(&self) -> crate::tree::NodeId {
        self.b.last_finished()
    }

    // ---- recovery ----------------------------------------------------

    /// Wraps the rest of the logical line in an `Error` node.
    pub(crate) fn recover_line(&mut self, stop_at_else: bool) {
        if self.at_eos() {
            return;
        }
        self.start(NodeKind::Error);
        while let Some(k) = self.cur() {
            if matches!(k, TokenKind::Newline | TokenKind::Colon) {
                break;
            }
            if stop_at_else && k == TokenKind::Keyword(Keyword::Else) {
                break;
            }
            self.bump();
        }
        self.finish();
    }

    /// Nesting limit reached: report once and skip the rest of the line.
    pub(crate) fn too_deep(&mut self) {
        self.error(MISPLACED, "nested too deeply");
        self.start(NodeKind::Error);
        while !self.at_eos() {
            self.bump();
        }
        self.finish();
    }

    /// Chain limit reached: report and skip the rest of the line.
    pub(crate) fn chain_too_long(&mut self) {
        self.error(MISPLACED, "expression is too long");
        self.start(NodeKind::Error);
        while !self.at_eos() {
            self.bump();
        }
        self.finish();
    }

    /// Reports "unexpected X" for the current token and skips the line.
    pub(crate) fn unexpected_line(&mut self, stop_at_else: bool) {
        if !self.cur().is_some_and(|k| k.is_error()) {
            let d = self.describe_cur();
            self.error(UNEXPECTED, format!("unexpected {d}"));
        }
        self.recover_line(stop_at_else);
    }

    /// After a statement: the line must end here. `mark` is the diagnostic
    /// count when the statement started; if it already reported something
    /// the trailing junk is skipped silently.
    pub(crate) fn end_of_stmt(&mut self, mark: usize, stop_at_else: bool) {
        // Block statements consume their trailing newline inside their body, so
        // being at the start of a line also ends the statement.
        if self.at_eos() || self.at_line_start() || (stop_at_else && self.at_kw(Keyword::Else)) {
            return;
        }
        if self.diags.len() == mark {
            let d = self.describe_cur();
            self.error(EXPECTED, format!("expected end of statement, found {d}"));
        }
        self.recover_line(stop_at_else);
    }

    /// Guarantees progress: consumes one token into an `Error` node.
    pub(crate) fn force_progress(&mut self) {
        if self.cur().is_none() {
            return;
        }
        let d = self.describe_cur();
        self.error(UNEXPECTED, format!("unexpected {d}"));
        self.start(NodeKind::Error);
        self.bump();
        self.finish();
    }

    // ---- terminators -----------------------------------------------

    /// Classifies the current token as a block terminator.
    pub(crate) fn term_at(&self) -> Option<Term> {
        use Keyword as K;
        let Some(TokenKind::Keyword(k)) = self.cur() else {
            return self.proc_start_at().then_some(Term::PROCSTART);
        };
        match k {
            K::Else => Some(Term::ELSE),
            K::ElseIf => Some(Term::ELSEIF),
            K::EndIf => Some(Term::ENDIF),
            K::Case => Some(Term::CASE),
            K::Next => Some(Term::NEXT),
            K::Loop => Some(Term::LOOP),
            K::Wend => Some(Term::WEND),
            K::End => match self.kind(1) {
                Some(TokenKind::Keyword(K::If)) => Some(Term::ENDIF),
                Some(TokenKind::Keyword(K::Select)) => Some(Term::ENDSELECT),
                Some(TokenKind::Keyword(K::With)) => Some(Term::ENDWITH),
                Some(TokenKind::Keyword(K::Sub)) => Some(Term::ENDSUB),
                Some(TokenKind::Keyword(K::Function)) => Some(Term::ENDFUNCTION),
                Some(TokenKind::Keyword(K::Type)) => Some(Term::ENDTYPE),
                Some(TokenKind::Keyword(K::Enum)) => Some(Term::ENDENUM),
                _ if self.word_at(1, "property") => Some(Term::ENDPROPERTY),
                _ => None,
            },
            _ => self.proc_start_at().then_some(Term::PROCSTART),
        }
    }

    /// Whether a procedure header starts at the current token (at a line
    /// start): `[Public|Private|Friend|Static]* Sub|Function|Property`.
    pub(crate) fn proc_start_at(&self) -> bool {
        if !self.at_line_start() {
            return false;
        }
        self.proc_header_ahead()
    }

    pub(crate) fn proc_header_ahead(&self) -> bool {
        let mut n = 0;
        while let Some(TokenKind::Keyword(
            Keyword::Public
            | Keyword::Private
            | Keyword::Friend
            | Keyword::Static
            | Keyword::Global,
        )) = self.kind(n)
        {
            n += 1;
        }
        match self.kind(n) {
            Some(TokenKind::Keyword(Keyword::Sub | Keyword::Function)) => true,
            Some(TokenKind::Ident) => {
                self.word_at(n, "property")
                    && matches!(
                        self.kind(n + 1),
                        Some(TokenKind::Keyword(
                            Keyword::Get | Keyword::Let | Keyword::Set
                        ))
                    )
            }
            _ => false,
        }
    }

    pub(crate) fn terminates(&self, t: Term) -> bool {
        if t == Term::PROCSTART {
            return !self.frames.is_empty();
        }
        self.frames.iter().any(|f| f.intersects(t))
    }

    // ---- module ------------------------------------------------------

    fn parse_module(&mut self) {
        self.b.start_node(NodeKind::Module);
        self.parse_header();
        loop {
            if self.cc_item() {
                continue;
            }
            match self.cur() {
                None => break,
                Some(TokenKind::Newline | TokenKind::Colon) => {
                    self.bump();
                    continue;
                }
                _ => {}
            }
            let before = self.si;
            let mark = self.diags.len();
            self.parse_item();
            self.end_of_stmt(mark, false);
            if self.si == before {
                self.force_progress();
            }
        }
        self.emit_trivia();
        while self.raw < self.toks.len() {
            let t = self.toks[self.raw];
            self.b.token(t.kind, t.span);
            self.raw += 1;
        }
        self.b.finish_node();
        self.root = Some(self.b.last_finished());
    }

    /// Exported-module header: `VERSION 1.0 CLASS` and a `BEGIN ... END`
    /// property block. Contents are kept as flat tokens.
    fn parse_header(&mut self) {
        while matches!(self.cur(), Some(TokenKind::Newline)) {
            self.bump();
        }
        if self.word_at(0, "version") && self.at_line_start() {
            self.start(NodeKind::VersionLine);
            while !matches!(self.cur(), None | Some(TokenKind::Newline)) {
                self.bump();
            }
            self.finish();
            while matches!(self.cur(), Some(TokenKind::Newline)) {
                self.bump();
            }
        }
        if self.word_at(0, "begin") && self.at_line_start() {
            let start_off = self.cur_span().start;
            self.start(NodeKind::HeaderBlock);
            let mut depth = 0i32;
            loop {
                // One physical line.
                let first_begin = self.word_at(0, "begin");
                let end_only = self.kw_at(0, Keyword::End)
                    && matches!(self.kind(1), None | Some(TokenKind::Newline));
                if first_begin {
                    depth += 1;
                } else if end_only {
                    depth -= 1;
                }
                while !matches!(self.cur(), None | Some(TokenKind::Newline)) {
                    self.bump();
                }
                if depth <= 0 || self.cur().is_none() {
                    break;
                }
                self.bump(); // newline
            }
            let end_off = self.toks[self.raw.saturating_sub(1).min(self.toks.len() - 1)]
                .span
                .end;
            self.finish();
            self.suppress.push(Span::new(start_off, end_off));
        }
    }

    fn parse_item(&mut self) {
        use Keyword as K;
        match self.cur() {
            Some(TokenKind::Keyword(k)) => match k {
                K::Option => self.parse_option(),
                K::Attribute => self.parse_attribute(),
                K::DefBool
                | K::DefByte
                | K::DefCur
                | K::DefDate
                | K::DefDbl
                | K::DefInt
                | K::DefLng
                | K::DefLngLng
                | K::DefLngPtr
                | K::DefObj
                | K::DefSng
                | K::DefStr
                | K::DefVar
                | K::DefDec => self.parse_def_type(),
                K::Implements => self.parse_implements(),
                K::Declare => self.parse_declare(),
                K::Event => self.parse_event(),
                K::Const => self.parse_const(),
                K::Type => self.parse_type(),
                K::Enum => self.parse_enum(),
                K::Dim | K::WithEvents => self.parse_var_decl(),
                K::Sub | K::Function => self.parse_proc(),
                K::Public | K::Private | K::Global | K::Friend | K::Static => {
                    self.parse_modified_item()
                }
                _ => self.misplaced_statement(),
            },
            Some(TokenKind::Ident) if self.word_at(0, "property") => self.parse_proc(),
            _ => self.misplaced_statement(),
        }
    }

    fn misplaced_statement(&mut self) {
        if self.cur().is_some_and(|k| k.is_error()) {
            self.recover_line(false);
            return;
        }
        let d = self.describe_cur();
        self.error(
            MISPLACED,
            format!("{d} is not a module-level declaration (executable statements belong in a procedure)"),
        );
        self.recover_line(false);
    }

    /// After `Public`/`Private`/`Global`/`Friend`/`Static`: pick the
    /// declaration kind from the first non-modifier token.
    fn parse_modified_item(&mut self) {
        use Keyword as K;
        let mut n = 0;
        while let Some(TokenKind::Keyword(
            K::Public | K::Private | K::Global | K::Friend | K::Static,
        )) = self.kind(n)
        {
            n += 1;
        }
        match self.kind(n) {
            Some(TokenKind::Keyword(K::Sub | K::Function)) => self.parse_proc(),
            Some(TokenKind::Ident) if self.word_at(n, "property") => self.parse_proc(),
            Some(TokenKind::Keyword(K::Declare)) => self.parse_declare(),
            Some(TokenKind::Keyword(K::Const)) => self.parse_const(),
            Some(TokenKind::Keyword(K::Type)) => self.parse_type(),
            Some(TokenKind::Keyword(K::Enum)) => self.parse_enum(),
            Some(TokenKind::Keyword(K::Event)) => self.parse_event(),
            _ => self.parse_var_decl(),
        }
    }
}
