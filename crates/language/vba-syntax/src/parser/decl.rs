//! Module-level declarations (MS-VBAL §4.2 module header, §5.2 module
//! declaration elements, §5.3 procedure declarations).

use vba_lexer::{Keyword, KeywordClass, TokenKind};
use vba_profile::{Bitness, Feature};

use super::{EXPECTED, MISPLACED, MISSING_CLOSER, Parser, Term, VERSION_GATE};
use crate::NodeKind;

fn is_modifier(k: Option<TokenKind>) -> bool {
    matches!(
        k,
        Some(TokenKind::Keyword(
            Keyword::Public
                | Keyword::Private
                | Keyword::Global
                | Keyword::Friend
                | Keyword::Static
        ))
    )
}

impl Parser<'_> {
    fn bump_modifiers(&mut self) {
        while is_modifier(self.cur()) {
            self.bump();
        }
    }

    /// Consumes a declared name (identifier or foreign name).
    pub(crate) fn name_token(&mut self, what: &str) -> bool {
        if matches!(self.cur(), Some(TokenKind::Ident | TokenKind::ForeignName)) {
            self.bump();
            true
        } else {
            let d = self.describe_cur();
            self.error(EXPECTED, format!("expected {what}, found {d}"));
            false
        }
    }

    /// `Option Explicit | Base 0|1 | Compare Binary|Text|Database |
    /// Private Module` (§5.2.1).
    pub(crate) fn parse_option(&mut self) {
        self.start(NodeKind::OptionStmt);
        self.bump();
        if self.word_at(0, "explicit") {
            self.bump();
        } else if self.word_at(0, "base") {
            self.bump();
            let ok = self.tok(0).is_some_and(|t| {
                t.kind == TokenKind::IntegerLiteral && matches!(t.text, b"0" | b"1")
            });
            if ok {
                self.bump();
            } else {
                self.error(EXPECTED, "expected `0` or `1` after `Option Base`");
            }
        } else if self.word_at(0, "compare") {
            self.bump();
            if self.word_at(0, "binary") || self.word_at(0, "text") || self.word_at(0, "database") {
                self.bump();
            } else {
                self.error(EXPECTED, "expected `Binary`, `Text` or `Database`");
            }
        } else if self.at_kw(Keyword::Private) {
            self.bump();
            if self.word_at(0, "module") {
                self.bump();
            } else {
                self.error(EXPECTED, "expected `Module` after `Option Private`");
            }
        } else {
            self.error(
                EXPECTED,
                "expected `Explicit`, `Base`, `Compare` or `Private` after `Option`",
            );
        }
        self.finish();
    }

    /// `Attribute [object.]name = value {, value}` (§4.2).
    pub(crate) fn parse_attribute(&mut self) {
        self.start(NodeKind::AttributeStmt);
        self.bump();
        let name_ok = |k: Option<TokenKind>| {
            matches!(
                k,
                Some(TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_))
            )
        };
        if name_ok(self.cur()) {
            self.bump();
            if self.at(TokenKind::Dot) {
                self.bump();
                if name_ok(self.cur()) {
                    self.bump();
                } else {
                    self.error(EXPECTED, "expected attribute name");
                }
            }
        } else {
            self.error(EXPECTED, "expected attribute name");
        }
        if self.expect(TokenKind::Eq, "`=`") {
            self.parse_expr(0);
            while self.at(TokenKind::Comma) {
                self.bump();
                self.parse_expr(0);
            }
        }
        self.finish();
    }

    /// `DefInt A-C, X` (§5.2.1.1).
    pub(crate) fn parse_def_type(&mut self) {
        self.start(NodeKind::DefTypeStmt);
        self.bump();
        loop {
            self.start(NodeKind::DefRange);
            let letter = |p: &mut Self| {
                let single = p.tok(0).is_some_and(|t| {
                    t.kind == TokenKind::Ident
                        && t.text.len() == 1
                        && t.text[0].is_ascii_alphabetic()
                });
                if single {
                    p.bump();
                } else {
                    p.error(EXPECTED, "expected a single letter");
                }
                single
            };
            if letter(self) && self.at(TokenKind::Minus) {
                self.bump();
                letter(self);
            }
            self.finish();
            if self.at(TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
            if self.at_eos() {
                break;
            }
        }
        self.finish();
    }

    pub(crate) fn parse_implements(&mut self) {
        self.start(NodeKind::ImplementsStmt);
        self.bump();
        self.parse_type_ref();
        self.finish();
    }

    /// `[vis] Declare [PtrSafe] Sub|Function name Lib "x" [Alias "y"]
    /// [(params)] [As type]` (§5.2.3.5).
    pub(crate) fn parse_declare(&mut self) {
        self.start(NodeKind::DeclareStmt);
        self.bump_modifiers();
        self.bump();
        let mut ptrsafe = false;
        if self.at_kw(Keyword::PtrSafe) {
            self.bump();
            ptrsafe = true;
        } else if self.word_at(0, "ptrsafe") {
            // The lexer only reserves `PtrSafe` from VBA 7.0.
            let span = self.cur_span();
            self.diags.push(vba_diagnostics::Diagnostic::error(
                VERSION_GATE,
                self.file,
                span,
                "`PtrSafe` requires VBA 7.0 or later",
            ));
            self.bump();
        }
        if !ptrsafe
            && self.profile.supports(Feature::PtrSafe)
            && self.profile.target.bitness == Bitness::B64
        {
            let span = self.cur_span();
            self.diags.push(vba_diagnostics::Diagnostic::error(
                VERSION_GATE,
                self.file,
                span,
                "`Declare` on a 64-bit VBA 7 target requires `PtrSafe`",
            ));
        }
        if matches!(
            self.cur(),
            Some(TokenKind::Keyword(Keyword::Sub | Keyword::Function))
        ) {
            self.bump();
        } else {
            self.error(EXPECTED, "expected `Sub` or `Function`");
        }
        self.name_token("procedure name");
        if self.word_at(0, "lib") {
            self.bump();
            if !self.eat(TokenKind::StringLiteral) {
                self.error(EXPECTED, "expected library name string");
            }
        } else {
            self.error(EXPECTED, "expected `Lib`");
        }
        if self.word_at(0, "alias") {
            self.bump();
            if !self.eat(TokenKind::StringLiteral) {
                self.error(EXPECTED, "expected alias string");
            }
        }
        if self.at(TokenKind::LParen) {
            self.parse_param_list();
        }
        if self.at_kw(Keyword::As) {
            self.parse_as_clause();
        }
        self.finish();
    }

    /// `[vis] Event name [(params)]`.
    pub(crate) fn parse_event(&mut self) {
        self.start(NodeKind::EventStmt);
        self.bump_modifiers();
        self.bump();
        self.name_token("event name");
        if self.at(TokenKind::LParen) {
            self.parse_param_list();
        }
        self.finish();
    }

    /// `[vis] Const name [As type] = expr {, ...}` (§5.2.3.2).
    pub(crate) fn parse_const(&mut self) {
        self.start(NodeKind::ConstStmt);
        self.bump_modifiers();
        self.bump();
        loop {
            self.start(NodeKind::ConstItem);
            self.name_token("constant name");
            if self.at_kw(Keyword::As) {
                self.parse_as_clause();
            }
            if self.expect(TokenKind::Eq, "`=`") {
                self.parse_expr(0);
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

    /// `Dim|Private|Public|Global|Static [Shared] [WithEvents] name ...`
    /// (§5.2.3.1, §5.4.3.1).
    pub(crate) fn parse_var_decl(&mut self) {
        self.start(NodeKind::VarDeclStmt);
        while matches!(
            self.cur(),
            Some(TokenKind::Keyword(
                Keyword::Dim
                    | Keyword::Private
                    | Keyword::Public
                    | Keyword::Global
                    | Keyword::Static
            ))
        ) {
            self.bump();
        }
        self.eat_kw(Keyword::Shared);
        self.eat_kw(Keyword::WithEvents);
        loop {
            self.parse_declarator(NodeKind::VarDeclarator);
            if self.at(TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.finish();
    }

    /// `name [(bounds)] [As [New] type]`.
    pub(crate) fn parse_declarator(&mut self, kind: NodeKind) {
        self.start(kind);
        if kind == NodeKind::TypeMember && matches!(self.cur(), Some(TokenKind::Keyword(_))) {
            self.bump();
        } else {
            self.name_token("variable name");
        }
        if self.at(TokenKind::LParen) {
            self.parse_arg_list_paren();
        }
        if self.at_kw(Keyword::As) {
            self.parse_as_clause();
        }
        self.finish();
    }

    /// `[vis] Type name <members> End Type` (§5.2.3.3).
    pub(crate) fn parse_type(&mut self) {
        self.start(NodeKind::TypeStmt);
        self.bump_modifiers();
        self.bump();
        self.name_token("type name");
        self.parse_members(Term::ENDTYPE, |p| {
            // A member may be named like a reserved word (`Date As Long`,
            // MS-VBAL §5.2.3.3 reserved-name-member-dcl).
            let reserved = matches!(p.cur(), Some(TokenKind::Keyword(_)))
                && matches!(p.kind(1), Some(TokenKind::LParen))
                || matches!(p.cur(), Some(TokenKind::Keyword(_)))
                    && p.kind(1) == Some(TokenKind::Keyword(Keyword::As));
            if matches!(p.cur(), Some(TokenKind::Ident | TokenKind::ForeignName)) || reserved {
                p.parse_declarator(NodeKind::TypeMember);
            } else {
                p.unexpected_line(false);
            }
        });
        self.closer(Keyword::Type, "End Type");
        self.finish();
    }

    /// `[vis] Enum name <members> End Enum` (§5.2.3.4).
    pub(crate) fn parse_enum(&mut self) {
        self.start(NodeKind::EnumStmt);
        self.bump_modifiers();
        self.bump();
        self.name_token("enum name");
        self.parse_members(Term::ENDENUM, |p| {
            if matches!(p.cur(), Some(TokenKind::Ident | TokenKind::ForeignName)) {
                p.start(NodeKind::EnumMember);
                p.bump();
                if p.at(TokenKind::Eq) {
                    p.bump();
                    p.parse_expr(0);
                }
                p.finish();
            } else {
                p.unexpected_line(false);
            }
        });
        self.closer(Keyword::Enum, "End Enum");
        self.finish();
    }

    fn parse_members(&mut self, term: Term, member: fn(&mut Self)) {
        self.frames.push(term);
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
            if let Some(t) = self.term_at()
                && self.terminates(t)
            {
                break;
            }
            let before = self.si;
            let mark = self.diags.len();
            member(self);
            self.end_of_stmt(mark, false);
            if self.si == before {
                self.force_progress();
            }
        }
        self.frames.pop();
    }

    /// Consumes `End <kw>` or reports it missing.
    fn closer(&mut self, kw: Keyword, text: &str) {
        if self.at_kw(Keyword::End) && self.kw_at(1, kw) {
            self.bump();
            self.bump();
        } else {
            self.error(MISSING_CLOSER, format!("missing `{text}`"));
        }
    }

    /// `[vis] [Static] Sub|Function|Property Get|Let|Set name [(params)]
    /// [As type] Block End Sub|Function|Property` (§5.3.1).
    pub(crate) fn parse_proc(&mut self) {
        self.start(NodeKind::ProcDecl);
        let mut leading_static = false;
        while is_modifier(self.cur()) {
            leading_static |= self.at_kw(Keyword::Static);
            self.bump();
        }
        let mut expect_end = Term::ENDPROC;
        if self.at_kw(Keyword::Sub) {
            self.bump();
            expect_end = Term::ENDSUB;
        } else if self.at_kw(Keyword::Function) {
            self.bump();
            expect_end = Term::ENDFUNCTION;
        } else {
            // `Property Get|Let|Set`
            self.bump();
            self.bump();
            expect_end &= Term::ENDPROPERTY;
        }
        self.name_token("procedure name");
        if self.at(TokenKind::LParen) {
            self.parse_param_list();
        }
        if self.at_kw(Keyword::As) {
            self.parse_as_clause();
        }
        // A trailing `Static` gives the locals module extent (§5.3.1.2).
        if self.at_kw(Keyword::Static) {
            if leading_static {
                self.error(
                    MISPLACED,
                    "a procedure cannot be `Static` both before and after its name",
                );
            }
            self.bump();
        }
        if !self.at_eos() {
            self.unexpected_line(false);
        }
        self.parse_block(Term::ENDPROC);
        // Closing `End Sub|Function|Property`.
        if self.at_kw(Keyword::End) && self.term_at().is_some_and(|t| t.intersects(Term::ENDPROC)) {
            let t = self.term_at().unwrap_or(Term::empty());
            if t != expect_end {
                self.error(MISPLACED, "closing `End` does not match the procedure kind");
            }
            self.bump();
            self.bump();
        } else {
            self.error(
                MISSING_CLOSER,
                "missing `End Sub`, `End Function` or `End Property`",
            );
        }
        self.finish();
    }

    /// `( Param {, Param} )`.
    pub(crate) fn parse_param_list(&mut self) {
        self.start(NodeKind::ParamList);
        self.bump();
        if !self.at(TokenKind::RParen) {
            loop {
                self.parse_param();
                if self.at(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "`)`");
        self.finish();
    }

    fn parse_param(&mut self) {
        self.start(NodeKind::Param);
        while matches!(
            self.cur(),
            Some(TokenKind::Keyword(
                Keyword::Optional | Keyword::ByVal | Keyword::ByRef | Keyword::ParamArray
            ))
        ) {
            self.bump();
        }
        self.name_token("parameter name");
        if self.at(TokenKind::LParen) && self.kind(1) == Some(TokenKind::RParen) {
            self.bump();
            self.bump();
        }
        if self.at_kw(Keyword::As) {
            self.parse_as_clause();
        }
        if self.at(TokenKind::Eq) {
            self.bump();
            self.parse_expr(0);
        }
        self.finish();
    }

    /// `As [New] type [* length]`.
    pub(crate) fn parse_as_clause(&mut self) {
        self.start(NodeKind::AsClause);
        self.bump();
        self.eat_kw(Keyword::New);
        self.parse_type_ref();
        // `As String()`: array return type of a function.
        if self.at(TokenKind::LParen) && self.kind(1) == Some(TokenKind::RParen) {
            self.bump();
            self.bump();
        }
        if self.at(TokenKind::Star) {
            self.bump();
            self.parse_postfix(true);
        }
        self.finish();
    }

    /// A built-in type keyword or a possibly qualified name.
    pub(crate) fn parse_type_ref(&mut self) {
        self.start(NodeKind::TypeRef);
        match self.tok(0) {
            Some(t) => match t.kind {
                TokenKind::Keyword(k)
                    if matches!(k.class(), KeywordClass::ReservedType)
                        || matches!(k, Keyword::Date | Keyword::String | Keyword::Any) =>
                {
                    if k == Keyword::LongLong && !self.profile.supports(Feature::LongLong) {
                        self.diags.push(vba_diagnostics::Diagnostic::error(
                            VERSION_GATE,
                            self.file,
                            t.span,
                            "`LongLong` requires a 64-bit VBA 7 target",
                        ));
                    }
                    self.bump();
                }
                TokenKind::Ident | TokenKind::ForeignName => {
                    if t.text.eq_ignore_ascii_case(b"longptr")
                        && !self.profile.supports(Feature::LongPtr)
                    {
                        self.diags.push(vba_diagnostics::Diagnostic::error(
                            VERSION_GATE,
                            self.file,
                            t.span,
                            "`LongPtr` requires VBA 7.0 or later",
                        ));
                    } else if t.text.eq_ignore_ascii_case(b"longlong")
                        && !self.profile.supports(Feature::LongLong)
                    {
                        self.diags.push(vba_diagnostics::Diagnostic::error(
                            VERSION_GATE,
                            self.file,
                            t.span,
                            "`LongLong` requires a 64-bit VBA 7 target",
                        ));
                    }
                    self.bump();
                    while self.at(TokenKind::Dot)
                        && matches!(
                            self.kind(1),
                            Some(TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_))
                        )
                    {
                        self.bump();
                        self.bump();
                    }
                }
                _ => {
                    let d = self.describe_cur();
                    self.error(EXPECTED, format!("expected a type name, found {d}"));
                }
            },
            None => self.error(EXPECTED, "expected a type name, found end of input"),
        }
        self.finish();
    }
}
