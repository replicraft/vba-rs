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

use super::call::Sig;
use super::{Binder, LocalSym};

/// A qualifier that is not a value.
enum Namespace {
    Module(ModuleId),
    Enum(EnumId),
    Project,
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
                if matches!(o.ty, Type::Udt(_) | Type::Array { .. } | Type::Nothing) {
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
                    Type::Library(lt) => BExpr::new(
                        span,
                        Type::Library(lt.clone()),
                        BExprKind::New(NewTarget::Library(lt)),
                    ),
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
                Type::Udt(_) | Type::Array { .. } | Type::Nothing | Type::Any
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
        let ExprKind::Name(id) = &inner.kind else {
            self.err(
                codes::NOT_CALLABLE,
                span,
                "`AddressOf` requires the name of a procedure",
            );
            return self.error_expr(span);
        };
        let sema = self.sema;
        match sema.lookup_module_scope(self.module, &id.name) {
            Found::Sym(_, Symbol::Proc(p)) => {
                BExpr::new(span, ty, BExprKind::AddressOf(AddressTarget::Proc(p)))
            }
            Found::Sym(_, Symbol::Declare(d)) => {
                BExpr::new(span, ty, BExprKind::AddressOf(AddressTarget::Declare(d)))
            }
            Found::Ambiguous(ms) => {
                sema.ambiguous(self.module, id.span, &id.name.to_string(), &ms);
                self.error_expr(span)
            }
            _ => {
                self.err(
                    codes::NOT_CALLABLE,
                    id.span,
                    format!(
                        "`AddressOf` requires a Sub or Function, but `{}` is not one",
                        id.name
                    ),
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
        if id.suffix.is_none()
            && let Some(g) = self.sema.provider.global(&id.name.to_string_lossy())
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
        if (args.is_some() || stmt) && self.is_host_name(id) {
            return self.host_member(id, span, args);
        }
        if args.is_none() && !stmt {
            if self.explicit {
                self.err(
                    codes::UNDECLARED_NAME,
                    id.span,
                    format!("variable `{}` is not defined (Option Explicit)", id.name),
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
            format!("`{}` is not defined (Sub or Function not defined)", id.name),
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
        let sig = Sig::of_intrinsic(r.info, r.dollar);
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
        self.call_expr(
            span,
            ty,
            CallTarget::Intrinsic {
                name: r.info.name.to_owned(),
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
                        let ident = Ident {
                            name,
                            suffix: None,
                            kind: vba_ast::IdentKind::Plain,
                            span,
                        };
                        self.member_of(base, &ident, Some(args), false)
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
                    .filter(|r| matches!(self.local_ty(*r), Type::Udt(_)))
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
                    Found::None => (id.name == sema.project_name
                        && sema.provider.global(&id.name.to_string_lossy()).is_none())
                    .then_some(Namespace::Project),
                }
            }
            ExprKind::Member {
                target: Some(inner),
                name,
            } => match self.try_namespace(inner)? {
                Namespace::Project => sema
                    .module_ids
                    .get(&name.name)
                    .map(|m| Namespace::Module(*m)),
                Namespace::Module(m) => {
                    match sema.program.modules[m.index()].symbols.get(&name.name) {
                        Some((Symbol::Enum(e), _)) => Some(Namespace::Enum(*e)),
                        _ => None,
                    }
                }
                Namespace::Enum(_) => None,
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
                self.err(
                    codes::NO_SUCH_MEMBER,
                    name.span,
                    format!("project has no module `{}`", name.name),
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
                        self.use_symbol(m, sym, name, span, args, stmt, Some(receiver))
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

    fn member_of_lib(
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
                let sig = Sig::of_lib(&m);
                let bargs = self.match_args(&sig, args.unwrap_or(&[]), span);
                let la = bargs
                    .into_iter()
                    .map(|arg| BLateArg { name: None, arg })
                    .collect();
                let ty = if m.is_sub {
                    Type::Variant
                } else {
                    m.ty.clone()
                };
                let _ = LibMemberKind::Method;
                self.late(
                    receiver,
                    LateMember::Named(Name::new(&m.name)),
                    la,
                    ty,
                    Some(lt),
                )
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
