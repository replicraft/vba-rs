//! Statement binding (MS-VBAL §5.4).

use vba_ast::{
    AssignKind, BinaryOp, Block, CaseItem, ExitKind, Expr, ExprKind, IfStmt, LoopCondKind,
    OnErrorAction, OpenMode, OutputItem, ResumeTarget, Stmt, StmtKind,
};
use vba_source::Span;

use crate::bound::*;
use crate::codes;
use crate::program::{LabelKey, LocalInfo, LocalKind, Symbol};
use crate::types::*;

use super::call::{Ctx, Sig};
use super::{Binder, LoopKind, WithEntry};

impl Binder<'_, '_> {
    pub(crate) fn bind_block(&mut self, b: &Block) -> Vec<BStmt> {
        let mut out = Vec::new();
        for s in &b.stmts {
            if let Some(x) = self.bind_stmt(s) {
                out.push(x);
            }
        }
        out
    }

    fn place_is_fixed_array(&self, p: &BPlace) -> bool {
        match p {
            BPlace::Local(l) => self.locals[l.index()].fixed_bounds.is_some(),
            BPlace::Global(g) | BPlace::Field { global: g, .. } => {
                self.sema.program.globals[g.index()].fixed_bounds.is_some()
            }
            BPlace::UdtField { udt, index, .. } => self
                .sema
                .program
                .udt_fields(*udt)
                .get(*index as usize)
                .is_some_and(|f| f.fixed_bounds.is_some()),
            _ => false,
        }
    }

    fn stmt(&self, span: Span, kind: BStmtKind) -> Option<BStmt> {
        Some(BStmt { span, kind })
    }

    fn bind_stmt(&mut self, s: &Stmt) -> Option<BStmt> {
        let span = s.span;
        match &s.kind {
            StmtKind::Label(l) => {
                let key = match &l.label {
                    vba_ast::LabelRef::Name(i) => LabelKey::Name(i.name.clone()),
                    vba_ast::LabelRef::LineNumber { value: Some(v), .. } => LabelKey::Line(*v),
                    _ => return None,
                };
                let id = *self.label_map.get(&key)?;
                if self.labels[id.index()].span != l.span {
                    return None; // duplicate definition, already reported
                }
                self.stmt(span, BStmtKind::Label(id))
            }
            StmtKind::Dim(_) | StmtKind::Const(_) | StmtKind::Attribute(_) => None,
            StmtKind::Directive(_) | StmtKind::Inactive | StmtKind::Error => None,
            StmtKind::Assign {
                kind,
                target,
                value,
            } => self.bind_assign(*kind, target, value, span),
            StmtKind::Set { target, value } => self.bind_set(target, value, span),
            StmtKind::Call {
                callee,
                args,
                explicit,
                parenthesized,
            } => {
                let a = if args.is_empty() && !*explicit && !*parenthesized {
                    None
                } else {
                    Some(&args[..])
                };
                // MS-VBAL 5.4.2.9: a bare label name in a single-line `If`
                // acts as a `GoTo` to that label.
                if a.is_none()
                    && let ExprKind::Name(i) = &callee.kind
                    && let Some(&id) = self.label_map.get(&LabelKey::Name(i.name.clone()))
                {
                    return self.stmt(span, BStmtKind::GoTo(id));
                }
                let e = self.bind_ref(callee, a, true);
                if e.ty.is_error() {
                    return None;
                }
                match e.kind {
                    BExprKind::Call(_) | BExprKind::Late(_) => self.stmt(span, BStmtKind::Call(e)),
                    _ => {
                        self.err(
                            codes::NOT_CALLABLE,
                            span,
                            "this is not a procedure call: a Sub or Function was expected",
                        );
                        None
                    }
                }
            }
            StmtKind::If(i) => self.bind_if(i, span),
            StmtKind::Select(sel) => {
                let subject = self.bind_expr(&sel.subject);
                let subject = self.rvalue(subject);
                if matches!(
                    subject.ty,
                    Type::Udt(_) | Type::LibRecord(_) | Type::Array { .. } | Type::Nothing
                ) {
                    self.err(
                        codes::TYPE_MISMATCH,
                        subject.span,
                        format!("cannot `Select Case` on {}", self.tn(&subject.ty)),
                    );
                }
                let mut cases = Vec::new();
                for c in &sel.cases {
                    let mut items = Vec::new();
                    for it in &c.items {
                        match it {
                            CaseItem::Value(e) => {
                                let b = self.bind_expr(e);
                                items.push(BCaseItem::Value(self.rvalue(b)));
                            }
                            CaseItem::Range { lower, upper } => {
                                let l = self.bind_expr(lower);
                                let l = self.rvalue(l);
                                let u = self.bind_expr(upper);
                                let u = self.rvalue(u);
                                items.push(BCaseItem::Range(l, u));
                            }
                            CaseItem::Is { op, value } => {
                                if !matches!(
                                    op,
                                    BinaryOp::Eq
                                        | BinaryOp::Ne
                                        | BinaryOp::Lt
                                        | BinaryOp::Gt
                                        | BinaryOp::Le
                                        | BinaryOp::Ge
                                ) {
                                    self.err(
                                        codes::TYPE_MISMATCH,
                                        value.span,
                                        "`Case Is` needs a comparison operator",
                                    );
                                }
                                let b = self.bind_expr(value);
                                items.push(BCaseItem::Is(*op, self.rvalue(b)));
                            }
                        }
                    }
                    let body = self.bind_block(&c.body);
                    cases.push(BCase {
                        items,
                        is_else: c.is_else,
                        body,
                    });
                }
                self.stmt(span, BStmtKind::Select { subject, cases })
            }
            StmtKind::For(f) => self.bind_for(f, span),
            StmtKind::ForEach(f) => self.bind_for_each(f, span),
            StmtKind::Do(d) => {
                let pre = d.pre.as_ref().map(|c| {
                    let e = self.bind_expr(&c.expr);
                    (c.kind == LoopCondKind::Until, self.coerce_condition(e))
                });
                self.loops.push(LoopKind::Do);
                let body = self.bind_block(&d.body);
                self.loops.pop();
                let post = d.post.as_ref().map(|c| {
                    let e = self.bind_expr(&c.expr);
                    (c.kind == LoopCondKind::Until, self.coerce_condition(e))
                });
                self.stmt(
                    span,
                    BStmtKind::Do {
                        pre,
                        post,
                        body,
                        is_while: false,
                    },
                )
            }
            StmtKind::While { cond, body } => {
                let c = self.bind_expr(cond);
                let c = self.coerce_condition(c);
                self.loops.push(LoopKind::While);
                let body = self.bind_block(body);
                self.loops.pop();
                self.stmt(
                    span,
                    BStmtKind::Do {
                        pre: Some((false, c)),
                        post: None,
                        body,
                        is_while: true,
                    },
                )
            }
            StmtKind::With { object, body } => self.bind_with(object, body, span),
            StmtKind::GoTo(l) => {
                let id = self.label_id(l)?;
                self.stmt(span, BStmtKind::GoTo(id))
            }
            StmtKind::GoSub(l) => {
                let id = self.label_id(l)?;
                self.stmt(span, BStmtKind::GoSub(id))
            }
            StmtKind::Return => self.stmt(span, BStmtKind::Return),
            StmtKind::OnError(a) => {
                let act = match a {
                    OnErrorAction::GoTo(l) => BOnError::GoTo(self.label_id(l)?),
                    OnErrorAction::GoToZero => BOnError::Disable,
                    OnErrorAction::GoToMinusOne => BOnError::ClearError,
                    OnErrorAction::ResumeNext => BOnError::ResumeNext,
                };
                self.stmt(span, BStmtKind::OnError(act))
            }
            StmtKind::OnGoTo {
                selector,
                is_gosub,
                targets,
            } => {
                let sel = self.bind_expr(selector);
                let sel = self.coerce_index(sel, Type::Long);
                let mut ids = Vec::new();
                for t in targets {
                    ids.push(self.label_id(t)?);
                }
                self.stmt(
                    span,
                    BStmtKind::OnGoTo {
                        selector: sel,
                        targets: ids,
                        is_gosub: *is_gosub,
                    },
                )
            }
            StmtKind::Resume(t) => {
                let r = match t {
                    ResumeTarget::Same => BResume::Same,
                    ResumeTarget::Next => BResume::Next,
                    ResumeTarget::Label(l) => BResume::Label(self.label_id(l)?),
                };
                self.stmt(span, BStmtKind::Resume(r))
            }
            StmtKind::Exit(k) => {
                if !self.exit_allowed(*k) {
                    let what = match k {
                        ExitKind::Sub => "`Exit Sub` is only valid in a Sub",
                        ExitKind::Function => "`Exit Function` is only valid in a Function",
                        ExitKind::Property => "`Exit Property` is only valid in a Property",
                        ExitKind::For => "`Exit For` is not inside a `For` loop",
                        ExitKind::Do => "`Exit Do` is not inside a `Do` loop",
                    };
                    self.err(codes::EXIT_MISUSE, span, what);
                    return None;
                }
                self.stmt(span, BStmtKind::Exit(*k))
            }
            StmtKind::End => self.stmt(span, BStmtKind::End),
            StmtKind::Stop => self.stmt(span, BStmtKind::Stop),
            StmtKind::ReDim { preserve, items } => self.bind_redim(*preserve, items, span),
            StmtKind::Erase(targets) => {
                let mut ps = Vec::new();
                for t in targets {
                    let b = self.bind_expr(t);
                    if b.ty.is_error() {
                        continue;
                    }
                    if !(b.ty.is_array() || b.ty.is_variant()) {
                        self.err(codes::ARRAY_RULE, t.span, "`Erase` needs an array variable");
                        continue;
                    }
                    match self.expr_to_place(&b) {
                        Some(p) => ps.push(p),
                        None => {
                            self.err(codes::ARRAY_RULE, t.span, "`Erase` needs an array variable")
                        }
                    }
                }
                self.stmt(span, BStmtKind::Erase(ps))
            }
            StmtKind::Open(o) => {
                let path = self.string_arg(&o.path);
                let file = self.file_num(&o.file);
                let record_len = o.record_len.as_ref().map(|e| self.int_arg(e));
                self.stmt(
                    span,
                    BStmtKind::Io(IoStmt::Open {
                        path,
                        mode: o.mode.unwrap_or(OpenMode::Random),
                        access: o.access,
                        lock: o.lock,
                        file,
                        record_len,
                    }),
                )
            }
            StmtKind::Close(files) => {
                let fs = files.iter().map(|e| self.file_num(e)).collect();
                self.stmt(span, BStmtKind::Io(IoStmt::Close(fs)))
            }
            StmtKind::Print { file, items } => {
                let f = self.file_num(file);
                let items = self.out_items(items);
                self.stmt(span, BStmtKind::Io(IoStmt::Print { file: f, items }))
            }
            StmtKind::Write { file, items } => {
                let f = self.file_num(file);
                let items = self.out_items(items);
                self.stmt(span, BStmtKind::Io(IoStmt::Write { file: f, items }))
            }
            StmtKind::Input { file, targets } => {
                let f = self.file_num(file);
                let mut ps = Vec::new();
                for t in targets {
                    if let Some((p, _)) = self.bind_target(t, false) {
                        ps.push(p);
                    }
                }
                self.stmt(
                    span,
                    BStmtKind::Io(IoStmt::Input {
                        file: f,
                        targets: ps,
                    }),
                )
            }
            StmtKind::LineInput { file, target } => {
                let f = self.file_num(file);
                let (p, _) = self.bind_target(target, false)?;
                self.stmt(
                    span,
                    BStmtKind::Io(IoStmt::LineInput { file: f, target: p }),
                )
            }
            StmtKind::Get {
                file,
                record,
                target,
            } => {
                let f = self.file_num(file);
                let r = record.as_ref().map(|e| self.int_arg(e));
                let t = match target {
                    Some(t) => Some(self.bind_target(t, false)?.0),
                    None => None,
                };
                self.stmt(
                    span,
                    BStmtKind::Io(IoStmt::Get {
                        file: f,
                        record: r,
                        target: t,
                    }),
                )
            }
            StmtKind::Put {
                file,
                record,
                value,
            } => {
                let f = self.file_num(file);
                let r = record.as_ref().map(|e| self.int_arg(e));
                let v = value.as_ref().map(|e| {
                    let b = self.bind_expr(e);
                    self.rvalue(b)
                });
                self.stmt(
                    span,
                    BStmtKind::Io(IoStmt::Put {
                        file: f,
                        record: r,
                        value: v,
                    }),
                )
            }
            StmtKind::Seek { file, position } => {
                let f = self.file_num(file);
                let p = position.as_ref().map(|e| self.int_arg(e));
                self.stmt(
                    span,
                    BStmtKind::Io(IoStmt::Seek {
                        file: f,
                        position: p,
                    }),
                )
            }
            StmtKind::Lock { file, from, to } => {
                let f = self.file_num(file);
                let a = from.as_ref().map(|e| self.int_arg(e));
                let b = to.as_ref().map(|e| self.int_arg(e));
                self.stmt(
                    span,
                    BStmtKind::Io(IoStmt::Lock {
                        file: f,
                        from: a,
                        to: b,
                    }),
                )
            }
            StmtKind::Unlock { file, from, to } => {
                let f = self.file_num(file);
                let a = from.as_ref().map(|e| self.int_arg(e));
                let b = to.as_ref().map(|e| self.int_arg(e));
                self.stmt(
                    span,
                    BStmtKind::Io(IoStmt::Unlock {
                        file: f,
                        from: a,
                        to: b,
                    }),
                )
            }
            StmtKind::Width { file, width } => {
                let f = self.file_num(file);
                let w = width.as_ref().map(|e| self.int_arg(e));
                self.stmt(span, BStmtKind::Io(IoStmt::Width { file: f, width: w }))
            }
            StmtKind::Name { old, new } => {
                let o = self.string_arg(old);
                let n = self.string_arg(new);
                self.stmt(span, BStmtKind::Io(IoStmt::Name { old: o, new: n }))
            }
            StmtKind::RaiseEvent { name, args } => {
                if !self.class_like {
                    self.err(
                        codes::EVENTS,
                        span,
                        "`RaiseEvent` is only valid in class modules",
                    );
                    return None;
                }
                let sema = self.sema;
                let mi = &sema.program.modules[self.module.index()];
                match mi.symbols.get(&name.name) {
                    Some((Symbol::Event(e), _)) => {
                        let ei = &sema.program.events[e.index()];
                        let sig = Sig::of_event(ei);
                        let bargs = self.match_args(&sig, args, span);
                        self.stmt(
                            span,
                            BStmtKind::RaiseEvent {
                                event: *e,
                                args: bargs,
                            },
                        )
                    }
                    _ => {
                        self.err(
                            codes::EVENTS,
                            name.span,
                            format!("`{}` is not an event of this class", name.name),
                        );
                        None
                    }
                }
            }
            StmtKind::DebugPrint(items) => {
                let items = self.out_items(items);
                self.stmt(span, BStmtKind::DebugPrint(items))
            }
        }
    }

    fn out_items(&mut self, items: &[OutputItem]) -> Vec<BOutItem> {
        items
            .iter()
            .map(|i| match i {
                OutputItem::Expr(e) => {
                    let b = self.bind_expr(e);
                    BOutItem::Value(self.rvalue(b))
                }
                OutputItem::Semicolon => BOutItem::Semicolon,
                OutputItem::Comma => BOutItem::Comma,
            })
            .collect()
    }

    fn file_num(&mut self, e: &Expr) -> BExpr {
        let b = self.bind_expr(e);
        let b = self.rvalue(b);
        // A scalar that does not convert to Integer is error 52 at run time
        // (MS-VBAL 5.4.5.1.1), which the file statement itself raises.
        if matches!(
            b.ty,
            Type::String
                | Type::FixedString(_)
                | Type::Variant
                | Type::Boolean
                | Type::Date
                | Type::Single
                | Type::Double
                | Type::Currency
                | Type::Decimal
                | Type::LongLong
        ) {
            return b;
        }
        self.coerce_index(b, Type::Integer)
    }

    fn int_arg(&mut self, e: &Expr) -> BExpr {
        let b = self.bind_expr(e);
        self.coerce_index(b, Type::Long)
    }

    fn string_arg(&mut self, e: &Expr) -> BExpr {
        let b = self.bind_expr(e);
        let b = self.rvalue(b);
        self.coerce(b, &Type::String, Ctx::Let)
    }

    fn bind_assign(
        &mut self,
        kind: AssignKind,
        target: &Expr,
        value: &Expr,
        span: Span,
    ) -> Option<BStmt> {
        if kind == AssignKind::Let
            && let ExprKind::Call { callee, args } = &target.kind
            && let ExprKind::Name(id) = &callee.kind
            && matches!(
                id.name.to_string_lossy().to_ascii_lowercase().as_str(),
                "mid" | "midb"
            )
            && !self.scope.contains_key(&id.name)
            && matches!(
                self.sema.lookup_module_scope(self.module, &id.name),
                crate::sema::Found::None
            )
        {
            return self.bind_mid_statement(id, args, value, span);
        }
        if kind == AssignKind::Let
            && let ExprKind::Name(id) = &target.kind
            && matches!(
                id.name.to_string_lossy().to_ascii_lowercase().as_str(),
                "date" | "time"
            )
            && matches!(id.suffix, None | Some(vba_lexer::TypeSuffix::String))
            && !self.scope.contains_key(&id.name)
            && matches!(
                self.sema.lookup_module_scope(self.module, &id.name),
                crate::sema::Found::None
            )
        {
            // `Date = x` / `Time = x` set the system clock (through the host).
            let is_date = id.name.eq_ignore_case("date");
            let v = self.bind_expr(value);
            let v = self.rvalue(v);
            let v = self.coerce(v, &Type::Date, Ctx::Let);
            if v.ty.is_error() {
                return None;
            }
            let call = self.call_expr(
                span,
                Type::Variant,
                CallTarget::Intrinsic {
                    name: if is_date {
                        "DateStatement"
                    } else {
                        "TimeStatement"
                    }
                    .to_owned(),
                    dollar: false,
                },
                None,
                vec![BArg::Value(v)],
            );
            return self.stmt(span, BStmtKind::Call(call));
        }
        let tgt = self.bind_target(target, false);
        let v = self.bind_expr(value);
        let (place, pty) = tgt?;
        if v.ty.is_error() || pty.is_error() {
            return None;
        }
        let mode = match kind {
            AssignKind::Let => AssignMode::Let,
            AssignKind::LSet => AssignMode::LSet,
            AssignKind::RSet => AssignMode::RSet,
        };
        if matches!(mode, AssignMode::LSet | AssignMode::RSet) {
            let ok_target = pty.is_string()
                || (mode == AssignMode::LSet && matches!(pty, Type::Udt(_) | Type::LibRecord(_)))
                || pty.is_variant();
            if !ok_target {
                self.err(
                    codes::TYPE_MISMATCH,
                    target.span,
                    "`LSet`/`RSet` need a string (or, for `LSet`, a user-defined type) target",
                );
                return None;
            }
            let v = if matches!(pty, Type::Udt(_) | Type::LibRecord(_)) {
                self.coerce(v, &pty, Ctx::Let)
            } else {
                let v = self.rvalue(v);
                self.coerce(v, &Type::String, Ctx::Let)
            };
            return self.stmt(
                span,
                BStmtKind::Assign {
                    target: place,
                    value: v,
                    mode,
                },
            );
        }
        // `w = 5` with an object target assigns the object's default member
        // (MS-VBAL 5.4.3.8: let-assignment uses the default member).
        if mode == AssignMode::Let
            && let Type::Class(m) = &pty
            && let Some(dm) = self.sema.program.modules[m.index()].default_member.clone()
        {
            let synth = Expr {
                span: target.span,
                kind: ExprKind::Member {
                    target: Some(Box::new(target.clone())),
                    name: vba_ast::Ident {
                        name: dm,
                        suffix: None,
                        kind: vba_ast::IdentKind::Plain,
                        span: target.span,
                    },
                },
            };
            let (place, pty) = self.bind_target(&synth, false)?;
            let v = self.rvalue(v);
            let v = self.coerce(v, &pty, Ctx::Let);
            if v.ty.is_error() {
                return None;
            }
            return self.stmt(
                span,
                BStmtKind::Assign {
                    target: place,
                    value: v,
                    mode,
                },
            );
        }
        if pty.is_object() {
            self.err(
                codes::ASSIGNMENT_KIND,
                span,
                format!(
                    "assigning to an object variable ({}) requires `Set`",
                    self.tn(&pty)
                ),
            );
            return None;
        }
        if pty.is_array() && self.place_is_fixed_array(&place) {
            self.err(
                codes::ARRAY_RULE,
                target.span,
                "a fixed-size array cannot be assigned to",
            );
            return None;
        }
        let v = self.coerce(v, &pty, Ctx::Let);
        if v.ty.is_error() {
            return None;
        }
        self.stmt(
            span,
            BStmtKind::Assign {
                target: place,
                value: v,
                mode,
            },
        )
    }

    /// `Mid[B][$](target, start[, length]) = replacement`: a statement, bound
    /// as a call of the run-time provided `MidStatement`/`MidBStatement` with
    /// the target passed by reference.
    fn bind_mid_statement(
        &mut self,
        id: &vba_ast::Ident,
        args: &[vba_ast::Arg],
        value: &Expr,
        span: Span,
    ) -> Option<BStmt> {
        let exprs: Vec<&Expr> = args.iter().filter_map(|a| a.value.as_ref()).collect();
        if !(2..=3).contains(&args.len())
            || exprs.len() != args.len()
            || args.iter().any(|a| a.name.is_some())
        {
            self.err(
                codes::BAD_ARGUMENTS,
                span,
                "the `Mid` statement takes a string variable, a start and an optional length",
            );
            return None;
        }
        let t = self.bind_expr(exprs[0]);
        let place = self.expr_to_place(&t);
        let (Some(place), true) = (place, t.ty.is_string() || t.ty.is_variant()) else {
            if !t.ty.is_error() {
                self.err(
                    codes::TYPE_MISMATCH,
                    exprs[0].span,
                    "the first argument of the `Mid` statement must be a String variable",
                );
            }
            return None;
        };
        let start = self.bind_expr(exprs[1]);
        let start = self.coerce_index(start, Type::Long);
        let length = match exprs.get(2) {
            Some(e) => {
                let b = self.bind_expr(e);
                BArg::Value(self.coerce_index(b, Type::Long))
            }
            None => BArg::Missing,
        };
        let v = self.bind_expr(value);
        let v = self.rvalue(v);
        let v = self.coerce(v, &Type::String, Ctx::Let);
        let name = if id.name.eq_ignore_case("midb") {
            "MidBStatement"
        } else {
            "MidStatement"
        };
        let call = self.call_expr(
            span,
            Type::Variant,
            CallTarget::Intrinsic {
                name: name.to_owned(),
                dollar: false,
            },
            None,
            vec![BArg::Ref(place), BArg::Value(start), length, BArg::Value(v)],
        );
        self.stmt(span, BStmtKind::Call(call))
    }

    fn bind_set(&mut self, target: &Expr, value: &Expr, span: Span) -> Option<BStmt> {
        let tgt = self.bind_target(target, true);
        let v = self.bind_expr(value);
        let (place, pty) = tgt?;
        if v.ty.is_error() || pty.is_error() {
            return None;
        }
        if !(pty.is_object() || pty.is_variant()) {
            self.err(
                codes::ASSIGNMENT_KIND,
                span,
                format!(
                    "`Set` needs an object variable, but the target is {}",
                    self.tn(&pty)
                ),
            );
            return None;
        }
        if !(v.ty.is_object() || v.ty.is_variant()) {
            self.err(
                codes::TYPE_MISMATCH,
                value.span,
                format!("object required, found {}", self.tn(&v.ty)),
            );
            return None;
        }
        let v = self.coerce(v, &pty, Ctx::Arg);
        if v.ty.is_error() {
            return None;
        }
        self.stmt(
            span,
            BStmtKind::Assign {
                target: place,
                value: v,
                mode: AssignMode::Set,
            },
        )
    }

    fn bind_if(&mut self, i: &IfStmt, span: Span) -> Option<BStmt> {
        let mut arms = Vec::new();
        let c = self.bind_expr(&i.cond);
        let c = self.coerce_condition(c);
        let body = self.bind_block(&i.then_body);
        arms.push((c, body));
        for ei in &i.else_ifs {
            let c = self.bind_expr(&ei.cond);
            let c = self.coerce_condition(c);
            let body = self.bind_block(&ei.body);
            arms.push((c, body));
        }
        let else_body = i.else_body.as_ref().map(|b| self.bind_block(b));
        self.stmt(span, BStmtKind::If { arms, else_body })
    }

    fn counter_name(e: &Expr) -> Option<vba_ast::Name> {
        match &e.kind {
            ExprKind::Name(i) => Some(i.name.clone()),
            _ => None,
        }
    }

    fn bind_for(&mut self, f: &vba_ast::ForStmt, span: Span) -> Option<BStmt> {
        let c = self.bind_expr(&f.counter);
        let counter = if c.ty.is_error() {
            None
        } else {
            match self.expr_to_place(&c) {
                Some(p) if !matches!(p, BPlace::Index { .. }) => Some((p, c.ty.clone())),
                _ => {
                    self.err(
                        codes::CONTROL_VARIABLE,
                        f.counter.span,
                        "a `For` counter must be a simple numeric variable",
                    );
                    None
                }
            }
        };
        if let Some((_, t)) = &counter
            && !(t.is_numeric() || t.is_variant() || *t == Type::Date)
        {
            self.err(
                codes::CONTROL_VARIABLE,
                f.counter.span,
                format!("a `For` counter must be numeric, not {}", self.tn(t)),
            );
        }
        let conv = |this: &mut Self, e: &Expr| {
            let b = this.bind_expr(e);
            let b = this.rvalue(b);
            match &counter {
                Some((_, t)) if !t.is_variant() && !t.is_error() => this.coerce(b, t, Ctx::Let),
                _ => b,
            }
        };
        let start = conv(self, &f.start);
        let end = conv(self, &f.end);
        let step = f.step.as_ref().map(|e| conv(self, e));
        self.loops.push(LoopKind::For);
        self.for_counters.push(Self::counter_name(&f.counter));
        let body = self.bind_block(&f.body);
        self.for_counters.pop();
        self.loops.pop();
        if let Some(first) = f.next_counters.first()
            && let (Some(a), Some(b)) = (Self::counter_name(first), Self::counter_name(&f.counter))
            && a != b
        {
            self.err(
                codes::CONTROL_VARIABLE,
                first.span,
                format!("`Next {a}` does not match `For {b}`"),
            );
        }
        let (counter, counter_ty) = counter?;
        self.stmt(
            span,
            BStmtKind::For {
                counter,
                counter_ty,
                start,
                end,
                step,
                body,
            },
        )
    }

    fn bind_for_each(&mut self, f: &vba_ast::ForEachStmt, span: Span) -> Option<BStmt> {
        let item = self.bind_expr(&f.item);
        let coll = self.bind_expr(&f.collection);
        let place = if item.ty.is_error() {
            None
        } else {
            match self.expr_to_place(&item) {
                Some(p) if !matches!(p, BPlace::Index { .. }) => Some(p),
                _ => {
                    self.err(
                        codes::CONTROL_VARIABLE,
                        f.item.span,
                        "a `For Each` variable must be a simple variable",
                    );
                    None
                }
            }
        };
        if !coll.ty.is_error() && !item.ty.is_error() {
            match &coll.ty {
                Type::Array { .. } => {
                    if !item.ty.is_variant() {
                        self.err(
                            codes::CONTROL_VARIABLE,
                            f.item.span,
                            "the control variable of a `For Each` over an array must be a Variant",
                        );
                    }
                }
                t if t.is_object() || t.is_variant() => {
                    if !(item.ty.is_variant() || item.ty.is_object()) {
                        self.err(
                            codes::CONTROL_VARIABLE,
                            f.item.span,
                            "the control variable of a `For Each` must be a Variant or an object",
                        );
                    }
                }
                _ => self.err(
                    codes::CONTROL_VARIABLE,
                    f.collection.span,
                    "`For Each` needs an array or a collection",
                ),
            }
        }
        self.loops.push(LoopKind::For);
        self.for_counters.push(Self::counter_name(&f.item));
        let body = self.bind_block(&f.body);
        self.for_counters.pop();
        self.loops.pop();
        if let Some(first) = f.next_counters.first()
            && let (Some(a), Some(b)) = (Self::counter_name(first), Self::counter_name(&f.item))
            && a != b
        {
            self.err(
                codes::CONTROL_VARIABLE,
                first.span,
                format!("`Next {a}` does not match `For Each {b}`"),
            );
        }
        let place = place?;
        self.stmt(
            span,
            BStmtKind::ForEach {
                item_ty: item.ty.clone(),
                item: place,
                collection: coll,
                body,
            },
        )
    }

    fn bind_with(&mut self, object: &Expr, body: &Block, span: Span) -> Option<BStmt> {
        let obj = self.bind_expr(object);
        if obj.ty.is_error() {
            // Still bind the body so its own errors are found.
            self.withs.push(WithEntry {
                receiver: self.error_expr(object.span),
            });
            let _ = self.bind_block(body);
            self.withs.pop();
            return None;
        }
        if obj.ty.is_object() || obj.ty.is_variant() {
            let local = self.new_local(LocalInfo {
                name: vba_ast::Name::new(""),
                ty: obj.ty.clone(),
                kind: LocalKind::Temp,
                fixed_bounds: None,
                auto_new: false,
                span: object.span,
            });
            let receiver = BExpr::new(object.span, obj.ty.clone(), BExprKind::Local(local));
            self.withs.push(WithEntry { receiver });
            let body = self.bind_block(body);
            self.withs.pop();
            return self.stmt(
                span,
                BStmtKind::With {
                    local,
                    object: obj,
                    body,
                },
            );
        }
        if matches!(obj.ty, Type::Udt(_) | Type::LibRecord(_)) && self.expr_to_place(&obj).is_some()
        {
            self.withs.push(WithEntry { receiver: obj });
            let body = self.bind_block(body);
            self.withs.pop();
            return self.stmt(span, BStmtKind::Block(body));
        }
        self.err(
            codes::NO_OBJECT_CONTEXT,
            object.span,
            format!(
                "`With` needs an object or a user-defined type variable, not {}",
                self.tn(&obj.ty)
            ),
        );
        None
    }

    fn bind_redim(
        &mut self,
        preserve: bool,
        items: &[vba_ast::ReDimItem],
        span: Span,
    ) -> Option<BStmt> {
        let mut out = Vec::new();
        for it in items {
            let t = self.bind_expr(&it.target);
            if t.ty.is_error() {
                continue;
            }
            let Some(place) = self.expr_to_place(&t) else {
                self.err(
                    codes::ARRAY_RULE,
                    it.target.span,
                    "`ReDim` needs an array variable",
                );
                continue;
            };
            let (mut elem, rank) = match &t.ty {
                Type::Array { elem, rank } => ((**elem).clone(), *rank),
                Type::Variant => (Type::Variant, None),
                other => {
                    self.err(
                        codes::ARRAY_RULE,
                        it.target.span,
                        format!("`ReDim` needs an array, not {}", self.tn(other)),
                    );
                    continue;
                }
            };
            if self.place_is_fixed_array(&place) {
                self.err(
                    codes::ARRAY_RULE,
                    it.target.span,
                    "array already dimensioned: only dynamic arrays can be `ReDim`ed",
                );
                continue;
            }
            if let Some(a) = &it.as_clause {
                let declared = self.sema.resolve_as_clause(self.module, a, false);
                // `ReDim x(n) As T` on an implicitly declared name declares an array of T.
                if t.ty.is_variant()
                    && !declared.is_error()
                    && let BPlace::Local(l) = &place
                    && self.locals[l.index()].kind == LocalKind::Implicit
                {
                    self.locals[l.index()].ty = Type::Array {
                        elem: Box::new(declared.clone()),
                        rank: u8::try_from(it.bounds.len()).ok(),
                    };
                    elem = declared.clone();
                }
                if !declared.is_error() && declared != elem && !elem.is_variant() {
                    self.err(
                        codes::ARRAY_RULE,
                        a.span,
                        "the `As` type differs from the declared element type of the array",
                    );
                }
            }
            if let Some(r) = rank
                && usize::from(r) != it.bounds.len()
            {
                self.err(
                    codes::ARRAY_RULE,
                    it.span,
                    format!(
                        "wrong number of dimensions: the array has {r}, `ReDim` gives {}",
                        it.bounds.len()
                    ),
                );
                continue;
            }
            let mut bounds = Vec::new();
            for b in &it.bounds {
                let lower = b.lower.as_ref().map(|e| {
                    let x = self.bind_expr(e);
                    self.coerce_index(x, Type::Long)
                });
                let u = self.bind_expr(&b.upper);
                let upper = self.coerce_index(u, Type::Long);
                bounds.push(BBound { lower, upper });
            }
            out.push(BReDim {
                target: place,
                bounds,
                elem,
            });
        }
        self.stmt(
            span,
            BStmtKind::ReDim {
                preserve,
                items: out,
            },
        )
    }
}
