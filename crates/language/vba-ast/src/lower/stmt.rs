//! Statement lowering (MS-VBAL §5.4).

use vba_lexer::{Keyword, TokenKind};
use vba_source::Span;
use vba_syntax::{NodeKind, SyntaxElement, SyntaxNode, SyntaxToken};

use super::expr::comparison_op;
use super::{AST_INCOMPLETE, Lowerer, ident, is_expr_kind, is_kw, is_name_token, nodes_of, toks};
use crate::ast::*;
use crate::name::Ident;

fn err_expr(span: Span) -> Expr {
    Expr {
        span,
        kind: ExprKind::Error,
    }
}

impl Lowerer<'_> {
    pub(crate) fn block(&mut self, n: SyntaxNode<'_>) -> Block {
        let stmts = n.child_nodes().map(|c| self.stmt(c)).collect();
        Block {
            span: n.span(),
            stmts,
        }
    }

    fn block_nth(&mut self, n: SyntaxNode<'_>, nth: usize) -> Block {
        match nodes_of(n, NodeKind::Block).get(nth) {
            Some(b) => self.block(*b),
            None => Block {
                span: Span::empty(n.span().end),
                stmts: Vec::new(),
            },
        }
    }

    fn label_ref(&mut self, t: Option<&SyntaxToken<'_>>, at: Span) -> LabelRef {
        match t {
            Some(t) if t.kind() == TokenKind::IntegerLiteral => {
                let text = t.text();
                let end = text
                    .iter()
                    .position(|b| !b.is_ascii_digit())
                    .unwrap_or(text.len());
                LabelRef::LineNumber {
                    span: t.span(),
                    value: std::str::from_utf8(&text[..end])
                        .ok()
                        .and_then(|s| s.parse().ok()),
                }
            }
            Some(t) if is_name_token(t.kind()) => LabelRef::Name(ident(*t)),
            _ => {
                self.report(AST_INCOMPLETE, at, "expected a label");
                LabelRef::Name(Ident::missing(Span::empty(at.end)))
            }
        }
    }

    /// The expression behind a `#n` file number.
    fn file_number(&mut self, e: Expr) -> Expr {
        match e.kind {
            ExprKind::FileNumber(inner) => *inner,
            _ => e,
        }
    }

    fn arg_values(&mut self, n: SyntaxNode<'_>) -> Vec<Option<Expr>> {
        match n.child_node(NodeKind::ArgList) {
            Some(l) => self.args(l).into_iter().map(|a| a.value).collect(),
            None => Vec::new(),
        }
    }

    fn items_of_output(&mut self, list: Option<SyntaxNode<'_>>) -> Vec<OutputItem> {
        let Some(list) = list else {
            return Vec::new();
        };
        list.children()
            .filter_map(|c| match c {
                SyntaxElement::Node(e) if is_expr_kind(e.kind()) => {
                    Some(OutputItem::Expr(self.expr(e)))
                }
                SyntaxElement::Token(t) => match t.kind() {
                    TokenKind::Semicolon => Some(OutputItem::Semicolon),
                    TokenKind::Comma => Some(OutputItem::Comma),
                    _ => None,
                },
                SyntaxElement::Node(_) => None,
            })
            .collect()
    }

    pub(crate) fn stmt(&mut self, n: SyntaxNode<'_>) -> Stmt {
        let span = n.span();
        let kind = self.stmt_kind(n);
        Stmt { span, kind }
    }

    fn stmt_kind(&mut self, n: SyntaxNode<'_>) -> StmtKind {
        let t = toks(n);
        let span = n.span();
        match n.kind() {
            NodeKind::LabelStmt => StmtKind::Label(LabelDef {
                span,
                label: self.label_ref(t.first(), span),
            }),
            NodeKind::VarDeclStmt => StmtKind::Dim(self.var_decl(n)),
            NodeKind::ConstStmt => StmtKind::Const(self.const_decl(n)),
            NodeKind::ReDimStmt => {
                let items = nodes_of(n, NodeKind::ReDimItem)
                    .into_iter()
                    .map(|i| self.redim_item(i))
                    .collect();
                StmtKind::ReDim {
                    preserve: t.iter().any(|x| is_kw(x, Keyword::Preserve)),
                    items,
                }
            }
            NodeKind::EraseStmt => StmtKind::Erase(
                super::expr_nodes(n)
                    .into_iter()
                    .map(|e| self.expr(e))
                    .collect(),
            ),
            NodeKind::AssignStmt => {
                let kind = match t.first().map(|x| x.kind()) {
                    Some(TokenKind::Keyword(Keyword::LSet)) => AssignKind::LSet,
                    Some(TokenKind::Keyword(Keyword::RSet)) => AssignKind::RSet,
                    _ => AssignKind::Let,
                };
                StmtKind::Assign {
                    kind,
                    target: self.expr_child(n, 0),
                    value: self.expr_child(n, 1),
                }
            }
            NodeKind::SetStmt => StmtKind::Set {
                target: self.expr_child(n, 0),
                value: self.expr_child(n, 1),
            },
            NodeKind::CallStmt => self.call_stmt(n),
            NodeKind::IfStmt => {
                let else_ifs = nodes_of(n, NodeKind::ElseIfClause)
                    .into_iter()
                    .map(|c| ElseIf {
                        span: c.span(),
                        cond: self.expr_child(c, 0),
                        body: self.block_nth(c, 0),
                    })
                    .collect();
                let else_body = n
                    .child_node(NodeKind::ElseClause)
                    .map(|c| self.block_nth(c, 0));
                StmtKind::If(Box::new(IfStmt {
                    cond: self.expr_child(n, 0),
                    then_body: self.block_nth(n, 0),
                    else_ifs,
                    else_body,
                    single_line: false,
                }))
            }
            NodeKind::IfLineStmt => {
                let has_else = t.iter().any(|x| is_kw(x, Keyword::Else));
                StmtKind::If(Box::new(IfStmt {
                    cond: self.expr_child(n, 0),
                    then_body: self.block_nth(n, 0),
                    else_ifs: Vec::new(),
                    else_body: has_else.then(|| self.block_nth(n, 1)),
                    single_line: true,
                }))
            }
            NodeKind::SelectStmt => {
                let cases = nodes_of(n, NodeKind::CaseClause)
                    .into_iter()
                    .map(|c| self.case_clause(c))
                    .collect();
                StmtKind::Select(Box::new(SelectStmt {
                    subject: self.expr_child(n, 0),
                    cases,
                }))
            }
            NodeKind::ForStmt => self.for_stmt(n),
            NodeKind::ForEachStmt => self.for_each_stmt(n),
            NodeKind::DoStmt => self.do_stmt(n),
            NodeKind::WhileStmt => StmtKind::While {
                cond: self.expr_child(n, 0),
                body: self.block_nth(n, 0),
            },
            NodeKind::WithStmt => StmtKind::With {
                object: self.expr_child(n, 0),
                body: self.block_nth(n, 0),
            },
            NodeKind::GoToStmt => {
                // The bare line number after `Then` has no `GoTo` token.
                // `Go To` is two words: the label is the last token.
                let lab = t.iter().rfind(|x| !is_kw(x, Keyword::GoTo));
                StmtKind::GoTo(self.label_ref(lab, span))
            }
            NodeKind::GoSubStmt => StmtKind::GoSub(self.label_ref(t.last(), span)),
            NodeKind::ReturnStmt => StmtKind::Return,
            NodeKind::OnErrorStmt => self.on_error(n),
            NodeKind::OnGoToStmt => {
                let is_gosub = t.iter().any(|x| is_kw(x, Keyword::GoSub));
                let after = t
                    .iter()
                    .position(|x| is_kw(x, Keyword::GoSub) || is_kw(x, Keyword::GoTo));
                let targets: Vec<LabelRef> = after
                    .map(|p| {
                        t[p + 1..]
                            .iter()
                            .filter(|x| x.kind() != TokenKind::Comma)
                            .map(|x| self.label_ref(Some(x), span))
                            .collect()
                    })
                    .unwrap_or_default();
                StmtKind::OnGoTo {
                    selector: self.expr_child(n, 0),
                    is_gosub,
                    targets,
                }
            }
            NodeKind::ResumeStmt => StmtKind::Resume(match t.get(1) {
                None => ResumeTarget::Same,
                Some(x) if is_kw(x, Keyword::Next) => ResumeTarget::Next,
                Some(x) => ResumeTarget::Label(self.label_ref(Some(x), span)),
            }),
            NodeKind::ExitStmt => match t.get(1).map(|x| x.kind()) {
                Some(TokenKind::Keyword(Keyword::Sub)) => StmtKind::Exit(ExitKind::Sub),
                Some(TokenKind::Keyword(Keyword::Function)) => StmtKind::Exit(ExitKind::Function),
                Some(TokenKind::Keyword(Keyword::For)) => StmtKind::Exit(ExitKind::For),
                Some(TokenKind::Keyword(Keyword::Do)) => StmtKind::Exit(ExitKind::Do),
                Some(TokenKind::Ident) => StmtKind::Exit(ExitKind::Property),
                _ => {
                    self.report(AST_INCOMPLETE, span, "expected an Exit target");
                    StmtKind::Error
                }
            },
            NodeKind::EndStmt => StmtKind::End,
            NodeKind::StopStmt => StmtKind::Stop,
            NodeKind::OpenStmt => StmtKind::Open(Box::new(self.open_stmt(n))),
            NodeKind::CloseStmt => {
                let files = self
                    .arg_values(n)
                    .into_iter()
                    .flatten()
                    .map(|e| self.file_number(e))
                    .collect();
                StmtKind::Close(files)
            }
            NodeKind::PrintStmt | NodeKind::WriteStmt => {
                let file = match super::expr_nodes(n).first() {
                    Some(f) => {
                        let e = self.expr(*f);
                        self.file_number(e)
                    }
                    None => {
                        self.report(AST_INCOMPLETE, span, "expected a file number");
                        err_expr(Span::empty(span.end))
                    }
                };
                let items = self.items_of_output(n.child_node(NodeKind::OutputList));
                if n.kind() == NodeKind::PrintStmt {
                    StmtKind::Print { file, items }
                } else {
                    StmtKind::Write { file, items }
                }
            }
            NodeKind::DebugPrintStmt => {
                StmtKind::DebugPrint(self.items_of_output(n.child_node(NodeKind::OutputList)))
            }
            NodeKind::InputStmt
            | NodeKind::LineInputStmt
            | NodeKind::GetStmt
            | NodeKind::PutStmt
            | NodeKind::SeekStmt
            | NodeKind::LockStmt
            | NodeKind::UnlockStmt
            | NodeKind::WidthStmt => self.file_stmt(n),
            NodeKind::NameStmt => StmtKind::Name {
                old: self.expr_child(n, 0),
                new: self.expr_child(n, 1),
            },
            NodeKind::RaiseEventStmt => {
                let name = match t.get(1) {
                    Some(x) if is_name_token(x.kind()) => ident(*x),
                    _ => {
                        self.report(AST_INCOMPLETE, span, "expected an event name");
                        Ident::missing(Span::empty(span.end))
                    }
                };
                let args = match n.child_node(NodeKind::ArgList) {
                    Some(l) => self.args(l),
                    None => Vec::new(),
                };
                StmtKind::RaiseEvent { name, args }
            }
            NodeKind::AttributeStmt => StmtKind::Attribute(self.attribute(n)),
            NodeKind::InactiveText => StmtKind::Inactive,
            NodeKind::Error => StmtKind::Error,
            _ => match self.directive(n) {
                Some(d) => StmtKind::Directive(d),
                None => {
                    self.report(AST_INCOMPLETE, span, "unexpected node in a statement list");
                    StmtKind::Error
                }
            },
        }
    }

    fn redim_item(&mut self, n: SyntaxNode<'_>) -> ReDimItem {
        let span = n.span();
        let target = self.expr_child(n, 0);
        let as_clause = self.as_clause_opt(n);
        // The parser reads `a(1 To n)` as a call; split it back.
        let (target, bounds) = match (target, expr_nodes_first(n)) {
            (
                Expr {
                    kind: ExprKind::Call { callee, .. },
                    ..
                },
                Some(call),
            ) => {
                let bounds = match call.child_node(NodeKind::ArgList) {
                    Some(l) => self.bounds(l),
                    None => Vec::new(),
                };
                (*callee, bounds)
            }
            (target, _) => {
                self.report(AST_INCOMPLETE, span, "expected array bounds");
                (target, Vec::new())
            }
        };
        ReDimItem {
            span,
            target,
            bounds,
            as_clause,
        }
    }

    fn call_stmt(&mut self, n: SyntaxNode<'_>) -> StmtKind {
        let t = toks(n);
        let explicit = t.first().is_some_and(|x| is_kw(x, Keyword::Call));
        let callee = self.expr_child(n, 0);
        let bare = n.child_node(NodeKind::ArgList).map(|l| self.args(l));
        match (callee, bare) {
            (
                Expr {
                    kind: ExprKind::Call { callee, args },
                    ..
                },
                None,
            ) => StmtKind::Call {
                callee: *callee,
                args,
                explicit,
                parenthesized: true,
            },
            (callee, args) => StmtKind::Call {
                callee,
                args: args.unwrap_or_default(),
                explicit,
                parenthesized: false,
            },
        }
    }

    fn case_clause(&mut self, c: SyntaxNode<'_>) -> CaseClause {
        let is_else = toks(c).iter().any(|x| is_kw(x, Keyword::Else));
        let mut items = Vec::new();
        for item in c.child_nodes() {
            match item.kind() {
                NodeKind::CaseIsItem => {
                    let op = toks(item)
                        .iter()
                        .find_map(|x| comparison_op(x.kind()))
                        .unwrap_or(BinaryOp::Eq);
                    items.push(CaseItem::Is {
                        op,
                        value: self.expr_child(item, 0),
                    });
                }
                NodeKind::RangeExpr => items.push(CaseItem::Range {
                    lower: self.expr_child(item, 0),
                    upper: self.expr_child(item, 1),
                }),
                k if is_expr_kind(k) => items.push(CaseItem::Value(self.expr(item))),
                _ => {}
            }
        }
        CaseClause {
            span: c.span(),
            items,
            is_else,
            body: self.block_nth(c, 0),
        }
    }

    /// Expressions before and after the body block of `For`/`For Each`.
    fn split_around_block<'t>(
        &mut self,
        n: SyntaxNode<'t>,
    ) -> (Vec<SyntaxNode<'t>>, Vec<SyntaxNode<'t>>) {
        let mut before = Vec::new();
        let mut after = Vec::new();
        let mut seen_block = false;
        for c in n.child_nodes() {
            if c.kind() == NodeKind::Block {
                seen_block = true;
            } else if is_expr_kind(c.kind()) {
                if seen_block {
                    after.push(c);
                } else {
                    before.push(c);
                }
            }
        }
        (before, after)
    }

    fn for_stmt(&mut self, n: SyntaxNode<'_>) -> StmtKind {
        let (head, tail) = self.split_around_block(n);
        let mut it = head.into_iter().map(|e| self.expr(e));
        let counter = it.next();
        let start = it.next();
        let end = it.next();
        let step = it.next();
        let missing = |s: &mut Self, e: Option<Expr>| match e {
            Some(e) => e,
            None => {
                s.report(AST_INCOMPLETE, n.span(), "incomplete `For` header");
                err_expr(Span::empty(n.span().end))
            }
        };
        let counter = missing(self, counter);
        let start = missing(self, start);
        let end = missing(self, end);
        let next_counters = tail.into_iter().map(|e| self.expr(e)).collect();
        StmtKind::For(Box::new(ForStmt {
            counter,
            start,
            end,
            step,
            body: self.block_nth(n, 0),
            next_counters,
        }))
    }

    fn for_each_stmt(&mut self, n: SyntaxNode<'_>) -> StmtKind {
        let (head, tail) = self.split_around_block(n);
        let mut it = head.into_iter().map(|e| self.expr(e));
        let item = it.next();
        let collection = it.next();
        let item = item.unwrap_or_else(|| err_expr(Span::empty(n.span().end)));
        let collection = collection.unwrap_or_else(|| err_expr(Span::empty(n.span().end)));
        if matches!(item.kind, ExprKind::Error) || matches!(collection.kind, ExprKind::Error) {
            self.report(AST_INCOMPLETE, n.span(), "incomplete `For Each` header");
        }
        let next_counters = tail.into_iter().map(|e| self.expr(e)).collect();
        StmtKind::ForEach(Box::new(ForEachStmt {
            item,
            collection,
            body: self.block_nth(n, 0),
            next_counters,
        }))
    }

    fn do_stmt(&mut self, n: SyntaxNode<'_>) -> StmtKind {
        let mut pre = None;
        let mut post = None;
        let mut seen_block = false;
        let mut pending: Option<LoopCondKind> = None;
        for c in n.children() {
            match c {
                SyntaxElement::Token(t) => match t.kind() {
                    TokenKind::Keyword(Keyword::While) => pending = Some(LoopCondKind::While),
                    TokenKind::Keyword(Keyword::Until) => pending = Some(LoopCondKind::Until),
                    _ => {}
                },
                SyntaxElement::Node(c) if c.kind() == NodeKind::Block => seen_block = true,
                SyntaxElement::Node(c) if is_expr_kind(c.kind()) => {
                    if let Some(kind) = pending.take() {
                        let cond = LoopCond {
                            kind,
                            expr: self.expr(c),
                        };
                        if seen_block {
                            post = Some(cond);
                        } else {
                            pre = Some(cond);
                        }
                    }
                }
                SyntaxElement::Node(_) => {}
            }
        }
        StmtKind::Do(Box::new(DoStmt {
            pre,
            post,
            body: self.block_nth(n, 0),
        }))
    }

    fn on_error(&mut self, n: SyntaxNode<'_>) -> StmtKind {
        let t = toks(n);
        let span = n.span();
        // [On, Error, GoTo, label...] or [On, Error, Resume, Next]
        match t.get(2).map(|x| x.kind()) {
            Some(TokenKind::Keyword(Keyword::Resume)) => {
                StmtKind::OnError(OnErrorAction::ResumeNext)
            }
            Some(TokenKind::Keyword(Keyword::GoTo)) => {
                let action = match (t.get(3), t.get(4)) {
                    (Some(m), Some(one)) if m.kind() == TokenKind::Minus && one.text() == b"1" => {
                        OnErrorAction::GoToMinusOne
                    }
                    (Some(zero), _)
                        if zero.kind() == TokenKind::IntegerLiteral && zero.text() == b"0" =>
                    {
                        OnErrorAction::GoToZero
                    }
                    (lab, _) => OnErrorAction::GoTo(self.label_ref(lab, span)),
                };
                StmtKind::OnError(action)
            }
            _ => {
                self.report(AST_INCOMPLETE, span, "incomplete `On Error` statement");
                StmtKind::Error
            }
        }
    }

    fn open_stmt(&mut self, n: SyntaxNode<'_>) -> OpenStmt {
        let t = toks(n);
        let exprs = super::expr_nodes(n);
        let path = self.expr_child(n, 0);
        let file = {
            let e = self.expr_child(n, 1);
            self.file_number(e)
        };
        let record_len = if exprs.len() > 2 {
            Some(self.expr_child(n, 2))
        } else {
            None
        };
        let for_pos = t.iter().position(|x| is_kw(x, Keyword::For));
        let mode = for_pos.and_then(|p| t.get(p + 1)).and_then(|m| {
            match m.text().to_ascii_lowercase().as_slice() {
                b"input" => Some(OpenMode::Input),
                b"output" => Some(OpenMode::Output),
                b"append" => Some(OpenMode::Append),
                b"binary" => Some(OpenMode::Binary),
                b"random" => Some(OpenMode::Random),
                _ => None,
            }
        });
        let as_pos = t
            .iter()
            .position(|x| is_kw(x, Keyword::As))
            .unwrap_or(t.len());
        let start = for_pos.map_or(1, |p| p + 2).min(as_pos);
        let words: Vec<Vec<u8>> = t[start..as_pos]
            .iter()
            .map(|x| x.text().to_ascii_lowercase())
            .collect();
        let rw = |from: usize| {
            let mut r = false;
            let mut w = false;
            for word in words.iter().skip(from) {
                match word.as_slice() {
                    b"read" => r = true,
                    b"write" => w = true,
                    _ => break,
                }
            }
            (r, w)
        };
        let access = words
            .iter()
            .position(|w| w == b"access")
            .and_then(|p| match rw(p + 1) {
                (true, true) => Some(OpenAccess::ReadWrite),
                (true, false) => Some(OpenAccess::Read),
                (false, true) => Some(OpenAccess::Write),
                _ => None,
            });
        let lock = if let Some(p) = words.iter().position(|w| w == b"lock") {
            match rw(p + 1) {
                (true, true) => Some(OpenLock::LockReadWrite),
                (true, false) => Some(OpenLock::LockRead),
                (false, true) => Some(OpenLock::LockWrite),
                _ => None,
            }
        } else if words.iter().any(|w| w == b"shared") {
            Some(OpenLock::Shared)
        } else {
            None
        };
        OpenStmt {
            path,
            mode,
            access,
            lock,
            file,
            record_len,
        }
    }

    /// `Input`, `Line Input`, `Get`, `Put`, `Seek`, `Lock`, `Unlock`,
    /// `Width`: a file number followed by comma separated operands.
    fn file_stmt(&mut self, n: SyntaxNode<'_>) -> StmtKind {
        let span = n.span();
        let args = match n.child_node(NodeKind::ArgList) {
            Some(l) => self.args(l),
            None => Vec::new(),
        };
        let mut values = args.into_iter().map(|a| a.value);
        let file = match values.next().flatten() {
            Some(e) => self.file_number(e),
            None => {
                self.report(AST_INCOMPLETE, span, "expected a file number");
                err_expr(Span::empty(span.end))
            }
        };
        let rest: Vec<Option<Expr>> = values.collect();
        let mut rest = rest.into_iter();
        match n.kind() {
            NodeKind::InputStmt => StmtKind::Input {
                file,
                targets: rest.flatten().collect(),
            },
            NodeKind::LineInputStmt => StmtKind::LineInput {
                file,
                target: rest
                    .next()
                    .flatten()
                    .unwrap_or_else(|| err_expr(Span::empty(span.end))),
            },
            NodeKind::GetStmt => StmtKind::Get {
                file,
                record: rest.next().flatten(),
                target: rest.next().flatten(),
            },
            NodeKind::PutStmt => StmtKind::Put {
                file,
                record: rest.next().flatten(),
                value: rest.next().flatten(),
            },
            NodeKind::SeekStmt => StmtKind::Seek {
                file,
                position: rest.next().flatten(),
            },
            NodeKind::WidthStmt => StmtKind::Width {
                file,
                width: rest.next().flatten(),
            },
            kind => {
                let (from, to) = match rest.next().flatten() {
                    Some(Expr {
                        kind: ExprKind::Range { lower, upper },
                        ..
                    }) => (Some(*lower), Some(*upper)),
                    other => (other, None),
                };
                if kind == NodeKind::LockStmt {
                    StmtKind::Lock { file, from, to }
                } else {
                    StmtKind::Unlock { file, from, to }
                }
            }
        }
    }
}

fn expr_nodes_first<'t>(n: SyntaxNode<'t>) -> Option<SyntaxNode<'t>> {
    super::expr_nodes(n).into_iter().next()
}
