//! Executable statements (MS-VBAL §5.4) and statement blocks.

use vba_lexer::{Keyword, TokenKind};

use super::{DEVIATION, EXPECTED, MISPLACED, MISSING_CLOSER, Parser, Term};
use crate::NodeKind;

impl Parser<'_> {
    /// A statement list ending at a terminator of any open construct.
    /// `bits` are the terminators the caller accepts.
    pub(crate) fn parse_block(&mut self, bits: Term) {
        if self.depth >= super::MAX_DEPTH {
            self.error(MISPLACED, "statements are nested too deeply");
            self.start(NodeKind::Block);
            self.finish();
            return;
        }
        self.depth += 1;
        self.frames.push(bits);
        self.start(NodeKind::Block);
        loop {
            if self.pending_closes > 0 {
                break;
            }
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
            if let Some(t) = self.term_at()
                && self.terminates(t)
            {
                break;
            }
            let before = self.si;
            let mark = self.diags.len();
            self.parse_statement();
            self.end_of_stmt(mark, false);
            if self.si == before {
                self.force_progress();
            }
        }
        self.finish();
        self.frames.pop();
        self.depth -= 1;
    }

    /// Consumes `End <kw>` / a single-word closer, or reports it missing.
    fn end_closer(&mut self, kw: Keyword, text: &str) {
        if self.at_kw(Keyword::End) && self.kw_at(1, kw) {
            self.bump();
            self.bump();
        } else {
            self.error(MISSING_CLOSER, format!("missing `{text}`"));
        }
    }

    fn label_token(&mut self) {
        if matches!(
            self.cur(),
            Some(TokenKind::Ident | TokenKind::ForeignName | TokenKind::IntegerLiteral)
        ) {
            self.bump();
        } else {
            let d = self.describe_cur();
            self.error(EXPECTED, format!("expected a label, found {d}"));
        }
    }

    /// One statement (§5.4), including an optional leading label.
    pub(crate) fn parse_statement(&mut self) {
        if self.depth >= super::MAX_DEPTH {
            self.too_deep();
            return;
        }
        self.depth += 1;
        self.parse_statement_inner();
        self.depth -= 1;
    }

    fn parse_statement_inner(&mut self) {
        use Keyword as K;
        if self.at_line_start() {
            let is_number = self.at(TokenKind::IntegerLiteral);
            let is_ident_label = matches!(self.cur(), Some(TokenKind::Ident))
                && self.kind(1) == Some(TokenKind::Colon);
            if is_number || is_ident_label {
                if is_ident_label && self.space_before() {
                    let span = self.cur_span();
                    self.soft(DEVIATION, span, "labels must start in the first column");
                }
                self.start(NodeKind::LabelStmt);
                self.bump();
                self.eat(TokenKind::Colon);
                self.finish();
                if self.at_eos() {
                    return;
                }
            }
        }
        // `Go To` and `Go Sub` are spelled as two words (MS-VBAL 5.4.2.12, 5.4.2.14).
        if self.at(TokenKind::Ident)
            && self
                .tok(0)
                .is_some_and(|t| t.text.eq_ignore_ascii_case(b"go"))
            && (self.kw_at(1, K::To) || self.kw_at(1, K::Sub))
        {
            let kind = if self.kw_at(1, K::To) {
                NodeKind::GoToStmt
            } else {
                NodeKind::GoSubStmt
            };
            self.start(kind);
            self.bump();
            self.bump();
            self.label_token();
            self.finish();
            return;
        }
        match self.cur() {
            Some(TokenKind::Keyword(k)) => match k {
                K::Dim | K::Static | K::Private | K::Public | K::Global => {
                    if matches!(k, K::Private | K::Public | K::Global) {
                        let span = self.cur_span();
                        self.diags.push(vba_diagnostics::Diagnostic::error(
                            MISPLACED,
                            self.file,
                            span,
                            "`Public`, `Private` and `Global` are not allowed inside a procedure",
                        ));
                    }
                    self.parse_var_decl();
                }
                K::Const => self.parse_const(),
                K::ReDim => self.parse_redim(),
                K::Erase => self.simple_list(NodeKind::EraseStmt),
                K::Set => self.parse_set(),
                K::Let | K::LSet | K::RSet => self.parse_keyword_assign(),
                K::Call => self.parse_call_kw(),
                K::If => self.parse_if(),
                K::Select => self.parse_select(),
                K::For => self.parse_for(),
                K::Do => self.parse_do(),
                K::While => self.parse_while(),
                K::With => self.parse_with(),
                K::GoTo => self.label_stmt(NodeKind::GoToStmt),
                K::GoSub => self.label_stmt(NodeKind::GoSubStmt),
                K::Return => self.bare_stmt(NodeKind::ReturnStmt),
                K::Stop => self.bare_stmt(NodeKind::StopStmt),
                K::On => self.parse_on(),
                K::Resume => self.parse_resume(),
                K::Exit => self.parse_exit(),
                K::End => {
                    if self.term_at().is_some() {
                        self.stray_terminator();
                    } else {
                        self.bare_stmt(NodeKind::EndStmt);
                    }
                }
                K::Else | K::ElseIf | K::EndIf | K::Case | K::Next | K::Loop | K::Wend => {
                    self.stray_terminator();
                }
                K::Open => self.parse_open(),
                K::Close => self.file_stmt(NodeKind::CloseStmt),
                K::Print => self.parse_print(NodeKind::PrintStmt),
                K::Write => self.parse_print(NodeKind::WriteStmt),
                K::Input => self.file_stmt(NodeKind::InputStmt),
                K::Get => self.file_stmt(NodeKind::GetStmt),
                K::Put => self.file_stmt(NodeKind::PutStmt),
                K::Seek => self.file_stmt(NodeKind::SeekStmt),
                K::Lock => self.file_stmt(NodeKind::LockStmt),
                K::Unlock => self.file_stmt(NodeKind::UnlockStmt),
                K::RaiseEvent => self.parse_raise_event(),
                K::Attribute => self.parse_attribute(),
                K::Debug
                    if self.kind(1) == Some(TokenKind::Dot)
                        && (self.kw_at(2, K::Print)
                            || self.kind(2) == Some(TokenKind::Question)) =>
                {
                    self.parse_debug_print();
                }
                K::Option
                | K::Declare
                | K::Type
                | K::Enum
                | K::Event
                | K::Implements
                | K::Sub
                | K::Function
                | K::Friend => {
                    let d = self.describe_cur();
                    self.error(MISPLACED, format!("{d} is not allowed inside a procedure"));
                    self.recover_line(false);
                }
                _ if self.at_expr_start() => self.expr_statement(),
                _ => self.unexpected_line(false),
            },
            Some(TokenKind::Ident) => {
                if self.word_at(0, "line") && self.kw_at(1, K::Input) {
                    self.start(NodeKind::LineInputStmt);
                    self.bump();
                    self.bump();
                    // MS-VBAL 5.4.5.6: the file number is marked.
                    if !self.at(TokenKind::Hash) {
                        self.error(EXPECTED, "expected `#` file number");
                    }
                    self.parse_arg_list_bare();
                    self.finish();
                } else if self.word_at(0, "width") && self.kind(1) == Some(TokenKind::Hash) {
                    self.file_stmt(NodeKind::WidthStmt);
                } else if self.word_at(0, "name") && self.looks_like_name_stmt() {
                    self.start(NodeKind::NameStmt);
                    self.bump();
                    self.parse_expr(0);
                    if self.expect_kw(K::As, "`As`") {
                        self.parse_expr(0);
                    }
                    self.finish();
                } else {
                    self.expr_statement();
                }
            }
            Some(k) if super::expr::starts_expr(k) => self.expr_statement(),
            _ => self.unexpected_line(false),
        }
    }

    fn stray_terminator(&mut self) {
        let d = self.describe_cur();
        self.error(
            MISPLACED,
            format!("{d} without a matching opening statement"),
        );
        self.recover_line(false);
    }

    /// `Name old As new`: an `As` at parenthesis depth 0 on the line.
    fn looks_like_name_stmt(&self) -> bool {
        if matches!(
            self.kind(1),
            None | Some(
                TokenKind::Eq
                    | TokenKind::Dot
                    | TokenKind::LParen
                    | TokenKind::Colon
                    | TokenKind::Newline
                    | TokenKind::Bang
            )
        ) {
            return false;
        }
        let mut depth = 0i32;
        let mut n = 1;
        while let Some(k) = self.kind(n) {
            match k {
                TokenKind::Newline | TokenKind::Colon => return false,
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => depth -= 1,
                TokenKind::Keyword(Keyword::As) if depth == 0 => return true,
                _ => {}
            }
            n += 1;
        }
        false
    }

    fn bare_stmt(&mut self, kind: NodeKind) {
        self.start(kind);
        self.bump();
        self.finish();
    }

    fn label_stmt(&mut self, kind: NodeKind) {
        self.start(kind);
        self.bump();
        self.label_token();
        self.finish();
    }

    /// `kw expr {, expr}`.
    fn simple_list(&mut self, kind: NodeKind) {
        self.start(kind);
        self.bump();
        loop {
            self.parse_expr(0);
            if self.at(TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.finish();
    }

    /// `kw [args]` with file-number style arguments.
    fn file_stmt(&mut self, kind: NodeKind) {
        self.start(kind);
        self.bump();
        if matches!(kind, NodeKind::LockStmt | NodeKind::UnlockStmt) {
            self.lock_args();
        } else if self.at_expr_start() || self.at(TokenKind::Comma) {
            self.parse_arg_list_bare();
        }
        self.finish();
    }

    /// `#file [, [start] [To end]]`: the record range of `Lock` and `Unlock`
    /// may omit its start (MS-VBAL 5.4.5.4).
    fn lock_args(&mut self) {
        self.start(NodeKind::ArgList);
        self.parse_arg();
        if self.at(TokenKind::Comma) {
            self.bump();
            self.start(NodeKind::Arg);
            if self.at_kw(Keyword::To) {
                self.start(NodeKind::RangeExpr);
                self.bump();
                self.parse_expr(0);
                self.finish();
            } else if self.at_expr_start() {
                self.parse_range_or_expr();
            }
            self.finish();
        }
        self.finish();
    }

    // ---- assignment and calls ------------------------------------------

    fn parse_set(&mut self) {
        self.start(NodeKind::SetStmt);
        self.bump();
        self.parse_postfix(false);
        if self.expect(TokenKind::Eq, "`=`") {
            self.parse_expr(0);
        }
        self.finish();
    }

    fn parse_keyword_assign(&mut self) {
        self.start(NodeKind::AssignStmt);
        self.bump();
        self.parse_postfix(false);
        if self.expect(TokenKind::Eq, "`=`") {
            self.parse_expr(0);
        }
        self.finish();
    }

    fn parse_call_kw(&mut self) {
        self.start(NodeKind::CallStmt);
        self.bump();
        self.parse_postfix(true);
        self.finish();
    }

    /// A statement starting with a name or member access: an assignment or
    /// an implicit call.
    fn expr_statement(&mut self) {
        let cp = self.cp();
        self.parse_postfix(false);
        if self.at(TokenKind::Eq) {
            self.start_at(cp, NodeKind::AssignStmt);
            self.bump();
            self.parse_expr(0);
            self.finish();
        } else {
            self.start_at(cp, NodeKind::CallStmt);
            if self.at_expr_start() || self.at(TokenKind::Comma) {
                self.parse_arg_list_bare();
            }
            self.finish();
        }
    }

    // ---- control flow ---------------------------------------------------

    fn parse_if(&mut self) {
        // Find `Then` on this line to decide between block and single-line.
        let mut n = 1;
        let mut then_at = None;
        while let Some(k) = self.kind(n) {
            match k {
                TokenKind::Newline => break,
                TokenKind::Keyword(Keyword::Then) => {
                    then_at = Some(n);
                    break;
                }
                _ => n += 1,
            }
        }
        let block = match then_at {
            None => true,
            Some(t) => matches!(self.kind(t + 1), None | Some(TokenKind::Newline)),
        };
        if block {
            self.parse_if_block();
        } else {
            self.parse_if_line();
        }
    }

    fn parse_if_block(&mut self) {
        self.start(NodeKind::IfStmt);
        self.bump();
        self.parse_expr(0);
        self.expect_kw(Keyword::Then, "`Then`");
        self.parse_block(Term::ENDIF | Term::ELSE | Term::ELSEIF);
        let mut seen_else = false;
        loop {
            if self.at_kw(Keyword::ElseIf) {
                self.start(NodeKind::ElseIfClause);
                self.bump();
                self.parse_expr(0);
                self.expect_kw(Keyword::Then, "`Then`");
                self.parse_block(Term::ENDIF | Term::ELSE | Term::ELSEIF);
                self.finish();
            } else if self.at_kw(Keyword::Else) {
                if seen_else {
                    self.error(MISPLACED, "duplicate `Else`");
                }
                seen_else = true;
                self.start(NodeKind::ElseClause);
                self.bump();
                if !self.at_eos() {
                    let span = self.cur_span();
                    self.soft(
                        DEVIATION,
                        span,
                        "statement on the same line as the `Else` of a block `If`",
                    );
                }
                self.parse_block(Term::ENDIF | Term::ELSE | Term::ELSEIF);
                self.finish();
            } else {
                break;
            }
        }
        if self.at_kw(Keyword::EndIf) {
            self.bump();
        } else {
            self.end_closer(Keyword::If, "End If");
        }
        self.finish();
    }

    fn parse_if_line(&mut self) {
        self.start(NodeKind::IfLineStmt);
        self.bump();
        self.parse_expr(0);
        self.expect_kw(Keyword::Then, "`Then`");
        self.parse_inline_block();
        if self.at_kw(Keyword::Else) {
            self.bump();
            self.parse_inline_block();
        }
        self.finish();
    }

    /// Statements of a single-line `If`, up to `Else` or the end of line.
    fn parse_inline_block(&mut self) {
        self.start(NodeKind::Block);
        loop {
            match self.cur() {
                None | Some(TokenKind::Newline) => break,
                Some(TokenKind::Keyword(Keyword::Else)) => break,
                Some(TokenKind::Colon) => {
                    self.bump();
                    continue;
                }
                _ => {}
            }
            let before = self.si;
            let mark = self.diags.len();
            if self.at(TokenKind::IntegerLiteral)
                && matches!(
                    self.kind(1),
                    None | Some(
                        TokenKind::Newline | TokenKind::Colon | TokenKind::Keyword(Keyword::Else)
                    )
                )
            {
                // `If c Then 100` means `GoTo 100`.
                self.start(NodeKind::GoToStmt);
                self.bump();
                self.finish();
            } else {
                self.parse_statement();
            }
            self.end_of_stmt(mark, true);
            if self.si == before {
                self.force_progress();
            }
        }
        self.finish();
    }

    fn parse_select(&mut self) {
        self.start(NodeKind::SelectStmt);
        self.bump();
        self.expect_kw(Keyword::Case, "`Case`");
        self.parse_expr(0);
        self.frames.push(Term::CASE | Term::ENDSELECT);
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
            if self.at_kw(Keyword::Case) {
                self.parse_case_clause();
                continue;
            }
            if let Some(t) = self.term_at()
                && self.terminates(t)
            {
                break;
            }
            let before = self.si;
            self.error(EXPECTED, "expected `Case`");
            self.recover_line(false);
            if self.si == before {
                self.force_progress();
            }
        }
        self.frames.pop();
        self.end_closer(Keyword::Select, "End Select");
        self.finish();
    }

    fn parse_case_clause(&mut self) {
        self.start(NodeKind::CaseClause);
        self.bump();
        if self.at_kw(Keyword::Else) {
            self.bump();
        } else {
            loop {
                self.parse_case_item();
                if self.at(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.parse_block(Term::CASE | Term::ENDSELECT);
        self.finish();
    }

    fn parse_case_item(&mut self) {
        // `Case < 5` without `Is` is the same comparison clause (MS-VBAL 5.4.2.10).
        let bare_operator = matches!(
            self.cur(),
            Some(
                TokenKind::Eq
                    | TokenKind::Ne
                    | TokenKind::Lt
                    | TokenKind::Gt
                    | TokenKind::Le
                    | TokenKind::Ge
            )
        );
        if self.at_kw(Keyword::Is) || bare_operator {
            self.start(NodeKind::CaseIsItem);
            if !bare_operator {
                self.bump();
            }
            if matches!(
                self.cur(),
                Some(
                    TokenKind::Eq
                        | TokenKind::Ne
                        | TokenKind::Lt
                        | TokenKind::Gt
                        | TokenKind::Le
                        | TokenKind::Ge
                )
            ) {
                self.bump();
            } else {
                self.error(EXPECTED, "expected a comparison operator after `Is`");
            }
            self.parse_expr(0);
            self.finish();
        } else {
            self.parse_range_or_expr();
        }
    }

    fn parse_for(&mut self) {
        let each = self.kw_at(1, Keyword::Each);
        self.start(if each {
            NodeKind::ForEachStmt
        } else {
            NodeKind::ForStmt
        });
        self.bump();
        if each {
            self.bump();
        }
        self.parse_postfix(true);
        if each {
            if self.expect_kw(Keyword::In, "`In`") {
                self.parse_expr(0);
            }
        } else if self.expect(TokenKind::Eq, "`=`") {
            self.parse_expr(0);
            if self.expect_kw(Keyword::To, "`To`") {
                self.parse_expr(0);
                if self.word_at(0, "step") {
                    self.bump();
                    self.parse_expr(0);
                }
            }
        }
        self.parse_block(Term::NEXT);
        if self.pending_closes > 0 {
            // Closed by an inner `Next i, j`.
            self.pending_closes -= 1;
        } else if self.at_kw(Keyword::Next) {
            self.bump();
            if self.at_expr_start() {
                let mut counters = 1;
                self.parse_postfix(true);
                while self.at(TokenKind::Comma) {
                    self.bump();
                    self.parse_postfix(true);
                    counters += 1;
                }
                self.pending_closes += counters - 1;
            }
        } else {
            self.error(MISSING_CLOSER, "missing `Next`");
        }
        self.finish();
    }

    fn parse_do(&mut self) {
        self.start(NodeKind::DoStmt);
        self.bump();
        let mut has_condition = false;
        if self.at_kw(Keyword::While) || self.at_kw(Keyword::Until) {
            has_condition = true;
            self.bump();
            self.parse_expr(0);
        }
        self.parse_block(Term::LOOP);
        if self.at_kw(Keyword::Loop) {
            self.bump();
            if self.at_kw(Keyword::While) || self.at_kw(Keyword::Until) {
                // MS-VBAL 5.4.2.6: a `Do` statement has a condition at the
                // top or at the bottom, not both.
                if has_condition {
                    self.error(
                        EXPECTED,
                        "a `Do` loop cannot have a condition on both `Do` and `Loop`",
                    );
                }
                self.bump();
                self.parse_expr(0);
            }
        } else {
            self.error(MISSING_CLOSER, "missing `Loop`");
        }
        self.finish();
    }

    fn parse_while(&mut self) {
        self.start(NodeKind::WhileStmt);
        self.bump();
        self.parse_expr(0);
        self.parse_block(Term::WEND);
        if !self.eat_kw(Keyword::Wend) {
            self.error(MISSING_CLOSER, "missing `Wend`");
        }
        self.finish();
    }

    fn parse_with(&mut self) {
        self.start(NodeKind::WithStmt);
        self.bump();
        self.parse_expr(0);
        self.parse_block(Term::ENDWITH);
        self.end_closer(Keyword::With, "End With");
        self.finish();
    }

    fn parse_on(&mut self) {
        if self.word_at(1, "error") {
            self.start(NodeKind::OnErrorStmt);
            self.bump();
            self.bump();
            if self.at_kw(Keyword::GoTo) {
                self.bump();
                self.eat(TokenKind::Minus);
                self.label_token();
            } else if self.at_kw(Keyword::Resume) {
                self.bump();
                self.expect_kw(Keyword::Next, "`Next`");
            } else {
                self.error(
                    EXPECTED,
                    "expected `GoTo` or `Resume Next` after `On Error`",
                );
            }
            self.finish();
        } else {
            self.start(NodeKind::OnGoToStmt);
            self.bump();
            self.parse_expr(0);
            if self.at_kw(Keyword::GoTo) || self.at_kw(Keyword::GoSub) {
                self.bump();
                loop {
                    self.label_token();
                    if self.at(TokenKind::Comma) {
                        self.bump();
                    } else {
                        break;
                    }
                }
            } else {
                self.error(EXPECTED, "expected `GoTo` or `GoSub`");
            }
            self.finish();
        }
    }

    fn parse_resume(&mut self) {
        self.start(NodeKind::ResumeStmt);
        self.bump();
        if self.at_kw(Keyword::Next)
            || matches!(
                self.cur(),
                Some(TokenKind::Ident | TokenKind::ForeignName | TokenKind::IntegerLiteral)
            )
        {
            self.bump();
        }
        self.finish();
    }

    fn parse_exit(&mut self) {
        self.start(NodeKind::ExitStmt);
        self.bump();
        if matches!(
            self.cur(),
            Some(TokenKind::Keyword(
                Keyword::Sub | Keyword::Function | Keyword::For | Keyword::Do
            ))
        ) || self.word_at(0, "property")
        {
            self.bump();
        } else {
            self.error(
                EXPECTED,
                "expected `Sub`, `Function`, `Property`, `For` or `Do`",
            );
        }
        self.finish();
    }

    fn parse_redim(&mut self) {
        self.start(NodeKind::ReDimStmt);
        self.bump();
        self.eat_kw(Keyword::Preserve);
        loop {
            self.start(NodeKind::ReDimItem);
            self.parse_postfix(true);
            if self.at_kw(Keyword::As) {
                self.parse_as_clause();
            }
            self.finish();
            if self.at(TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.finish();
    }

    // ---- file I/O -------------------------------------------------------

    fn parse_open(&mut self) {
        self.start(NodeKind::OpenStmt);
        self.bump();
        self.parse_expr(0);
        if self.at_kw(Keyword::For) {
            self.bump();
            if matches!(self.cur(), Some(TokenKind::Ident | TokenKind::Keyword(_)))
                && !self.at_kw(Keyword::As)
            {
                self.bump();
            } else {
                self.error(EXPECTED, "expected a file mode");
            }
        }
        // `Access ...`, `Shared`, `Lock Read|Write|Read Write`
        while !self.at_eos() && !self.at_kw(Keyword::As) {
            if matches!(self.cur(), Some(TokenKind::Ident | TokenKind::Keyword(_))) {
                self.bump();
            } else {
                break;
            }
        }
        if self.expect_kw(Keyword::As, "`As`") {
            self.parse_expr(0);
            if self.at_kw(Keyword::Len) {
                self.bump();
                if self.expect(TokenKind::Eq, "`=`") {
                    self.parse_expr(0);
                }
            }
        }
        self.finish();
    }

    /// `Print|Write [#n,] items`.
    fn parse_print(&mut self, kind: NodeKind) {
        self.start(kind);
        self.bump();
        if self.at(TokenKind::Hash) {
            self.parse_postfix(true);
            self.eat(TokenKind::Comma);
        } else {
            self.error(EXPECTED, "expected `#` file number");
        }
        self.parse_output_list();
        self.finish();
    }

    /// `Debug.Print|Debug.? [#n,] items` (MS-VBAL 5.4.5.8, revision 3.0).
    /// With a file number the statement is the file `Print` statement.
    fn parse_debug_print(&mut self) {
        if self.kind(3) == Some(TokenKind::Hash) {
            self.start(NodeKind::PrintStmt);
            for _ in 0..3 {
                self.bump();
            }
            self.parse_postfix(true);
            self.eat(TokenKind::Comma);
        } else {
            self.start(NodeKind::DebugPrintStmt);
            for _ in 0..3 {
                self.bump();
            }
        }
        self.parse_output_list();
        self.finish();
    }

    fn parse_output_list(&mut self) {
        self.start(NodeKind::OutputList);
        loop {
            if self.at(TokenKind::Semicolon) || self.at(TokenKind::Comma) {
                self.bump();
            } else if self.at_expr_start() {
                self.parse_expr(0);
            } else {
                break;
            }
        }
        self.finish();
    }

    fn parse_raise_event(&mut self) {
        self.start(NodeKind::RaiseEventStmt);
        self.bump();
        self.name_token("event name");
        if self.at(TokenKind::LParen) {
            self.parse_arg_list_paren();
        }
        self.finish();
    }
}
