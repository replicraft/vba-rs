//! Assignable locations: converting bound expressions to places and
//! binding assignment targets.

use vba_ast::{Arg, Expr, ExprKind, Ident, ProcKind};
use vba_source::Span;

use crate::bound::*;
use crate::codes;
use crate::program::Symbol;
use crate::provider::LibMemberKind;
use crate::sema::Found;
use crate::types::*;

use super::Binder;
use super::call::Sig;

impl Binder<'_, '_> {
    /// The storage location denoted by `e`, if it denotes one (a variable,
    /// field, array element or UDT member): the places that can be passed by
    /// reference.
    pub(crate) fn expr_to_place(&self, e: &BExpr) -> Option<BPlace> {
        match &e.kind {
            BExprKind::Local(l) => Some(BPlace::Local(*l)),
            BExprKind::Global(g) => Some(BPlace::Global(*g)),
            BExprKind::Field { receiver, global } => Some(BPlace::Field {
                receiver: receiver.clone(),
                global: *global,
            }),
            BExprKind::Index { base, indices } => Some(BPlace::Index {
                base: Box::new(self.expr_to_place(base)?),
                indices: indices.clone(),
                elem: e.ty.clone(),
            }),
            BExprKind::UdtField { base, udt, index } => Some(BPlace::UdtField {
                base: Box::new(self.expr_to_place(base)?),
                udt: *udt,
                index: *index,
                ty: e.ty.clone(),
            }),
            _ => None,
        }
    }

    /// The location `v(i)` of an element of the array a `Variant` location
    /// may hold, for `v(i)(j) = x`. Only integral subscripts qualify: the
    /// run time falls back to the default member of an object for them.
    fn late_element_place(&self, e: &BExpr) -> Option<BPlace> {
        let BExprKind::Late(l) = &e.kind else {
            return None;
        };
        if l.kind != LateKind::Get
            || l.member != LateMember::Default
            || l.dispid.is_some()
            || l.receiver.ty != Type::Variant
            || l.args.is_empty()
        {
            return None;
        }
        let mut indices = Vec::with_capacity(l.args.len());
        for a in &l.args {
            match (&a.name, &a.arg) {
                (None, BArg::Value(x)) if x.ty.is_integral() => indices.push(x.clone()),
                _ => return None,
            }
        }
        let base = self
            .expr_to_place(&l.receiver)
            .or_else(|| self.late_element_place(&l.receiver))?;
        Some(BPlace::Index {
            base: Box::new(base),
            indices,
            elem: Type::Variant,
        })
    }

    /// Binds the target of `Let`/`Set`/`LSet`/`RSet`. `set` selects
    /// `Property Set` / `DISPATCH_PROPERTYPUTREF`.
    pub(crate) fn bind_target(&mut self, e: &Expr, set: bool) -> Option<(BPlace, Type)> {
        // MS-VBAL 5.6.6: a parenthesized expression is a value, not a location.
        if matches!(e.kind, ExprKind::Paren(_)) {
            self.spec(
                codes::NOT_ASSIGNABLE,
                e.span,
                "a parenthesized expression cannot be assigned to",
            );
            // Lenient mode: assign to the inner expression.
            if let ExprKind::Paren(inner) = &e.kind
                && !self.sema.strict_spec
            {
                return self.bind_target(inner, set);
            }
            return None;
        }
        // The implicit result variable of a function or property getter.
        let name_of = |x: &Expr| match &x.kind {
            ExprKind::Name(id) => Some(id.clone()),
            _ => None,
        };
        if let Some(id) = name_of(e)
            && self.info().has_result
            && id.name == self.info().name
            && !self.scope.contains_key(&id.name)
            && let Some(r) = self.result_local
        {
            let ty = self.local_ty(r);
            return Some((BPlace::Local(r), ty));
        }
        if let Some(r) = self.try_write_only_property(e, set) {
            return r;
        }
        let name_span = match &e.kind {
            ExprKind::Member { name, .. } => Some(name.span),
            ExprKind::Name(id) => Some(id.span),
            ExprKind::Call { callee, .. } => match &callee.kind {
                ExprKind::Member { name, .. } => Some(name.span),
                _ => None,
            },
            _ => None,
        };
        let saved = std::mem::replace(&mut self.target_span, name_span);
        let b = self.bind_expr(e);
        self.target_span = saved;
        if b.ty.is_error() {
            return None;
        }
        self.place_for_assignment(b, set)
    }

    /// Converts a bound read expression into a place for assignment.
    fn place_for_assignment(&mut self, b: BExpr, set: bool) -> Option<(BPlace, Type)> {
        if let Some(p) = self.expr_to_place(&b) {
            return Some((p, b.ty));
        }
        let span = b.span;
        let sema = self.sema;
        match b.kind {
            BExprKind::Call(call) => {
                let BCall {
                    target,
                    receiver,
                    args,
                } = *call;
                if let CallTarget::Proc(p) = target {
                    let pi = &sema.program.procs[p.index()];
                    if pi.kind == ProcKind::PropertyGet
                        && let Some((Symbol::Property(g), _)) = sema.program.modules
                            [pi.module.index()]
                        .symbols
                        .get(&pi.name)
                    {
                        let chosen = if set { g.set } else { g.let_ };
                        return match chosen {
                            Some(c) => {
                                let cinfo = &sema.program.procs[c.index()];
                                let ty = cinfo
                                    .params
                                    .last()
                                    .map(|p| p.ty.clone())
                                    .unwrap_or(Type::Error);
                                Some((
                                    BPlace::Property {
                                        call: Box::new(BCall {
                                            target: CallTarget::Proc(c),
                                            receiver,
                                            args,
                                        }),
                                        set,
                                        ty: ty.clone(),
                                    },
                                    ty,
                                ))
                            }
                            None => {
                                let msg = if set && g.let_.is_some() {
                                    format!(
                                        "property `{}` has no `Property Set` (use a plain assignment)",
                                        pi.name
                                    )
                                } else if !set && g.set.is_some() {
                                    format!(
                                        "property `{}` has no `Property Let` (use `Set`)",
                                        pi.name
                                    )
                                } else {
                                    format!("property `{}` is read-only", pi.name)
                                };
                                self.err(codes::ASSIGNMENT_KIND, span, msg);
                                None
                            }
                        };
                    }
                }
                self.err(
                    codes::NOT_ASSIGNABLE,
                    span,
                    "cannot assign to the result of a procedure call",
                );
                None
            }
            BExprKind::Late(mut late) if late.kind == LateKind::Get => {
                let mut value_ty = Type::Variant;
                if let (Some(lt), LateMember::Named(n)) = (&late.static_receiver, &late.member)
                    && (late.dispid.is_some() || lt.library == crate::builtins::VBA_LIBRARY)
                    && let Some(m) = self.sema.provider.member(lt, &n.to_string_lossy())
                {
                    match m.kind {
                        // `Ctl = v` where `Ctl` is a read-only property that
                        // returns an object: the assignment goes to the
                        // default member of that object, which the caller
                        // (the `Let` statement) binds.
                        LibMemberKind::PropertyGet
                            if !set
                                && m.params.is_empty()
                                && matches!(&m.ty, Type::Library(ct)
                                    if self.sema.provider.default_member(ct).is_some_and(|d|
                                        d.params.iter().all(|p| p.optional || p.param_array))) =>
                        {
                            return Some((
                                BPlace::Late {
                                    late,
                                    ty: m.ty.clone(),
                                    receiver_place: None,
                                },
                                m.ty.clone(),
                            ));
                        }
                        LibMemberKind::PropertyGet => {
                            self.err(
                                codes::READ_ONLY_PROPERTY,
                                span,
                                format!("property `{}` of `{lt}` is read-only", m.name),
                            );
                            return None;
                        }
                        LibMemberKind::Method if !m.is_default => {
                            self.err(
                                codes::NOT_ASSIGNABLE,
                                span,
                                format!(
                                    "`{}` of `{lt}` is a method and cannot be assigned to",
                                    m.name
                                ),
                            );
                            return None;
                        }
                        _ => {}
                    }
                    value_ty = m.ty.clone();
                }
                late.kind = if set {
                    LateKind::PropertyPutRef
                } else {
                    LateKind::PropertyPut
                };
                let receiver_place = (late.member == LateMember::Default
                    && late.receiver.ty == Type::Variant)
                    .then(|| {
                        self.expr_to_place(&late.receiver)
                            .or_else(|| self.late_element_place(&late.receiver))
                    })
                    .flatten()
                    .map(Box::new);
                Some((
                    BPlace::Late {
                        late,
                        ty: value_ty.clone(),
                        receiver_place,
                    },
                    value_ty,
                ))
            }
            BExprKind::Const(_) => {
                self.err(codes::NOT_ASSIGNABLE, span, "cannot assign to a constant");
                None
            }
            _ => {
                self.err(
                    codes::NOT_ASSIGNABLE,
                    span,
                    "this expression cannot be assigned to",
                );
                None
            }
        }
    }

    /// A property with `Property Let`/`Set` but no `Property Get` cannot be
    /// bound as a read; bind it directly as an assignment target.
    fn try_write_only_property(&mut self, e: &Expr, set: bool) -> Option<Option<(BPlace, Type)>> {
        let (callee, args): (&Expr, &[Arg]) = match &e.kind {
            ExprKind::Call { callee, args } => (callee, args),
            _ => (e, &[]),
        };
        let sema = self.sema;
        let (group, receiver, name): (_, Option<BExpr>, &Ident) = match &callee.kind {
            ExprKind::Name(id) if !self.scope.contains_key(&id.name) => {
                match sema.lookup_module_scope(self.module, &id.name) {
                    Found::Sym(owner, Symbol::Property(g)) if g.get.is_none() => {
                        let recv = sema.program.modules[owner.index()]
                            .is_class_like()
                            .then(|| self.me_expr(callee.span));
                        (g, recv, id)
                    }
                    _ => return None,
                }
            }
            ExprKind::Member {
                target: Some(t),
                name,
            } => {
                if matches!(t.kind, ExprKind::Name(_) | ExprKind::Member { .. })
                    && self.is_namespace_like(t)
                {
                    return None;
                }
                let recv = self.bind_expr(t);
                let Type::Class(m) = recv.ty.clone() else {
                    return None;
                };
                match sema.program.modules[m.index()].symbols.get(&name.name) {
                    Some((Symbol::Property(g), _)) if g.get.is_none() => (*g, Some(recv), name),
                    _ => return None,
                }
            }
            ExprKind::Member { target: None, name } => {
                // `.Name` inside a `With` block.
                let recv = self.withs.last()?.receiver.clone();
                let Type::Class(m) = recv.ty.clone() else {
                    return None;
                };
                match sema.program.modules[m.index()].symbols.get(&name.name) {
                    Some((Symbol::Property(g), _)) if g.get.is_none() => (*g, Some(recv), name),
                    _ => return None,
                }
            }
            _ => return None,
        };
        let chosen = if set { group.set } else { group.let_ };
        let Some(c) = chosen else {
            self.err(
                codes::ASSIGNMENT_KIND,
                name.span,
                format!(
                    "property `{}` has no matching `Property Let`/`Set`",
                    name.name
                ),
            );
            return Some(None);
        };
        let info = &sema.program.procs[c.index()];
        let mut sig = Sig::of_proc(info);
        let value = sig.params.pop();
        let bargs = self.match_args(&sig, args, e.span);
        let ty = value.map(|v| v.ty).unwrap_or(Type::Error);
        Some(Some((
            BPlace::Property {
                call: Box::new(BCall {
                    target: CallTarget::Proc(c),
                    receiver,
                    args: bargs,
                }),
                set,
                ty: ty.clone(),
            },
            ty,
        )))
    }

    fn is_namespace_like(&self, t: &Expr) -> bool {
        let sema = self.sema;
        match &t.kind {
            ExprKind::Name(id) => {
                !self.scope.contains_key(&id.name)
                    && matches!(
                        sema.lookup_module_scope(self.module, &id.name),
                        Found::Module(_) | Found::Sym(_, Symbol::Enum(_))
                    )
            }
            ExprKind::Member {
                target: Some(inner),
                ..
            } => self.is_namespace_like(inner),
            _ => false,
        }
    }

    #[allow(dead_code)]
    fn unused(&self, _: Span) {}
}
