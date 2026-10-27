//! Call signatures, argument matching (MS-VBAL §5.6) and implicit
//! coercions (§5.6, `Let`/`Set` rules).

use vba_ast::{Arg, Expr, ExprKind, Ident};
use vba_source::Span;
use vba_stdlib::FunctionInfo;
use vba_value::VarType;

use crate::bound::{BArg, BExpr, BExprKind, BLate, CallTarget, Constant, LateKind, LateMember};
use crate::codes;
use crate::program::{DeclareInfo, EventInfo, LocalInfo, LocalKind, ProcInfo};
use crate::provider::LibMember;
use crate::types::*;

use super::Binder;

/// A formal parameter as seen by a call site.
#[derive(Debug, Clone)]
pub(crate) struct SigParam {
    pub name: String,
    pub ty: Type,
    pub by_val: bool,
    pub optional: bool,
    pub param_array: bool,
    /// A `ByVal String` parameter of a `Declare`: the callee fills the
    /// buffer, so a variable argument is passed by reference and receives
    /// the converted-back buffer.
    pub buffer: bool,
    /// The default a library declares for an omitted optional argument.
    pub default: Option<vba_value::Variant>,
}

/// The callable part of a procedure-like thing.
#[derive(Debug, Clone)]
pub(crate) struct Sig {
    pub what: String,
    pub params: Vec<SigParam>,
    /// `None` for a `Sub`.
    pub ret: Option<Type>,
}

pub(crate) fn var_type_to_type(v: VarType) -> Type {
    match v {
        VarType::Integer => Type::Integer,
        VarType::Long => Type::Long,
        VarType::Single => Type::Single,
        VarType::Double => Type::Double,
        VarType::Currency => Type::Currency,
        VarType::Date => Type::Date,
        VarType::String => Type::String,
        VarType::Object | VarType::DataObject => Type::Object,
        VarType::Boolean => Type::Boolean,
        VarType::Decimal => Type::Decimal,
        VarType::Byte => Type::Byte,
        VarType::LongLong => Type::LongLong,
        _ => Type::Variant,
    }
}

/// Marks a late-bound access as early-bound by member id.
pub(crate) fn with_dispid(mut e: BExpr, id: Option<i32>) -> BExpr {
    if let BExprKind::Late(l) = &mut e.kind {
        l.dispid = id;
    }
    e
}

impl Sig {
    pub fn of_proc(p: &ProcInfo) -> Sig {
        Sig {
            what: p.describe(),
            params: p
                .params
                .iter()
                .map(|q| SigParam {
                    name: q.name.to_string(),
                    ty: q.ty.clone(),
                    by_val: q.by_val,
                    optional: q.optional,
                    param_array: q.param_array,
                    buffer: false,
                    default: None,
                })
                .collect(),
            ret: p.has_result.then(|| p.ret.clone()),
        }
    }

    pub fn of_declare(d: &DeclareInfo) -> Sig {
        Sig {
            what: format!("Declare {}", d.name),
            params: d
                .params
                .iter()
                .map(|q| SigParam {
                    name: q.name.to_string(),
                    ty: q.ty.clone(),
                    by_val: q.by_val,
                    optional: q.optional,
                    param_array: q.param_array,
                    buffer: q.by_val && q.ty.is_string(),
                    default: None,
                })
                .collect(),
            ret: d.is_function.then(|| d.ret.clone()),
        }
    }

    pub fn of_event(e: &EventInfo) -> Sig {
        Sig {
            what: format!("Event {}", e.name),
            params: e
                .params
                .iter()
                .map(|q| SigParam {
                    name: q.name.to_string(),
                    ty: q.ty.clone(),
                    by_val: q.by_val,
                    optional: q.optional,
                    param_array: q.param_array,
                    buffer: false,
                    default: None,
                })
                .collect(),
            ret: None,
        }
    }

    pub fn of_lib(m: &LibMember) -> Sig {
        Sig {
            what: m.name.clone(),
            params: m
                .params
                .iter()
                .map(|q| SigParam {
                    name: q.name.clone(),
                    ty: q.ty.clone(),
                    by_val: !q.by_ref,
                    optional: q.optional,
                    param_array: q.param_array,
                    buffer: false,
                    default: q.default.clone(),
                })
                .collect(),
            ret: (!m.is_sub).then(|| m.ty.clone()),
        }
    }

    pub fn of_intrinsic(f: &FunctionInfo, dollar: bool) -> Sig {
        Sig {
            what: f.name.to_owned(),
            params: f
                .params
                .iter()
                .map(|q| SigParam {
                    name: q.name.to_owned(),
                    ty: var_type_to_type(q.ty),
                    by_val: !q.by_ref,
                    optional: q.optional,
                    param_array: q.param_array,
                    buffer: false,
                    default: None,
                })
                .collect(),
            ret: Some(if dollar && f.dollar {
                Type::String
            } else {
                var_type_to_type(f.returns)
            }),
        }
    }
}

/// Why a value is being converted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ctx {
    /// `Let` assignment or operand: objects yield their default value.
    Let,
    /// Argument passing: objects stay objects when the target is a
    /// `Variant`.
    Arg,
}

impl Binder<'_, '_> {
    pub(crate) fn mk_convert(&self, e: BExpr, to: Type) -> BExpr {
        let span = e.span;
        BExpr::new(span, to, BExprKind::Convert(Box::new(e)))
    }

    /// Fetches the default value of an object-typed expression (the `Let`
    /// coercion). Non-object expressions are returned unchanged.
    pub(crate) fn rvalue(&mut self, e: BExpr) -> BExpr {
        let mut e = e;
        for _ in 0..4 {
            if !e.ty.is_object() {
                return e;
            }
            e = match e.ty.clone() {
                Type::Nothing => {
                    self.err(
                        codes::TYPE_MISMATCH,
                        e.span,
                        "`Nothing` has no default value",
                    );
                    return self.error_expr(e.span);
                }
                Type::Class(m) => {
                    let mi = &self.sema.program.modules[m.index()];
                    match mi.default_member.clone() {
                        Some(name) => {
                            let ident = Ident {
                                name,
                                suffix: None,
                                kind: vba_ast::IdentKind::Plain,
                                span: e.span,
                            };
                            self.member_of(e, &ident, Some(&[]), false)
                        }
                        None => {
                            self.err(
                                codes::TYPE_MISMATCH,
                                e.span,
                                format!(
                                    "class `{}` has no default member (use `Set` to assign an object reference)",
                                    mi.name
                                ),
                            );
                            return self.error_expr(e.span);
                        }
                    }
                }
                Type::Library(lt) => match self.sema.provider.default_member(&lt) {
                    Some(m) => {
                        let ty = m.ty.clone();
                        let l = self.late(
                            e,
                            LateMember::Named(vba_ast::Name::new(&m.name)),
                            Vec::new(),
                            ty,
                            Some(lt),
                        );
                        with_dispid(l, m.dispid)
                    }
                    None => {
                        self.info_late(e.span, &lt);
                        self.late(e, LateMember::Default, Vec::new(), Type::Variant, Some(lt))
                    }
                },
                _ => self.late(e, LateMember::Default, Vec::new(), Type::Variant, None),
            };
        }
        e
    }

    pub(crate) fn late(
        &self,
        receiver: BExpr,
        member: LateMember,
        args: Vec<crate::bound::BLateArg>,
        ty: Type,
        static_receiver: Option<LibraryType>,
    ) -> BExpr {
        let span = receiver.span;
        BExpr::new(
            span,
            ty,
            BExprKind::Late(Box::new(BLate {
                receiver,
                member,
                kind: LateKind::Get,
                args,
                static_receiver,
                dispid: None,
            })),
        )
    }

    pub(crate) fn info_late(&self, span: Span, lt: &LibraryType) {
        self.sema.report(
            vba_diagnostics::Severity::Info,
            self.module,
            codes::LATE_BOUND_LIBRARY_MEMBER,
            span,
            format!(
                "library type `{lt}` is not described by any provider; member access is bound late"
            ),
        );
    }

    /// Whether `from` can be stored into a location of object type `to`.
    fn object_coercion(&self, from: &Type, to: &Type) -> Coercion {
        if from == to {
            return Coercion::Identity;
        }
        match (from, to) {
            (Type::Nothing, _) => Coercion::Identity,
            (_, Type::Object) => Coercion::Identity,
            (Type::Class(a), Type::Class(_) | Type::Library(_)) => {
                let ai = &self.sema.program.modules[a.index()];
                if ai.implements.iter().any(|i| i.interface == *to) {
                    Coercion::Identity
                } else if let Type::Class(b) = to
                    && self.sema.program.modules[b.index()]
                        .implements
                        .iter()
                        .any(|i| i.interface == Type::Class(*a))
                {
                    Coercion::RuntimeCast
                } else {
                    Coercion::Invalid
                }
            }
            (Type::Library(_), Type::Library(_)) => Coercion::RuntimeCast,
            (Type::Library(_), Type::Class(_)) => Coercion::Invalid,
            (Type::Object, Type::Class(_) | Type::Library(_)) => Coercion::RuntimeCast,
            _ => Coercion::Invalid,
        }
    }

    /// Converts `e` to type `to` as an assignment or argument would,
    /// inserting explicit conversion nodes and reporting impossible ones.
    pub(crate) fn coerce(&mut self, e: BExpr, to: &Type, ctx: Ctx) -> BExpr {
        if e.ty.is_error() || to.is_error() || *to == Type::Any {
            return e;
        }
        if e.ty == *to {
            return e;
        }
        let span = e.span;
        let from = e.ty.clone();
        // Constants are converted at analysis time when that is exact.
        if let BExprKind::Const(Constant::Value(v)) = &e.kind
            && !from.is_object()
            && !matches!(from, Type::Udt(_) | Type::LibRecord(_) | Type::Array { .. })
        {
            match to {
                Type::Variant => {
                    return BExpr::new(span, Type::Variant, e.kind);
                }
                t if !t.is_object()
                    && !matches!(
                        t,
                        Type::FixedString(_)
                            | Type::Udt(_)
                            | Type::LibRecord(_)
                            | Type::Array { .. }
                    ) =>
                {
                    if let Some(vt) = t.var_type()
                        && let Ok(c) = vba_value::convert::convert_to(v, vt)
                    {
                        return BExpr::new(span, to.clone(), BExprKind::Const(Constant::Value(c)));
                    }
                }
                _ => {}
            }
        }
        let mismatch = |this: &Self| {
            this.err(
                codes::TYPE_MISMATCH,
                span,
                format!("cannot convert {} to {}", this.tn(&from), this.tn(to)),
            );
        };
        match to {
            Type::Variant => {
                if from.is_object() {
                    if ctx == Ctx::Arg {
                        return e;
                    }
                    return self.rvalue(e);
                }
                match from {
                    Type::Udt(_) | Type::LibRecord(_) if ctx == Ctx::Arg && self.udt_arg_ok => {
                        self.mk_convert(e, Type::Variant)
                    }
                    Type::Udt(_) | Type::LibRecord(_) => {
                        mismatch(self);
                        self.error_expr(span)
                    }
                    _ => self.mk_convert(e, Type::Variant),
                }
            }
            t if t.is_object() => {
                if from.is_variant() {
                    return self.mk_convert(e, to.clone());
                }
                // MS-VBAL 5.3.1.11: a scalar cannot be passed to an
                // object-typed parameter.
                if ctx == Ctx::Arg && (from.is_numeric() || from.is_string()) {
                    mismatch(self);
                    return self.error_expr(span);
                }
                match self.object_coercion(&from, t) {
                    Coercion::Identity => e,
                    Coercion::RuntimeCast => self.mk_convert(e, to.clone()),
                    _ => {
                        mismatch(self);
                        self.error_expr(span)
                    }
                }
            }
            Type::Udt(_) | Type::LibRecord(_) => {
                mismatch(self);
                self.error_expr(span)
            }
            Type::Array { elem: te, rank: tr } => match &from {
                Type::Array { elem: fe, rank: fr } => {
                    if fe == te && (tr.is_none() || fr.is_none() || tr == fr) {
                        e
                    } else {
                        mismatch(self);
                        self.error_expr(span)
                    }
                }
                Type::Variant => self.mk_convert(e, to.clone()),
                // `b = s`: a String converts to a `Byte` array.
                Type::String | Type::FixedString(_) if **te == Type::Byte => {
                    self.mk_convert(e, to.clone())
                }
                _ => {
                    mismatch(self);
                    self.error_expr(span)
                }
            },
            _scalar => {
                if from.is_object() {
                    let v = self.rvalue(e);
                    if v.ty.is_error() {
                        return v;
                    }
                    return self.coerce(v, to, ctx);
                }
                match from {
                    // `s = b`: a `Byte` array converts to a String.
                    Type::Array { ref elem, .. } if **elem == Type::Byte && to.is_string() => {
                        self.mk_convert(e, to.clone())
                    }
                    Type::Udt(_) | Type::LibRecord(_) | Type::Array { .. } => {
                        mismatch(self);
                        self.error_expr(span)
                    }
                    _ => {
                        // Enum <-> Long are the same representation.
                        if matches!(
                            (&from, to),
                            (Type::Enum(_) | Type::LibEnum(_), Type::Long)
                                | (Type::Long, Type::Enum(_) | Type::LibEnum(_))
                        ) {
                            let mut e = e;
                            e.ty = to.clone();
                            return e;
                        }
                        self.mk_convert(e, to.clone())
                    }
                }
            }
        }
    }

    /// Converts to `Boolean` for conditions; `Variant` is left to the
    /// run-time truthiness rule (Null is false).
    pub(crate) fn coerce_condition(&mut self, e: BExpr) -> BExpr {
        let e = self.rvalue(e);
        match &e.ty {
            Type::Boolean | Type::Variant | Type::Error => e,
            Type::Udt(_) | Type::LibRecord(_) | Type::Array { .. } | Type::Nothing => {
                self.err(
                    codes::TYPE_MISMATCH,
                    e.span,
                    "a condition must be a Boolean or numeric value",
                );
                self.error_expr(e.span)
            }
            _ => self.mk_convert(e, Type::Boolean),
        }
    }

    /// Converts to a `Long` (subscripts, file numbers, `On..GoTo` selectors).
    pub(crate) fn coerce_index(&mut self, e: BExpr, to: Type) -> BExpr {
        let e = self.rvalue(e);
        if e.ty == to || e.ty.is_error() {
            return e;
        }
        match &e.ty {
            Type::Udt(_) | Type::LibRecord(_) | Type::Array { .. } | Type::Nothing => {
                self.err(
                    codes::TYPE_MISMATCH,
                    e.span,
                    format!("expected {}", self.tn(&to)),
                );
                self.error_expr(e.span)
            }
            Type::Byte | Type::Integer | Type::Long | Type::Enum(_) | Type::LibEnum(_)
                if to == Type::Long =>
            {
                // Integral subscripts need no conversion.
                e
            }
            _ => self.mk_convert(e, to),
        }
    }

    // ---- arguments ---------------------------------------------------------

    /// Matches an argument list against a signature (§5.6): positional,
    /// then named arguments, `Optional`, `ParamArray`.
    pub(crate) fn match_args(&mut self, sig: &Sig, args: &[Arg], call_span: Span) -> Vec<BArg> {
        let pa = sig.params.iter().position(|p| p.param_array);
        let fixed = pa.unwrap_or(sig.params.len());
        let mut slots: Vec<Option<BArg>> = vec![None; fixed];
        let mut extra: Vec<BArg> = Vec::new();
        let mut named_seen = false;
        let mut pos = 0usize;
        let mut ok = true;
        for a in args {
            if let Some(n) = &a.name {
                named_seen = true;
                let Some(idx) = sig
                    .params
                    .iter()
                    .take(fixed)
                    .position(|p| p.name.eq_ignore_ascii_case(&n.name.to_string()))
                else {
                    self.err(
                        codes::BAD_ARGUMENTS,
                        n.span,
                        format!("{} has no parameter named `{}`", sig.what, n.name),
                    );
                    ok = false;
                    if let Some(v) = &a.value {
                        let _ = self.bind_expr(v);
                    }
                    continue;
                };
                if slots[idx].is_some() {
                    self.err(
                        codes::BAD_ARGUMENTS,
                        n.span,
                        format!("argument `{}` is given more than once", n.name),
                    );
                    ok = false;
                    continue;
                }
                match &a.value {
                    Some(v) => slots[idx] = Some(self.bind_arg(v, &sig.params[idx])),
                    None => {
                        self.err(
                            codes::BAD_ARGUMENTS,
                            a.span,
                            "a named argument needs a value",
                        );
                        ok = false;
                    }
                }
                continue;
            }
            if named_seen {
                self.err(
                    codes::BAD_ARGUMENTS,
                    a.span,
                    "positional arguments cannot follow named arguments",
                );
                ok = false;
                continue;
            }
            let idx = pos;
            pos += 1;
            if idx < fixed {
                match &a.value {
                    Some(v) => slots[idx] = Some(self.bind_arg(v, &sig.params[idx])),
                    None => {
                        if sig.params[idx].optional {
                            slots[idx] = Some(self.omitted_arg(&sig.params[idx], a.span));
                        } else {
                            self.err(
                                codes::BAD_ARGUMENTS,
                                a.span,
                                format!("argument `{}` is not optional", sig.params[idx].name),
                            );
                            ok = false;
                        }
                    }
                }
            } else if let Some(p) = pa {
                match &a.value {
                    Some(v) => {
                        let be = self.bind_expr(v);
                        let be = self.coerce(be, &Type::Variant, Ctx::Arg);
                        extra.push(BArg::Value(be));
                    }
                    None => extra.push(BArg::Missing),
                }
                let _ = p;
            } else {
                self.err(
                    codes::BAD_ARGUMENTS,
                    a.span,
                    format!("too many arguments for {}", sig.what),
                );
                ok = false;
                if let Some(v) = &a.value {
                    let _ = self.bind_expr(v);
                }
            }
        }
        let mut out = Vec::with_capacity(fixed + extra.len());
        for (i, s) in slots.into_iter().enumerate() {
            match s {
                Some(a) => out.push(a),
                None => {
                    if sig.params[i].optional {
                        let a = self.omitted_arg(&sig.params[i], call_span);
                        out.push(a);
                    } else {
                        if ok {
                            self.err(
                                codes::BAD_ARGUMENTS,
                                call_span,
                                format!(
                                    "missing argument `{}` in call to {}",
                                    sig.params[i].name, sig.what
                                ),
                            );
                        }
                        out.push(BArg::Missing);
                    }
                }
            }
        }
        out.extend(extra);
        out
    }

    /// An omitted `Optional` argument: missing, or the default the library
    /// declares for it.
    fn omitted_arg(&mut self, p: &SigParam, span: Span) -> BArg {
        match &p.default {
            Some(v) if !matches!(v, vba_value::Variant::Empty) => {
                let ty = var_type_to_type(v.vartype());
                let c = BExpr::new(span, ty, BExprKind::Const(Constant::Value(v.clone())));
                BArg::Value(self.coerce(c, &p.ty, Ctx::Arg))
            }
            _ => BArg::Missing,
        }
    }

    /// Binds one argument against its parameter (`ByRef`, `ByVal`, copies).
    pub(crate) fn bind_arg(&mut self, e: &Expr, p: &SigParam) -> BArg {
        let forced_value = matches!(e.kind, ExprKind::Paren(_));
        let be = self.bind_expr(e);
        if be.ty.is_error() {
            return BArg::Value(be);
        }
        if p.buffer
            && !forced_value
            && be.ty.is_string()
            && let Some(place) = self.expr_to_place(&be)
        {
            return BArg::Ref(place);
        }
        if p.by_val || forced_value {
            let be = if forced_value {
                self.rvalue_if_scalar_param(be, &p.ty)
            } else {
                be
            };
            return BArg::Value(self.coerce(be, &p.ty, Ctx::Arg));
        }
        // ByRef.
        if let Some(place) = self.expr_to_place(&be) {
            let pty = &p.ty;
            let same = be.ty == *pty
                || matches!(
                    (&be.ty, pty),
                    (Type::Enum(_) | Type::LibEnum(_), Type::Long)
                        | (Type::Long, Type::Enum(_) | Type::LibEnum(_))
                )
                || *pty == Type::Any
                || match (&be.ty, pty) {
                    (Type::Array { elem: a, rank: ra }, Type::Array { elem: b, rank: rb }) => {
                        a == b && (ra == rb || rb.is_none() || ra.is_none())
                    }
                    (Type::FixedString(_), Type::String) => true,
                    _ => false,
                };
            if same {
                return BArg::Ref(place);
            }
            if *pty == Type::Variant {
                // A Variant parameter receives a converted copy; the variable
                // is assigned from it when the call returns.
                let value = self.coerce(be.clone(), pty, Ctx::Arg);
                if value.ty.is_error() || be.ty.is_error() {
                    return BArg::Value(value);
                }
                let temp = self.new_local(LocalInfo {
                    name: vba_ast::Name::new(""),
                    ty: Type::Variant,
                    kind: LocalKind::Temp,
                    fixed_bounds: None,
                    auto_new: false,
                    span: be.span,
                });
                let read = BExpr::new(be.span, Type::Variant, BExprKind::Local(temp));
                let back = self.coerce(read, &be.ty, Ctx::Let);
                return BArg::RefCopy(Box::new(crate::bound::BRefCopy {
                    place,
                    temp,
                    value,
                    back,
                }));
            }
            self.err(
                codes::BYREF_MISMATCH,
                be.span,
                format!(
                    "ByRef argument type mismatch: parameter `{}` is {} but the argument is {}",
                    p.name,
                    self.tn(pty),
                    self.tn(&be.ty)
                ),
            );
            return BArg::Ref(place);
        }
        BArg::Value(self.coerce(be, &p.ty, Ctx::Arg))
    }

    /// `(obj)` evaluates the default member; this is applied by
    /// [`Binder::coerce`] for scalar parameters, and here for the
    /// parenthesised `Variant` case.
    fn rvalue_if_scalar_param(&mut self, be: BExpr, _pty: &Type) -> BExpr {
        if be.ty.is_object() {
            self.rvalue(be)
        } else {
            be
        }
    }

    /// Arguments of a late-bound call: names are kept, variables are passed
    /// by reference, objects stay objects.
    pub(crate) fn late_args(&mut self, args: &[Arg]) -> Vec<crate::bound::BLateArg> {
        let mut out = Vec::new();
        for a in args {
            let name = a.name.as_ref().map(|n| n.name.clone());
            let arg = match &a.value {
                None => BArg::Missing,
                Some(v) => {
                    let forced = matches!(v.kind, ExprKind::Paren(_));
                    let be = self.bind_expr(v);
                    if !forced
                        && !be.ty.is_error()
                        && let Some(p) = self.expr_to_place(&be)
                    {
                        BArg::Ref(p)
                    } else {
                        BArg::Value(be)
                    }
                }
            };
            out.push(crate::bound::BLateArg { name, arg });
        }
        out
    }

    pub(crate) fn call_expr(
        &self,
        span: Span,
        ty: Type,
        target: CallTarget,
        receiver: Option<BExpr>,
        args: Vec<BArg>,
    ) -> BExpr {
        BExpr::new(
            span,
            ty,
            BExprKind::Call(Box::new(crate::bound::BCall {
                target,
                receiver,
                args,
            })),
        )
    }
}
