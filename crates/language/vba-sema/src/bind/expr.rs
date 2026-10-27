//! Expression binding: simple name resolution (MS-VBAL §5.6), member
//! access (§5.6), index and call classification.

use vba_ast::{Arg, BinaryOp, Expr, ExprKind, Ident, Name, UnaryOp};
use vba_lexer::TypeSuffix;
use vba_source::Span;
use vba_value::Variant;

use crate::bound::*;
use crate::codes;
use crate::fold::{literal, long_ptr};
use crate::program::*;
use crate::provider::{LibGlobal, LibMemberKind};
use crate::sema::Found;
use crate::types::*;

use super::call::{Sig, with_dispid};
use super::{Binder, LocalSym};

/// A qualifier that is not a value.
enum Namespace {
    Module(ModuleId),
    Enum(EnumId),
    Project,
    /// A library enum: `IOMode.ForReading`.
    LibEnum(LibraryType),
    /// A library: `Scripting.ForReading`, `Scripting.IOMode`.
    Library(String),
}

fn lookup_intrinsic(id: &Ident) -> Option<vba_stdlib::Resolved> {
    let mut n = id.name.to_string_lossy();
    match id.suffix {
        None => {}
        Some(TypeSuffix::String) => n.push('$'),
        Some(_) => return None,
    }
    vba_stdlib::lookup(&n)
}

impl Binder<'_, '_> {
    pub(crate) fn me_expr(&self, span: Span) -> BExpr {
        BExpr::new(span, self.me_type(), BExprKind::Me)
    }

    fn local_expr(&self, l: LocalId, span: Span) -> BExpr {
        BExpr::new(span, self.local_ty(l), BExprKind::Local(l))
    }

    /// A reference to a module-level variable: a static for standard
    /// modules, an instance field for classes.
    fn global_expr(&self, g: GlobalId, span: Span, receiver: Option<BExpr>) -> BExpr {
        let sema = self.sema;
        let gi = &sema.program.globals[g.index()];
        if sema.program.modules[gi.module.index()].is_class_like() {
            let recv = receiver.unwrap_or_else(|| self.me_expr(span));
            BExpr::new(
                span,
                gi.ty.clone(),
                BExprKind::Field {
                    receiver: Box::new(recv),
                    global: g,
                },
            )
        } else {
            BExpr::new(span, gi.ty.clone(), BExprKind::Global(g))
        }
    }

    // ---- entry points -------------------------------------------------------

    /// Binds an expression without applying the `Let` coercion (an object
    /// stays an object).
    pub(crate) fn bind_expr(&mut self, e: &Expr) -> BExpr {
        let span = e.span;
        if self.sema.charge_node() {
            return self.error_expr(span);
        }
        match &e.kind {
            ExprKind::Literal(l) => match literal(l, self.sema.profile()) {
                Ok((c, t)) => BExpr::new(span, t, BExprKind::Const(c)),
                Err(m) => {
                    self.err(codes::CONSTANT_EXPRESSION, span, m);
                    self.error_expr(span)
                }
            },
            ExprKind::Name(_) | ExprKind::Member { .. } | ExprKind::Dict { .. } => {
                self.bind_ref(e, None, false)
            }
            ExprKind::Call { callee, args } => self.bind_ref(callee, Some(args), false),
            ExprKind::Paren(inner) => {
                let b = self.bind_expr(inner);
                let mut b = self.rvalue(b);
                b.span = span;
                b
            }
            ExprKind::Unary { op, operand } => {
                let o = self.bind_expr(operand);
                let o = self.rvalue(o);
                if o.ty.is_error() {
                    return self.error_expr(span);
                }
                if matches!(
                    o.ty,
                    Type::Udt(_) | Type::LibRecord(_) | Type::Array { .. } | Type::Nothing
                ) {
                    self.err(
                        codes::TYPE_MISMATCH,
                        span,
                        format!("operator is not defined for {}", self.tn(&o.ty)),
                    );
                    return self.error_expr(span);
                }
                // Fold negation and `Not` of constants (`-1`, `Not True`).
                if let BExprKind::Const(Constant::Value(v)) = &o.kind {
                    let folded = match op {
                        UnaryOp::Neg => {
                            vba_value::ops::negate(v, vba_value::ops::Overflow::Error).ok()
                        }
                        UnaryOp::Not => vba_value::ops::not(v).ok(),
                        UnaryOp::Plus => None,
                    };
                    if let Some(r) = folded {
                        let ty = crate::fold::type_of_variant(&r);
                        return BExpr::new(span, ty, BExprKind::Const(Constant::Value(r)));
                    }
                }
                let ty = match op {
                    UnaryOp::Neg | UnaryOp::Plus => negate_result(&o.ty),
                    UnaryOp::Not => not_result(&o.ty),
                };
                BExpr::new(
                    span,
                    ty,
                    BExprKind::Unary {
                        op: *op,
                        operand: Box::new(o),
                    },
                )
            }
            ExprKind::Binary { op, lhs, rhs } => self.bind_binary(*op, lhs, rhs, span),
            ExprKind::New(t) => {
                let ty = self.sema.resolve_type_ref(self.module, t, false);
                match ty {
                    Type::Class(m) => {
                        BExpr::new(span, Type::Class(m), BExprKind::New(NewTarget::Class(m)))
                    }
                    Type::Library(lt) => {
                        self.sema
                            .check_creatable(self.module, span, &Type::Library(lt.clone()));
                        BExpr::new(
                            span,
                            Type::Library(lt.clone()),
                            BExprKind::New(NewTarget::Library(lt)),
                        )
                    }
                    Type::Error => self.error_expr(span),
                    other => {
                        self.err(
                            codes::TYPE_MISMATCH,
                            span,
                            format!("`New` requires a class, not {}", self.tn(&other)),
                        );
                        self.error_expr(span)
                    }
                }
            }
            ExprKind::AddressOf(inner) => self.bind_address_of(inner, span),
            ExprKind::TypeOfIs { expr, ty } => {
                // MS-VBAL 5.6.7: the operand is a variable, function, property
                // or member, not a literal such as `Nothing`.
                if matches!(expr.kind, ExprKind::Literal(_)) {
                    self.err(
                        codes::TYPE_MISMATCH,
                        expr.span,
                        "`TypeOf` requires a variable or member, not a literal",
                    );
                    return self.error_expr(span);
                }
                let b = self.bind_expr(expr);
                let t = self.sema.resolve_type_ref(self.module, ty, false);
                if !(b.ty.is_object() || b.ty.is_variant() || b.ty.is_error()) {
                    self.err(
                        codes::TYPE_MISMATCH,
                        b.span,
                        "`TypeOf` requires an object expression",
                    );
                    return self.error_expr(span);
                }
                if !(t.is_object() || t.is_error()) {
                    self.err(
                        codes::TYPE_MISMATCH,
                        ty.span,
                        "`TypeOf ... Is` requires a class or interface type",
                    );
                    return self.error_expr(span);
                }
                BExpr::new(
                    span,
                    Type::Boolean,
                    BExprKind::TypeOfIs {
                        expr: Box::new(b),
                        ty: t,
                    },
                )
            }
            ExprKind::FileNumber(inner) => self.bind_expr(inner),
            ExprKind::Range { .. } => {
                self.err(
                    codes::NOT_CALLABLE,
                    span,
                    "a `To` range is only valid in a subscript list",
                );
                self.error_expr(span)
            }
            ExprKind::Error => self.error_expr(span),
        }
    }

    fn bind_binary(&mut self, op: BinaryOp, lhs: &Expr, rhs: &Expr, span: Span) -> BExpr {
        let l = self.bind_expr(lhs);
        let r = self.bind_expr(rhs);
        let compare = self.options().compare;
        if op == BinaryOp::Is {
            let ok = |t: &Type| t.is_object() || t.is_variant() || t.is_error();
            if !ok(&l.ty) || !ok(&r.ty) {
                self.err(codes::TYPE_MISMATCH, span, "`Is` requires object operands");
                return self.error_expr(span);
            }
            return BExpr::new(
                span,
                Type::Boolean,
                BExprKind::Binary {
                    op,
                    lhs: Box::new(l),
                    rhs: Box::new(r),
                    compare,
                },
            );
        }
        let l = self.rvalue(l);
        let r = self.rvalue(r);
        if l.ty.is_error() || r.ty.is_error() {
            return self.error_expr(span);
        }
        for t in [&l.ty, &r.ty] {
            if matches!(
                t,
                Type::Udt(_) | Type::LibRecord(_) | Type::Array { .. } | Type::Nothing | Type::Any
            ) {
                self.err(
                    codes::TYPE_MISMATCH,
                    span,
                    format!("operator is not defined for {}", self.tn(t)),
                );
                return self.error_expr(span);
            }
        }
        let ty = binary_result(op, &l.ty, &r.ty);
        BExpr::new(
            span,
            ty,
            BExprKind::Binary {
                op,
                lhs: Box::new(l),
                rhs: Box::new(r),
                compare,
            },
        )
    }

    fn bind_address_of(&mut self, inner: &Expr, span: Span) -> BExpr {
        if !self
            .sema
            .profile()
            .supports(vba_profile::Feature::AddressOf)
        {
            self.err(
                codes::UNSUPPORTED,
                span,
                "`AddressOf` is not available in this language profile",
            );
        }
        let ty = long_ptr(self.sema.profile());
        let (name, name_span, target) = match &inner.kind {
            ExprKind::Name(id) => {
                let found = if self.scope.contains_key(&id.name) {
                    None
                } else {
                    Some(self.sema.lookup_module_scope(self.module, &id.name))
                };
                match found {
                    Some(Found::Ambiguous(ms)) => {
                        self.sema
                            .ambiguous(self.module, id.span, &id.name.to_string(), &ms);
                        return self.error_expr(span);
                    }
                    Some(Found::Sym(_, sym)) => (&id.name, id.span, Some(sym)),
                    _ => (&id.name, id.span, None),
                }
            }
            // `Module.Proc` and `Project.Module.Proc`.
            ExprKind::Member {
                target: Some(q),
                name,
            } => match self.try_namespace(q) {
                Some(Namespace::Module(m)) => {
                    let mi = &self.sema.program.modules[m.index()];
                    let sym = mi.symbols.get(&name.name).map(|(s, _)| *s);
                    match sym {
                        Some(s) if m != self.module && !self.sema.symbol_public(s) => {
                            self.err(
                                codes::NOT_ACCESSIBLE,
                                name.span,
                                format!("`{}` is private to module `{}`", name.name, mi.name),
                            );
                            return self.error_expr(span);
                        }
                        _ => (&name.name, name.span, sym),
                    }
                }
                _ => (&name.name, name.span, None),
            },
            _ => {
                self.err(
                    codes::NOT_CALLABLE,
                    span,
                    "`AddressOf` requires the name of a procedure",
                );
                return self.error_expr(span);
            }
        };
        let sema = self.sema;
        match target {
            Some(Symbol::Proc(p)) => {
                if let Some(why) = callback_restriction(sema, p) {
                    self.err(
                        codes::NOT_CALLABLE,
                        name_span,
                        format!("`AddressOf {name}` is not allowed: {why}"),
                    );
                    return self.error_expr(span);
                }
                BExpr::new(span, ty, BExprKind::AddressOf(AddressTarget::Proc(p)))
            }
            Some(Symbol::Declare(_)) => {
                self.err(
                    codes::NOT_CALLABLE,
                    name_span,
                    format!(
                        "`AddressOf {name}` is not allowed: a Declare names an external procedure, not a Sub or Function of this project"
                    ),
                );
                self.error_expr(span)
            }
            _ => {
                self.err(
                    codes::NOT_CALLABLE,
                    name_span,
                    format!("`AddressOf` requires a Sub or Function, but `{name}` is not one"),
                );
                self.error_expr(span)
            }
        }
    }

    // ---- references ---------------------------------------------------------

    /// Binds a name, member access, `!` access or call. `args` is `Some`
    /// when argument parentheses were written; `stmt` when the result may be
    /// discarded (a call statement).
    pub(crate) fn bind_ref(&mut self, e: &Expr, args: Option<&[Arg]>, stmt: bool) -> BExpr {
        match &e.kind {
            ExprKind::Name(id) => self.bind_name(id, e.span, args, stmt),
            ExprKind::Member { target, name } => {
                self.bind_member(target.as_deref(), name, e.span, args, stmt)
            }
            ExprKind::Dict { target, name } => {
                self.bind_dict(target.as_deref(), name, e.span, args)
            }
            ExprKind::Paren(_) | ExprKind::Call { .. } | ExprKind::Literal(_) => {
                let b = self.bind_expr(e);
                match args {
                    Some(a) => self.apply_args(b, a, e.span),
                    None => b,
                }
            }
            _ => {
                let b = self.bind_expr(e);
                match args {
                    Some(a) => self.apply_args(b, a, e.span),
                    None => b,
                }
            }
        }
    }

    fn bind_name(&mut self, id: &Ident, span: Span, args: Option<&[Arg]>, stmt: bool) -> BExpr {
        if id.is_missing() {
            return self.error_expr(span);
        }
        if id.name.eq_ignore_case("Me") && !self.scope.contains_key(&id.name) {
            if !self.class_like
                && self.sema.friend.is_some_and(|(f, _)| f == self.module)
                && let Some(LocalSym::Local(l)) = self.scope.get(&Name::new("zzMe")).cloned()
            {
                // Code evaluated for a frame: `Me` is the parameter that
                // carries the frame's object (see `Options::friend`).
                let e = self.local_expr(l, span);
                return match args {
                    Some(a) => self.apply_args(e, a, span),
                    None => e,
                };
            }
            if !self.class_like {
                self.err(
                    codes::NO_OBJECT_CONTEXT,
                    span,
                    "`Me` is only valid in class modules",
                );
                return self.error_expr(span);
            }
            let me = self.me_expr(span);
            return match args {
                Some(a) => self.apply_args(me, a, span),
                None => me,
            };
        }
        if let Some(sym) = self.scope.get(&id.name).cloned() {
            return match sym {
                LocalSym::Local(l) => {
                    // MS-VBAL 5.6.10: a type character must match the declared type.
                    if let Some(sfx) = id.suffix {
                        let declared = match self.local_ty(l) {
                            Type::Array { elem, .. } => *elem,
                            other => other,
                        };
                        let wanted = self.sema.suffix_type(sfx);
                        if !declared.is_error() && !declared.is_variant() && declared != wanted {
                            self.spec(
                                codes::TYPE_MISMATCH,
                                id.span,
                                format!(
                                    "the type character does not match the declared type of `{}`",
                                    id.name
                                ),
                            );
                            if self.sema.strict_spec {
                                return self.error_expr(span);
                            }
                        }
                    }
                    let b = self.local_expr(l, span);
                    self.var_use(b, span, args)
                }
                LocalSym::Static(g) => {
                    let b = self.global_expr(g, span, None);
                    self.var_use(b, span, args)
                }
                LocalSym::Const(c, t) => {
                    if args.is_some() {
                        self.err(
                            codes::NOT_CALLABLE,
                            span,
                            "a constant cannot be called or indexed",
                        );
                        return self.error_expr(span);
                    }
                    BExpr::new(span, t, BExprKind::Const(c))
                }
            };
        }
        let sema = self.sema;
        match sema.lookup_module_scope(self.module, &id.name) {
            Found::Sym(owner, sym) => self.use_symbol(owner, sym, id, span, args, stmt, None),
            Found::Module(m) => self.module_as_value(m, id, span, args),
            Found::Ambiguous(ms) => {
                sema.ambiguous(self.module, id.span, &id.name.to_string(), &ms);
                self.error_expr(span)
            }
            Found::None if self.host_has_member(id) => {
                let lt = self.host_class().expect("host class");
                let me = self.host_me(span);
                self.member_of_lib(me, lt, id, args, stmt, span)
            }
            Found::None => self.bind_library_name(id, span, args, stmt),
        }
    }

    fn module_as_value(
        &mut self,
        m: ModuleId,
        id: &Ident,
        span: Span,
        args: Option<&[Arg]>,
    ) -> BExpr {
        let mi = &self.sema.program.modules[m.index()];
        if mi.is_class_like() && mi.predeclared {
            let b = BExpr::new(span, Type::Class(m), BExprKind::DefaultInstance(m));
            return match args {
                Some(a) => self.apply_args(b, a, span),
                None => b,
            };
        }
        self.err(
            codes::NOT_CALLABLE,
            id.span,
            format!(
                "`{}` is a {} and cannot be used as a value",
                id.name,
                if mi.is_class_like() {
                    "class (create an instance with `New`)"
                } else {
                    "module"
                }
            ),
        );
        self.error_expr(span)
    }

    fn bind_library_name(
        &mut self,
        id: &Ident,
        span: Span,
        args: Option<&[Arg]>,
        stmt: bool,
    ) -> BExpr {
        if let Some(r) = lookup_intrinsic(id) {
            return self.call_intrinsic(r, span, args);
        }
        // The `$` forms of the run-time functions that return a String
        // (`Command$`, `CurDir$`, `Dir$`, `Input$`, `InputB$`, `InputBox$`).
        let dollar_rt = id.suffix == Some(TypeSuffix::String)
            && matches!(
                id.name.to_string_lossy().to_ascii_lowercase().as_str(),
                "command" | "curdir" | "dir" | "input" | "inputb" | "inputbox"
            );
        if (id.suffix.is_none() || dollar_rt)
            && let Some(g) = self.sema.provider.global(&id.name.to_string_lossy())
            && (id.suffix.is_none() || matches!(g, LibGlobal::Function(_)))
        {
            return match g {
                LibGlobal::Constant(v, t) => {
                    if args.is_some() {
                        self.err(
                            codes::NOT_CALLABLE,
                            span,
                            "a constant cannot be called or indexed",
                        );
                        return self.error_expr(span);
                    }
                    BExpr::new(span, t, BExprKind::Const(Constant::Value(v)))
                }
                LibGlobal::Object(t) => {
                    let b = BExpr::new(span, t, BExprKind::LibGlobal(id.name.to_string()));
                    self.var_use(b, span, args)
                }
                LibGlobal::Function(m) => {
                    if m.is_sub && !stmt {
                        self.err(
                            codes::NOT_CALLABLE,
                            span,
                            format!("`{}` is a Sub and does not return a value", m.name),
                        );
                        return self.error_expr(span);
                    }
                    let sig = Sig::of_lib(&m);
                    let bargs = self.match_args(&sig, args.unwrap_or(&[]), span);
                    let ty = if m.is_sub {
                        Type::Variant
                    } else if dollar_rt {
                        Type::String
                    } else {
                        m.ty.clone()
                    };
                    self.call_expr(
                        span,
                        ty,
                        CallTarget::Intrinsic {
                            name: m.name.clone(),
                            dollar: false,
                        },
                        None,
                        bargs,
                    )
                }
            };
        }
        // Unresolved.
        if self.is_host_name(id)
            && (args.is_some()
                || stmt
                || self
                    .sema
                    .host_members
                    .contains(&id.name.to_string_lossy().to_ascii_lowercase()))
        {
            return self.host_member(id, span, args);
        }
        if args.is_none() && !stmt {
            if self.explicit {
                self.err(
                    codes::UNDECLARED_NAME,
                    id.span,
                    format!(
                        "variable `{}` is not defined (Option Explicit){}",
                        id.name,
                        self.host_scope_note()
                    ),
                );
                return self.error_expr(span);
            }
            // Implicit declaration (§5.6): a local Variant (or Def type).
            let ty = match id.suffix {
                Some(s) => self.sema.suffix_type(s),
                None => self.options().default_type_for(&id.name),
            };
            let l = self.new_local(LocalInfo {
                name: id.name.clone(),
                ty,
                kind: LocalKind::Implicit,
                fixed_bounds: None,
                auto_new: false,
                span: id.span,
            });
            self.scope.insert(id.name.clone(), LocalSym::Local(l));
            return self.local_expr(l, span);
        }
        self.err(
            codes::UNDEFINED_NAME,
            id.span,
            format!(
                "`{}` is not defined (Sub or Function not defined){}",
                id.name,
                self.host_scope_note()
            ),
        );
        if let Some(a) = args {
            for x in a {
                if let Some(v) = &x.value {
                    let _ = self.bind_expr(v);
                }
            }
        }
        self.error_expr(span)
    }

    fn call_intrinsic(
        &mut self,
        r: vba_stdlib::Resolved,
        span: Span,
        args: Option<&[Arg]>,
    ) -> BExpr {
        let mut sig = Sig::of_intrinsic(r.info, r.dollar);
        if r.info.name == "VarPtr" {
            // The token names the variable itself, so the argument is
            // passed by reference without a converted copy.
            sig.params[0].ty = Type::Any;
        }
        // `InStr([start,] string1, string2[, compare])`: the two-argument form
        // has no `start`, which the declared signature cannot express.
        let shifted;
        let args = match args {
            Some(a)
                if matches!(r.info.name, "InStr" | "InStrB")
                    && a.len() == 2
                    && a.iter().all(|x| x.name.is_none() && x.value.is_some()) =>
            {
                shifted = vec![
                    Arg {
                        span: Span::empty(a[0].span.start),
                        name: None,
                        value: None,
                    },
                    a[0].clone(),
                    a[1].clone(),
                ];
                Some(shifted.as_slice())
            }
            other => other,
        };
        let outer = std::mem::replace(&mut self.udt_arg_ok, matches!(r.info.name, "Len" | "LenB"));
        let bargs = self.match_args(&sig, args.unwrap_or(&[]), span);
        self.udt_arg_ok = outer;
        let ty = sig.ret.clone().unwrap_or(Type::Variant);
        // MS-VBAL 6.1.2.11.1.22: `Len` of a variable of a numeric type is the
        // number of bytes needed to store it. The run-time value alone cannot
        // tell `Dim i As Integer` from a Variant holding an Integer, so the
        // static type selects the byte-size intrinsic.
        let mut name = r.info.name;
        if matches!(name, "Len" | "LenB")
            && let [BArg::Value(e)] = bargs.as_slice()
        {
            let inner = match &e.kind {
                BExprKind::Convert(inner) => inner.ty.clone(),
                _ => e.ty.clone(),
            };
            if matches!(
                inner,
                Type::Boolean
                    | Type::Byte
                    | Type::Integer
                    | Type::Long
                    | Type::LongLong
                    | Type::Single
                    | Type::Double
                    | Type::Currency
                    | Type::Date
            ) {
                name = "LenSize";
            }
        }
        self.call_expr(
            span,
            ty,
            CallTarget::Intrinsic {
                name: name.to_owned(),
                dollar: r.dollar,
            },
            None,
            bargs,
        )
    }

    /// A symbol found in module `owner`'s scope, optionally through an
    /// explicit receiver.
    #[allow(clippy::too_many_arguments)]
    fn use_symbol(
        &mut self,
        owner: ModuleId,
        sym: Symbol,
        id: &Ident,
        span: Span,
        args: Option<&[Arg]>,
        stmt: bool,
        receiver: Option<BExpr>,
    ) -> BExpr {
        let sema = self.sema;
        match sym {
            Symbol::Var(g) => {
                let b = self.global_expr(g, span, receiver);
                self.var_use(b, span, args)
            }
            Symbol::Const(c) => {
                if args.is_some() {
                    self.err(
                        codes::NOT_CALLABLE,
                        span,
                        "a constant cannot be called or indexed",
                    );
                    return self.error_expr(span);
                }
                let ci = &sema.program.consts[c.index()];
                if ci.ty.is_error() {
                    return self.error_expr(span);
                }
                BExpr::new(span, ci.ty.clone(), BExprKind::Const(ci.value.clone()))
            }
            Symbol::EnumMember(e, i) => {
                if args.is_some() {
                    self.err(
                        codes::NOT_CALLABLE,
                        span,
                        "an enum member cannot be called or indexed",
                    );
                    return self.error_expr(span);
                }
                let v = sema.program.enums[e.index()].members[i as usize].1;
                BExpr::new(
                    span,
                    Type::Enum(e),
                    BExprKind::Const(Constant::Value(Variant::Long(v))),
                )
            }
            Symbol::Proc(p) => self.call_proc(p, owner, receiver, span, args, stmt),
            Symbol::Property(g) => {
                let Some(get) = g.get else {
                    self.err(
                        codes::PROPERTY_PAIRING,
                        id.span,
                        format!("property `{}` is write-only", id.name),
                    );
                    return self.error_expr(span);
                };
                if stmt {
                    self.err(
                        codes::NOT_CALLABLE,
                        id.span,
                        format!("`{}` is a property, not a procedure", id.name),
                    );
                    return self.error_expr(span);
                }
                self.call_proc(get, owner, receiver, span, args, false)
            }
            Symbol::Declare(d) => {
                let di = &sema.program.declares[d.index()];
                if !di.is_function && !stmt {
                    self.err(
                        codes::NOT_CALLABLE,
                        span,
                        format!("`{}` is a Sub and does not return a value", di.name),
                    );
                    return self.error_expr(span);
                }
                let sig = Sig::of_declare(di);
                let bargs = self.match_args(&sig, args.unwrap_or(&[]), span);
                let ty = if di.is_function {
                    di.ret.clone()
                } else {
                    Type::Variant
                };
                self.call_expr(span, ty, CallTarget::Declare(d), None, bargs)
            }
            Symbol::Enum(_) | Symbol::Udt(_) => {
                self.err(
                    codes::NOT_CALLABLE,
                    id.span,
                    format!("`{}` is a type and cannot be used as a value", id.name),
                );
                self.error_expr(span)
            }
            Symbol::Event(_) => {
                self.err(
                    codes::EVENTS,
                    id.span,
                    format!("`{}` is an event; use `RaiseEvent`", id.name),
                );
                self.error_expr(span)
            }
        }
    }

    fn call_proc(
        &mut self,
        p: ProcId,
        owner: ModuleId,
        receiver: Option<BExpr>,
        span: Span,
        args: Option<&[Arg]>,
        stmt: bool,
    ) -> BExpr {
        let sema = self.sema;
        let info = &sema.program.procs[p.index()];
        if !info.has_result && !stmt {
            self.err(
                codes::NOT_CALLABLE,
                span,
                format!("{} does not return a value", info.describe()),
            );
            if let Some(a) = args {
                for x in a {
                    if let Some(v) = &x.value {
                        let _ = self.bind_expr(v);
                    }
                }
            }
            return self.error_expr(span);
        }
        let recv = match receiver {
            Some(r) => Some(r),
            None if sema.program.modules[owner.index()].is_class_like() => Some(self.me_expr(span)),
            None => None,
        };
        let sig = Sig::of_proc(info);
        let bargs = self.match_args(&sig, args.unwrap_or(&[]), span);
        let ty = if info.has_result {
            info.ret.clone()
        } else {
            Type::Variant
        };
        self.call_expr(span, ty, CallTarget::Proc(p), recv, bargs)
    }

    /// `base` followed by an argument list: an array index, a default-member
    /// invocation or a late-bound index.
    pub(crate) fn var_use(&mut self, base: BExpr, span: Span, args: Option<&[Arg]>) -> BExpr {
        match args {
            None => base,
            Some(a) => self.apply_args(base, a, span),
        }
    }

    pub(crate) fn apply_args(&mut self, base: BExpr, args: &[Arg], span: Span) -> BExpr {
        if base.ty.is_error() {
            for a in args {
                if let Some(v) = &a.value {
                    let _ = self.bind_expr(v);
                }
            }
            return base;
        }
        match base.ty.clone() {
            Type::Array { elem, rank } => {
                if args.is_empty() {
                    return base;
                }
                let mut indices = Vec::new();
                let mut ok = true;
                for a in args {
                    if a.name.is_some() {
                        self.err(
                            codes::ARRAY_RULE,
                            a.span,
                            "array subscripts cannot be named",
                        );
                        ok = false;
                        continue;
                    }
                    let Some(v) = &a.value else {
                        self.err(codes::ARRAY_RULE, a.span, "a subscript is missing");
                        ok = false;
                        continue;
                    };
                    if matches!(v.kind, ExprKind::Range { .. }) {
                        self.err(
                            codes::ARRAY_RULE,
                            v.span,
                            "a `To` range is not a valid subscript",
                        );
                        ok = false;
                        continue;
                    }
                    let b = self.bind_expr(v);
                    indices.push(self.coerce_index(b, Type::Long));
                }
                if !ok {
                    return self.error_expr(span);
                }
                if let Some(r) = rank
                    && usize::from(r) != indices.len()
                {
                    self.err(
                        codes::ARRAY_RULE,
                        span,
                        format!(
                            "wrong number of subscripts: the array has {r} dimension(s), {} given",
                            indices.len()
                        ),
                    );
                    return self.error_expr(span);
                }
                BExpr::new(
                    span,
                    *elem,
                    BExprKind::Index {
                        base: Box::new(base),
                        indices,
                    },
                )
            }
            Type::Variant | Type::Object => {
                let la = self.late_args(args);
                self.late(base, LateMember::Default, la, Type::Variant, None)
            }
            Type::Class(m) => {
                let mi = &self.sema.program.modules[m.index()];
                match mi.default_member.clone() {
                    Some(name) => {
                        // MS-VBAL 5.6.13: a default member that takes fewer
                        // parameters than there are arguments and returns an
                        // object is applied first; the surplus arguments go
                        // to the default member of its result.
                        let own = self
                            .default_member_arity(m, &name)
                            .filter(|&n| args.len() > n && args.iter().all(|a| a.name.is_none()));
                        let ident = Ident {
                            name,
                            suffix: None,
                            kind: vba_ast::IdentKind::Plain,
                            span,
                        };
                        match own {
                            Some(n) => {
                                let first = self.member_of(base, &ident, Some(&args[..n]), false);
                                self.apply_args(first, &args[n..], span)
                            }
                            None => self.member_of(base, &ident, Some(args), false),
                        }
                    }
                    None if self.host_class_of(m).is_some() => {
                        let lt = self.host_class_of(m).expect("host class");
                        self.host_class_index(base, m, lt, args, span)
                    }
                    None if matches!(
                        mi.kind,
                        vba_project::ModuleKind::Designer | vba_project::ModuleKind::Document
                    ) =>
                    {
                        // `Me!Name`, `Me("Name")`: the default member of the
                        // host-provided base class.
                        self.sema.report(
                            vba_diagnostics::Severity::Info,
                            self.module,
                            codes::LATE_BOUND_LIBRARY_MEMBER,
                            span,
                            format!(
                                "`{}` does not declare a default member; it is assumed to be provided by the host base class and is bound late",
                                mi.name
                            ),
                        );
                        let la = self.late_args(args);
                        self.late(base, LateMember::Default, la, Type::Variant, None)
                    }
                    None => {
                        self.err(
                            codes::NOT_CALLABLE,
                            span,
                            format!("class `{}` has no default member", mi.name),
                        );
                        self.error_expr(span)
                    }
                }
            }
            Type::Library(lt) => match self.sema.provider.default_member(&lt) {
                Some(m) => {
                    let ident = Ident {
                        name: Name::new(&m.name),
                        suffix: None,
                        kind: vba_ast::IdentKind::Plain,
                        span,
                    };
                    self.member_of(base, &ident, Some(args), false)
                }
                None => {
                    self.info_late(span, &lt);
                    let la = self.late_args(args);
                    self.late(base, LateMember::Default, la, Type::Variant, Some(lt))
                }
            },
            other => {
                self.err(
                    codes::NOT_CALLABLE,
                    span,
                    format!(
                        "a value of type {} cannot be called or indexed",
                        self.tn(&other)
                    ),
                );
                for a in args {
                    if let Some(v) = &a.value {
                        let _ = self.bind_expr(v);
                    }
                }
                self.error_expr(span)
            }
        }
    }

    /// The number of parameters of the default member `name` of class `m`
    /// when it is a procedure or property getter whose result can be an
    /// object (so that surplus arguments may go to the result's default
    /// member).
    fn default_member_arity(&self, m: ModuleId, name: &Name) -> Option<usize> {
        let sema = self.sema;
        let (sym, _) = sema.program.modules[m.index()].symbols.get(name).copied()?;
        let p = match sym {
            Symbol::Proc(p) => p,
            Symbol::Property(g) => g.get?,
            _ => return None,
        };
        let info = &sema.program.procs[p.index()];
        let object_like = matches!(
            info.ret,
            Type::Class(_) | Type::Object | Type::Variant | Type::Library(_)
        );
        (info.has_result && object_like && !info.params.iter().any(|q| q.param_array))
            .then_some(info.params.len())
    }

    // ---- member access ------------------------------------------------------

    fn bind_member(
        &mut self,
        target: Option<&Expr>,
        name: &Ident,
        span: Span,
        args: Option<&[Arg]>,
        stmt: bool,
    ) -> BExpr {
        let Some(t) = target else {
            return match self.withs.last() {
                Some(w) => {
                    let recv = w.receiver.clone();
                    self.member_of(recv, name, args, stmt)
                }
                None => {
                    self.err(
                        codes::NO_OBJECT_CONTEXT,
                        span,
                        format!("`.{}` is only valid inside a `With` block", name.name),
                    );
                    self.error_expr(span)
                }
            };
        };
        if let Some(ns) = self.try_namespace(t) {
            return self.bind_in_namespace(ns, name, span, args, stmt);
        }
        // `VBA.Left`, `Strings.Left`: qualified intrinsic names.
        if let ExprKind::Name(q) = &t.kind
            && !self.scope.contains_key(&q.name)
            && matches!(
                self.sema.lookup_module_scope(self.module, &q.name),
                Found::None
            )
            && let Some(r) = vba_stdlib::lookup(&format!(
                "{}.{}{}",
                q.name,
                name.name,
                if name.suffix == Some(TypeSuffix::String) {
                    "$"
                } else {
                    ""
                }
            ))
        {
            return self.call_intrinsic(r, span, args);
        }
        // `Mk.X` inside `Function Mk As P` (P a user-defined type) names a
        // field of the result variable, not a recursive call.
        let result_udt = match &t.kind {
            ExprKind::Name(id)
                if self.info().has_result
                    && id.name == self.info().name
                    && !self.scope.contains_key(&id.name) =>
            {
                self.result_local
                    .filter(|r| matches!(self.local_ty(*r), Type::Udt(_) | Type::LibRecord(_)))
            }
            _ => None,
        };
        let recv = match &t.kind {
            _ if let Some(r) = result_udt => self.local_expr(r, t.span),
            ExprKind::Name(id) if self.is_host_name(id) => self.host_member(id, t.span, None),
            _ => self.bind_expr(t),
        };
        self.member_of(recv, name, args, stmt)
    }

    /// Whether the module is a document or designer module, whose base class
    /// (the form, the worksheet) and controls come from the host.
    fn host_backed(&self) -> bool {
        matches!(
            self.sema.program.modules[self.module.index()].kind,
            vba_project::ModuleKind::Designer | vba_project::ModuleKind::Document
        )
    }

    /// An unresolved simple name in a host-backed module.
    fn is_host_name(&self, id: &Ident) -> bool {
        self.host_backed()
            && self.host_class().is_none()
            && !id.is_missing()
            && !id.name.eq_ignore_case("Me")
            && !self.scope.contains_key(&id.name)
            && matches!(
                self.sema.lookup_module_scope(self.module, &id.name),
                Found::None
            )
            && lookup_intrinsic(id).is_none()
            && self
                .sema
                .provider
                .global(&id.name.to_string_lossy())
                .is_none()
    }

    /// A member of the host-provided base class of a document or designer
    /// module (a control, `Caption`, `Show`...), bound late. The analysis
    /// cannot verify it without the form description, so it says so.
    fn host_member(&mut self, id: &Ident, span: Span, args: Option<&[Arg]>) -> BExpr {
        self.sema.report(
            vba_diagnostics::Severity::Info,
            self.module,
            codes::LATE_BOUND_LIBRARY_MEMBER,
            id.span,
            format!(
                "`{}` is not declared in this module; it is assumed to be a host-provided member (control or base-class member) and is bound late",
                id.name
            ),
        );
        let me = self.me_expr(span);
        let la = self.late_args(args.unwrap_or(&[]));
        self.late(
            me,
            LateMember::Named(id.name.clone()),
            la,
            Type::Variant,
            None,
        )
    }

    fn try_namespace(&mut self, t: &Expr) -> Option<Namespace> {
        let sema = self.sema;
        match &t.kind {
            ExprKind::Name(id) => {
                if id.name.eq_ignore_case("Me") || self.scope.contains_key(&id.name) {
                    return None;
                }
                match sema.lookup_module_scope(self.module, &id.name) {
                    Found::Sym(_, Symbol::Enum(e)) => Some(Namespace::Enum(e)),
                    Found::Sym(..) | Found::Ambiguous(_) => None,
                    Found::Module(m) => Some(Namespace::Module(m)),
                    Found::None => {
                        let text = id.name.to_string_lossy();
                        if id.name == sema.project_name && sema.provider.global(&text).is_none() {
                            Some(Namespace::Project)
                        } else if let Some(Type::LibEnum(lt)) =
                            sema.provider.resolve_declared(None, &text)
                        {
                            Some(Namespace::LibEnum(lt))
                        } else if crate::builtins::is_vba_enum_type(&text) {
                            // `VbVarType.vbString`: the constants live in
                            // the library's global scope.
                            Some(Namespace::Library(crate::builtins::VBA_LIBRARY.to_owned()))
                        } else if sema.provider.has_library(&text) {
                            Some(Namespace::Library(text))
                        } else {
                            None
                        }
                    }
                }
            }
            ExprKind::Member {
                target: Some(inner),
                name,
            } => match self.try_namespace(inner)? {
                Namespace::Project => match sema.module_ids.get(&name.name) {
                    Some(m) => Some(Namespace::Module(*m)),
                    None => match sema.project_symbol(&name.name) {
                        Some((_, Symbol::Enum(e))) => Some(Namespace::Enum(e)),
                        _ => None,
                    },
                },
                Namespace::Module(m) => {
                    match sema.program.modules[m.index()].symbols.get(&name.name) {
                        Some((Symbol::Enum(e), _)) => Some(Namespace::Enum(*e)),
                        _ => None,
                    }
                }
                Namespace::Library(q) => {
                    let member = name.name.to_string_lossy();
                    if q.eq_ignore_ascii_case(crate::builtins::VBA_LIBRARY)
                        && (crate::builtins::is_vba_module(&member)
                            || crate::builtins::is_vba_enum_type(&member))
                    {
                        // `VBA.Strings.Left`, `VBA.VbVarType.vbString`: the
                        // modules and enumerations add nothing to the
                        // library's global scope.
                        return Some(Namespace::Library(q));
                    }
                    match sema
                        .provider
                        .resolve_declared(Some(&q), &name.name.to_string_lossy())
                    {
                        Some(Type::LibEnum(lt)) => Some(Namespace::LibEnum(lt)),
                        _ => None,
                    }
                }
                Namespace::Enum(_) | Namespace::LibEnum(_) => None,
            },
            _ => None,
        }
    }

    fn bind_in_namespace(
        &mut self,
        ns: Namespace,
        name: &Ident,
        span: Span,
        args: Option<&[Arg]>,
        stmt: bool,
    ) -> BExpr {
        let sema = self.sema;
        match ns {
            Namespace::Project => {
                // `Project.Module` (the default instance of a predeclared
                // class) or `Project.Name` for a public name of any module.
                if let Some(m) = sema.module_ids.get(&name.name) {
                    return self.module_as_value(*m, name, span, args);
                }
                if let Some((m, _)) = sema.project_symbol(&name.name) {
                    return self.bind_in_namespace(Namespace::Module(m), name, span, args, stmt);
                }
                self.err(
                    codes::NO_SUCH_MEMBER,
                    name.span,
                    format!("project has no module or public name `{}`", name.name),
                );
                self.error_expr(span)
            }
            Namespace::LibEnum(lt) => {
                let text = name.name.to_string_lossy();
                let member = sema
                    .provider
                    .enum_members(&lt)
                    .and_then(|ms| ms.into_iter().find(|(n, _)| n.eq_ignore_ascii_case(&text)));
                match member {
                    Some((_, v)) if args.is_none() => {
                        sema.note_type(&Type::LibEnum(lt.clone()));
                        BExpr::new(
                            span,
                            Type::LibEnum(lt),
                            BExprKind::Const(Constant::Value(Variant::Long(v as i32))),
                        )
                    }
                    Some(_) => {
                        self.err(
                            codes::NOT_CALLABLE,
                            span,
                            "an enum member cannot be called or indexed",
                        );
                        self.error_expr(span)
                    }
                    None => {
                        self.err(
                            codes::NO_SUCH_MEMBER,
                            name.span,
                            format!("enum `{lt}` has no member `{}`", name.name),
                        );
                        self.error_expr(span)
                    }
                }
            }
            Namespace::Library(q) => {
                let text = name.name.to_string_lossy();
                if sema.provider.global(&text).is_some() || lookup_intrinsic(name).is_some() {
                    return self.bind_library_name(name, span, args, stmt);
                }
                self.err(
                    codes::NO_SUCH_MEMBER,
                    name.span,
                    format!("library `{q}` has no global `{}`", name.name),
                );
                self.error_expr(span)
            }
            Namespace::Enum(e) => match sema.enum_member_index(e, &name.name) {
                Some(i) => self.use_symbol(
                    sema.program.enums[e.index()].module,
                    Symbol::EnumMember(e, i as u32),
                    name,
                    span,
                    args,
                    stmt,
                    None,
                ),
                None => {
                    self.err(
                        codes::NO_SUCH_MEMBER,
                        name.span,
                        format!(
                            "enum `{}` has no member `{}`",
                            sema.program.enums[e.index()].name,
                            name.name
                        ),
                    );
                    self.error_expr(span)
                }
            },
            Namespace::Module(m) => {
                let mi = &sema.program.modules[m.index()];
                let Some((sym, _)) = mi.symbols.get(&name.name).copied() else {
                    if mi.predeclared
                        && matches!(
                            mi.kind,
                            vba_project::ModuleKind::Designer | vba_project::ModuleKind::Document
                        )
                    {
                        let inst = BExpr::new(span, Type::Class(m), BExprKind::DefaultInstance(m));
                        return self.member_of(inst, name, args, stmt);
                    }
                    self.err(
                        codes::NO_SUCH_MEMBER,
                        name.span,
                        format!("module `{}` has no member `{}`", mi.name, name.name),
                    );
                    return self.error_expr(span);
                };
                let own = m == self.module;
                let public = sema.symbol_public(sym);
                if !own && !public {
                    self.err(
                        codes::NOT_ACCESSIBLE,
                        name.span,
                        format!("`{}` is private to module `{}`", name.name, mi.name),
                    );
                    return self.error_expr(span);
                }
                if mi.is_class_like() {
                    match sym {
                        Symbol::Var(_) | Symbol::Proc(_) | Symbol::Property(_) => {
                            if mi.predeclared {
                                let inst =
                                    BExpr::new(span, Type::Class(m), BExprKind::DefaultInstance(m));
                                return self.use_symbol(m, sym, name, span, args, stmt, Some(inst));
                            }
                            self.err(
                                codes::NOT_ACCESSIBLE,
                                name.span,
                                format!(
                                    "`{}` is a member of class `{}`: it needs an instance",
                                    name.name, mi.name
                                ),
                            );
                            return self.error_expr(span);
                        }
                        _ => {}
                    }
                }
                self.use_symbol(m, sym, name, span, args, stmt, None)
            }
        }
    }

    /// `receiver.name[(args)]` for a receiver that is a value.
    pub(crate) fn member_of(
        &mut self,
        receiver: BExpr,
        name: &Ident,
        args: Option<&[Arg]>,
        stmt: bool,
    ) -> BExpr {
        let span = receiver.span.merge(name.span);
        let sema = self.sema;
        if name.is_missing() {
            return self.error_expr(span);
        }
        match receiver.ty.clone() {
            Type::Error => {
                for a in args.unwrap_or(&[]) {
                    if let Some(v) = &a.value {
                        let _ = self.bind_expr(v);
                    }
                }
                self.error_expr(span)
            }
            Type::Class(m) => {
                let mi = &sema.program.modules[m.index()];
                match mi.symbols.get(&name.name).copied() {
                    Some((sym, _))
                        if matches!(
                            sym,
                            Symbol::Var(_) | Symbol::Proc(_) | Symbol::Property(_)
                        ) =>
                    {
                        if m != self.module && !sema.symbol_public(sym) {
                            self.err(
                                codes::NOT_ACCESSIBLE,
                                name.span,
                                format!("`{}` is private to class `{}`", name.name, mi.name),
                            );
                            for a in args.unwrap_or(&[]) {
                                if let Some(v) = &a.value {
                                    let _ = self.bind_expr(v);
                                }
                            }
                            return self.error_expr(span);
                        }
                        if let Some(sfx) = name.suffix {
                            let declared = match sym {
                                Symbol::Var(g) => Some(sema.program.globals[g.index()].ty.clone()),
                                Symbol::Proc(p) => {
                                    let i = &sema.program.procs[p.index()];
                                    i.has_result.then(|| i.ret.clone())
                                }
                                Symbol::Property(g) => {
                                    g.get.map(|p| sema.program.procs[p.index()].ret.clone())
                                }
                                _ => None,
                            };
                            // MS-VBAL 5.6.12: a type character must match the
                            // declared type of the member.
                            if let Some(d) = declared
                                && !d.is_error()
                                && !d.is_variant()
                                && d != sema.suffix_type(sfx)
                            {
                                self.spec(
                                    codes::TYPE_MISMATCH,
                                    name.span,
                                    format!(
                                        "the type character does not match the declared type of `{}`",
                                        name.name
                                    ),
                                );
                                if sema.strict_spec {
                                    for a in args.unwrap_or(&[]) {
                                        if let Some(v) = &a.value {
                                            let _ = self.bind_expr(v);
                                        }
                                    }
                                    return self.error_expr(span);
                                }
                            }
                        }
                        self.use_symbol(m, sym, name, span, args, stmt, Some(receiver))
                    }
                    _ if self.host_class_of(m).is_some() => {
                        let lt = self.host_class_of(m).expect("host class");
                        self.host_class_member(receiver, m, lt, name, args, stmt)
                    }
                    _ if matches!(
                        mi.kind,
                        vba_project::ModuleKind::Designer | vba_project::ModuleKind::Document
                    ) =>
                    {
                        // Members of the host-provided base class.
                        self.sema.report(
                            vba_diagnostics::Severity::Info,
                            self.module,
                            codes::LATE_BOUND_LIBRARY_MEMBER,
                            name.span,
                            format!(
                                "`{}` is not declared in module `{}`; it is assumed to be a host-provided member and is bound late",
                                name.name, mi.name
                            ),
                        );
                        let la = self.late_args(args.unwrap_or(&[]));
                        self.late(
                            receiver,
                            LateMember::Named(name.name.clone()),
                            la,
                            Type::Variant,
                            None,
                        )
                    }
                    _ => {
                        self.err(
                            codes::NO_SUCH_MEMBER,
                            name.span,
                            format!("class `{}` has no member `{}`", mi.name, name.name),
                        );
                        for a in args.unwrap_or(&[]) {
                            if let Some(v) = &a.value {
                                let _ = self.bind_expr(v);
                            }
                        }
                        self.error_expr(span)
                    }
                }
            }
            Type::Library(lt) => self.member_of_lib(receiver, lt, name, args, stmt, span),
            Type::LibRecord(lt) => {
                let Some(u) = sema.lib_record_id(&lt) else {
                    self.err(
                        codes::NO_SUCH_MEMBER,
                        name.span,
                        format!("record `{lt}` is not described by any provider"),
                    );
                    return self.error_expr(span);
                };
                match sema.lib_record_field(u, &name.name) {
                    Some((index, ty)) => {
                        let b = BExpr::new(
                            span,
                            ty,
                            BExprKind::UdtField {
                                base: Box::new(receiver),
                                udt: u,
                                index,
                            },
                        );
                        self.var_use(b, span, args)
                    }
                    None => {
                        self.err(
                            codes::NO_SUCH_MEMBER,
                            name.span,
                            format!("record `{lt}` has no field `{}`", name.name),
                        );
                        self.error_expr(span)
                    }
                }
            }
            Type::Object | Type::Variant => {
                let la = self.late_args(args.unwrap_or(&[]));
                self.late(
                    receiver,
                    LateMember::Named(name.name.clone()),
                    la,
                    Type::Variant,
                    None,
                )
            }
            Type::Udt(u) => {
                let ui = &sema.program.udts[u.index()];
                match ui.fields.iter().position(|f| f.name == name.name) {
                    Some(i) => {
                        let ty = ui.fields[i].ty.clone();
                        let b = BExpr::new(
                            span,
                            ty,
                            BExprKind::UdtField {
                                base: Box::new(receiver),
                                udt: u,
                                index: i as u32,
                            },
                        );
                        self.var_use(b, span, args)
                    }
                    None => {
                        self.err(
                            codes::NO_SUCH_MEMBER,
                            name.span,
                            format!("type `{}` has no field `{}`", ui.name, name.name),
                        );
                        self.error_expr(span)
                    }
                }
            }
            other => {
                self.err(
                    codes::NO_SUCH_MEMBER,
                    name.span,
                    format!("a value of type {} has no members", self.tn(&other)),
                );
                self.error_expr(span)
            }
        }
    }

    pub(crate) fn member_of_lib(
        &mut self,
        receiver: BExpr,
        lt: LibraryType,
        name: &Ident,
        args: Option<&[Arg]>,
        stmt: bool,
        span: Span,
    ) -> BExpr {
        let provider = self.sema.provider;
        match provider.member(&lt, &name.name.to_string_lossy()) {
            Some(m) => {
                if m.is_sub && !stmt {
                    self.err(
                        codes::NOT_CALLABLE,
                        name.span,
                        format!("`{}` is a Sub and does not return a value", m.name),
                    );
                    return self.error_expr(span);
                }
                if m.kind == LibMemberKind::PropertyPut && self.target_span != Some(name.span) {
                    self.err(
                        codes::WRITE_ONLY_PROPERTY,
                        name.span,
                        format!("property `{}` of `{lt}` is write-only", m.name),
                    );
                    return self.error_expr(span);
                }
                self.sema.note_type(&m.ty);
                // More arguments than the member declares: when it returns
                // an object (or a Variant that may hold one), the surplus
                // goes to the default member of the result, as in
                // `rs.Fields("Name")`.
                let all_args = args.unwrap_or(&[]);
                let chain = !m.is_sub
                    && all_args.len() > m.params.len()
                    && !m.params.iter().any(|p| p.param_array)
                    && all_args.iter().all(|a| a.name.is_none())
                    && matches!(m.ty, Type::Library(_) | Type::Object | Type::Variant);
                // The result of a fully described type without a default
                // member cannot take them: the call has too many arguments.
                let no_default = chain
                    && matches!(&m.ty, Type::Library(rt)
                        if provider.describes_members(rt) && provider.default_member(rt).is_none());
                let (own_args, tail) = if chain {
                    all_args.split_at(m.params.len())
                } else {
                    (all_args, &[][..])
                };
                let sig = Sig::of_lib(&m);
                let bargs = self.match_args(&sig, own_args, span);
                let la = bargs
                    .into_iter()
                    .map(|arg| BLateArg { name: None, arg })
                    .collect();
                let ty = if m.is_sub {
                    Type::Variant
                } else {
                    m.ty.clone()
                };
                let l = self.late(
                    receiver,
                    LateMember::Named(Name::new(&m.name)),
                    la,
                    ty,
                    Some(lt),
                );
                let l = with_dispid(l, m.dispid);
                if tail.is_empty() {
                    l
                } else if no_default {
                    self.err(
                        codes::BAD_ARGUMENTS,
                        tail[0].span,
                        format!(
                            "wrong number of arguments: `{}` takes at most {} and its result `{}` has no default member to take the {} surplus",
                            m.name,
                            m.params.len(),
                            self.tn(&m.ty),
                            tail.len(),
                        ),
                    );
                    for a in tail {
                        if let Some(v) = &a.value {
                            let _ = self.bind_expr(v);
                        }
                    }
                    self.error_expr(span)
                } else {
                    self.apply_args(l, tail, span)
                }
            }
            None if provider.describes_members(&lt) => {
                self.err(
                    codes::NO_SUCH_MEMBER,
                    name.span,
                    format!("type `{lt}` has no member `{}`", name.name),
                );
                for a in args.unwrap_or(&[]) {
                    if let Some(v) = &a.value {
                        let _ = self.bind_expr(v);
                    }
                }
                self.error_expr(span)
            }
            None => {
                self.sema.report(
                    vba_diagnostics::Severity::Info,
                    self.module,
                    codes::LATE_BOUND_LIBRARY_MEMBER,
                    name.span,
                    format!(
                        "member `{}` of library type `{lt}` could not be verified; it is bound late",
                        name.name
                    ),
                );
                let la = self.late_args(args.unwrap_or(&[]));
                self.late(
                    receiver,
                    LateMember::Named(name.name.clone()),
                    la,
                    Type::Variant,
                    Some(lt),
                )
            }
        }
    }

    /// `target!name`: dictionary-style access through the default member.
    fn bind_dict(
        &mut self,
        target: Option<&Expr>,
        name: &Ident,
        span: Span,
        args: Option<&[Arg]>,
    ) -> BExpr {
        let recv = match target {
            Some(t) => self.bind_expr(t),
            None => match self.withs.last() {
                Some(w) => w.receiver.clone(),
                None => {
                    self.err(
                        codes::NO_OBJECT_CONTEXT,
                        span,
                        format!("`!{}` is only valid inside a `With` block", name.name),
                    );
                    return self.error_expr(span);
                }
            },
        };
        if recv.ty.is_error() {
            return recv;
        }
        let key = Arg {
            span: name.span,
            name: None,
            value: Some(Expr {
                span: name.span,
                kind: ExprKind::Literal(vba_ast::Literal::String(name.name.as_bytes().to_vec())),
            }),
        };
        let b = match recv.ty.clone() {
            Type::Class(_) | Type::Library(_) | Type::Object | Type::Variant => {
                self.apply_args(recv, std::slice::from_ref(&key), span)
            }
            other => {
                self.err(
                    codes::NOT_CALLABLE,
                    span,
                    format!("`!` access needs an object, not {}", self.tn(&other)),
                );
                return self.error_expr(span);
            }
        };
        match args {
            Some(a) => self.apply_args(b, a, span),
            None => b,
        }
    }
}

/// Why `AddressOf` cannot name this procedure, or `None` when it can.
///
/// The operand must be a `Sub` or `Function` of a standard module (MS-VBAL
/// 5.6.16.8, AddressOf Expressions, tracked in `docs/language/spec-coverage.md`;
/// the repository records no further normative text for it, so the shape rules
/// follow the Microsoft documentation of the `AddressOf` operator). A callback
/// has a fixed native signature, so `Optional` and `ParamArray` parameters have
/// no meaning for it. Whether the parameter list fits the function that
/// receives the pointer is known only to that function and is checked when the
/// pointer is used.
fn callback_restriction(sema: &crate::sema::Sema<'_>, p: ProcId) -> Option<String> {
    let info = &sema.program.procs[p.index()];
    if !matches!(
        info.kind,
        vba_ast::ProcKind::Sub | vba_ast::ProcKind::Function
    ) {
        return Some(format!(
            "`{}` is a property procedure, not a Sub or Function",
            info.name
        ));
    }
    if sema.program.module(info.module).is_class_like() {
        return Some(format!(
            "`{}` belongs to a class, document or designer module; the procedure must be in a standard module",
            info.name
        ));
    }
    if let Some(q) = info.params.iter().find(|q| q.optional || q.param_array) {
        return Some(format!(
            "parameter `{}` is {}, and a callback has a fixed signature",
            q.name,
            if q.param_array {
                "a ParamArray"
            } else {
                "Optional"
            }
        ));
    }
    None
}
