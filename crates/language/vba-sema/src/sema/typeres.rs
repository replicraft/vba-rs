//! Type resolution and signature building (pass 2).

use vba_ast::{
    AsClause, Bound, BuiltinType, Ident, Param, PassBy, ProcKind, Stmt, StmtKind, TypeRef,
    TypeRefKind,
};
use vba_lexer::TypeSuffix;
use vba_profile::Feature;
use vba_source::Span;

use crate::codes;
use crate::fold::{ConstScope, constant_to_i32, convert_constant, fold, long_ptr};
use crate::program::*;
use crate::types::*;
use crate::walk::walk_block;

use super::scope::TypeFound;
use super::{ModuleScope, Sema};

/// The resolved type of a declaration.
pub(crate) struct DeclType {
    pub ty: Type,
    pub auto_new: bool,
    pub fixed_bounds: Option<Vec<(i32, i32)>>,
}

/// Where a parameter list appears.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamCtx {
    Proc,
    Declare,
    Event,
}

impl Sema<'_> {
    pub(crate) fn suffix_type(&self, s: TypeSuffix) -> Type {
        match s {
            TypeSuffix::Integer => Type::Integer,
            TypeSuffix::Long => Type::Long,
            TypeSuffix::LongLong => Type::LongLong,
            TypeSuffix::Single => Type::Single,
            TypeSuffix::Double => Type::Double,
            TypeSuffix::Currency => Type::Currency,
            TypeSuffix::String => Type::String,
        }
    }

    pub(crate) fn resolve_as_clause(
        &self,
        module: ModuleId,
        a: &AsClause,
        allow_any: bool,
    ) -> Type {
        let base = self.resolve_type_ref(module, &a.ty, allow_any);
        if let Some(len) = &a.fixed_length {
            if base != Type::String {
                if !base.is_error() {
                    self.error(
                        module,
                        codes::TYPE_DECLARATION,
                        len.span,
                        "a length is only valid for `String`",
                    );
                }
                return Type::Error;
            }
            let scope = ModuleScope { sema: self, module };
            return match fold(&scope, len) {
                Ok((c, _)) => match constant_to_i32(&c) {
                    Ok(n) if (1..=65526).contains(&n) => Type::FixedString(n as u32),
                    _ => {
                        self.error(
                            module,
                            codes::TYPE_DECLARATION,
                            len.span,
                            "fixed string length must be between 1 and 65526",
                        );
                        Type::Error
                    }
                },
                Err(e) => {
                    self.error(module, codes::CONSTANT_EXPRESSION, e.span, e.message);
                    Type::Error
                }
            };
        }
        base
    }

    pub(crate) fn resolve_type_ref(&self, module: ModuleId, t: &TypeRef, allow_any: bool) -> Type {
        match &t.kind {
            TypeRefKind::Missing => Type::Error,
            TypeRefKind::Builtin(b) => match b {
                BuiltinType::Boolean => Type::Boolean,
                BuiltinType::Byte => Type::Byte,
                BuiltinType::Currency => Type::Currency,
                BuiltinType::Date => Type::Date,
                BuiltinType::Double => Type::Double,
                BuiltinType::Integer => Type::Integer,
                BuiltinType::Long => Type::Long,
                BuiltinType::LongLong => {
                    if self.profile().supports(Feature::LongLong) {
                        Type::LongLong
                    } else {
                        self.error(
                            module,
                            codes::TYPE_DECLARATION,
                            t.span,
                            "`LongLong` needs a 64-bit VBA7 profile",
                        );
                        Type::Error
                    }
                }
                BuiltinType::LongPtr => {
                    if self.profile().supports(Feature::LongPtr) {
                        long_ptr(self.profile())
                    } else {
                        self.error(
                            module,
                            codes::TYPE_DECLARATION,
                            t.span,
                            "`LongPtr` is not available in this language profile",
                        );
                        Type::Error
                    }
                }
                BuiltinType::Single => Type::Single,
                BuiltinType::String => Type::String,
                BuiltinType::Variant => Type::Variant,
                BuiltinType::Any => {
                    if allow_any {
                        Type::Any
                    } else {
                        self.error(
                            module,
                            codes::TYPE_DECLARATION,
                            t.span,
                            "`As Any` is only valid for `Declare` parameters",
                        );
                        Type::Error
                    }
                }
            },
            TypeRefKind::Named(idents) => {
                // MS-VBAL 3.3.5.3 BUILTIN-TYPE: `[Integer]` names the built-in
                // type of the same name.
                if let [one] = idents.as_slice() {
                    use BuiltinType as B;
                    const NAMES: [(&str, BuiltinType); 12] = [
                        ("Boolean", B::Boolean),
                        ("Byte", B::Byte),
                        ("Currency", B::Currency),
                        ("Date", B::Date),
                        ("Double", B::Double),
                        ("Integer", B::Integer),
                        ("Long", B::Long),
                        ("LongLong", B::LongLong),
                        ("LongPtr", B::LongPtr),
                        ("Single", B::Single),
                        ("String", B::String),
                        ("Variant", B::Variant),
                    ];
                    if let Some((_, b)) = NAMES.iter().find(|(n, _)| one.name.eq_ignore_case(n)) {
                        let synthetic = TypeRef {
                            span: t.span,
                            kind: TypeRefKind::Builtin(*b),
                        };
                        return self.resolve_type_ref(module, &synthetic, allow_any);
                    }
                }
                self.resolve_named_type(module, idents, t.span)
            }
        }
    }

    fn class_in_module(&self, owner: ModuleId, name: &Ident) -> Option<Type> {
        let m = &self.program.modules[owner.index()];
        match m.symbols.get(&name.name) {
            Some((crate::program::Symbol::Enum(e), _))
                if self.symbol_public(crate::program::Symbol::Enum(*e)) =>
            {
                Some(Type::Enum(*e))
            }
            Some((crate::program::Symbol::Udt(u), _))
                if self.symbol_public(crate::program::Symbol::Udt(*u)) =>
            {
                Some(Type::Udt(*u))
            }
            _ => None,
        }
    }

    fn resolve_named_type(&self, module: ModuleId, idents: &[Ident], span: Span) -> Type {
        if idents.iter().any(Ident::is_missing) {
            return Type::Error;
        }
        let text = || {
            idents
                .iter()
                .map(|i| i.name.to_string())
                .collect::<Vec<_>>()
                .join(".")
        };
        match idents {
            [one] => {
                if one.name.eq_ignore_case("Object") {
                    return Type::Object;
                }
                match self.lookup_type_name(module, &one.name) {
                    TypeFound::Enum(e) => return Type::Enum(e),
                    TypeFound::Udt(u) => return Type::Udt(u),
                    TypeFound::Class(m) => return Type::Class(m),
                    TypeFound::Ambiguous(ms) => {
                        self.ambiguous(module, span, &one.name.to_string(), &ms);
                        return Type::Error;
                    }
                    TypeFound::None => {}
                }
                if let Some(t) = self
                    .provider
                    .resolve_declared(None, &one.name.to_string_lossy())
                {
                    self.note_type(&t);
                    return t;
                }
            }
            [q, n] => {
                if let Some(t) = self.qualified_project_type(q, n) {
                    return t;
                }
                if let Some(t) = self
                    .provider
                    .resolve_declared(Some(&q.name.to_string_lossy()), &n.name.to_string_lossy())
                {
                    self.note_type(&t);
                    return t;
                }
            }
            [p, q, n] if p.name == self.project_name => {
                if let Some(t) = self.qualified_project_type(q, n) {
                    return t;
                }
            }
            _ => {}
        }
        self.error(
            module,
            codes::UNRESOLVED_LIBRARY_TYPE,
            span,
            format!(
                "unresolved library type `{}`: it is not a type of this project and no library provides it",
                text()
            ),
        );
        Type::Error
    }

    /// `Module.Type` or `Project.Class`.
    fn qualified_project_type(&self, q: &Ident, n: &Ident) -> Option<Type> {
        if let Some(&m) = self.module_ids.get(&q.name)
            && let Some(t) = self.class_in_module(m, n)
        {
            return Some(t);
        }
        if q.name == self.project_name
            && let Some(&m) = self.module_ids.get(&n.name)
            && self.program.modules[m.index()].is_class_like()
        {
            return Some(Type::Class(m));
        }
        if q.name == self.project_name {
            // `Project.Enum` and `Project.Type`: public in any module.
            return match self.project_symbol(&n.name) {
                Some((_, crate::program::Symbol::Enum(e))) => Some(Type::Enum(e)),
                Some((_, crate::program::Symbol::Udt(u))) => Some(Type::Udt(u)),
                _ => None,
            };
        }
        None
    }

    pub(crate) fn ambiguous(&self, module: ModuleId, span: Span, name: &str, ms: &[ModuleId]) {
        let names: Vec<String> = ms
            .iter()
            .map(|m| self.program.modules[m.index()].name.to_string())
            .collect();
        self.error(
            module,
            codes::AMBIGUOUS_NAME,
            span,
            format!(
                "ambiguous name `{name}`: defined in modules {}",
                names.join(", ")
            ),
        );
    }

    /// Resolves the type of a variable-like declaration.
    pub(crate) fn decl_type(
        &self,
        module: ModuleId,
        name: &Ident,
        as_clause: Option<&AsClause>,
        bounds: Option<&[Bound]>,
        scope: &dyn ConstScope,
    ) -> DeclType {
        let opts = &self.program.modules[module.index()].options;
        let mut auto_new = false;
        let mut base = match as_clause {
            Some(a) => {
                let t = self.resolve_as_clause(module, a, false);
                if let Some(s) = name.suffix {
                    let st = self.suffix_type(s);
                    if !t.is_error() && st != t {
                        self.error(
                            module,
                            codes::TYPE_DECLARATION,
                            name.span,
                            "type-declaration character does not match the declared type",
                        );
                    }
                }
                if a.is_new {
                    if matches!(t, Type::Class(_) | Type::Library(_)) {
                        self.check_creatable(module, a.span, &t);
                        auto_new = true;
                    } else if !t.is_error() {
                        self.error(
                            module,
                            codes::TYPE_DECLARATION,
                            a.span,
                            "`New` requires a class type",
                        );
                    }
                }
                t
            }
            None => match name.suffix {
                Some(s) => self.suffix_type(s),
                None => opts.default_type_for(&name.name),
            },
        };
        let mut fixed_bounds = None;
        if let Some(bs) = bounds {
            if bs.is_empty() {
                base = Type::array_of(base, None);
            } else {
                let (fb, rank) = self.eval_bounds(module, bs, scope);
                fixed_bounds = fb;
                base = Type::array_of(base, Some(rank));
            }
            // `Dim a(n) As New C`: each element is an automatic
            // instantiation variable (MS-VBAL 5.2.3.1.1); the flag stays on the
            // array variable.
        }
        DeclType {
            ty: base,
            auto_new,
            fixed_bounds,
        }
    }

    /// Evaluates declared array bounds.
    pub(crate) fn eval_bounds(
        &self,
        module: ModuleId,
        bs: &[Bound],
        scope: &dyn ConstScope,
    ) -> (Option<Vec<(i32, i32)>>, u8) {
        let base = i32::from(self.program.modules[module.index()].options.base);
        let rank = bs.len().min(60) as u8;
        if bs.len() > 60 {
            self.error(
                module,
                codes::ARRAY_RULE,
                bs[60].span,
                "an array can have at most 60 dimensions",
            );
        }
        let mut out = Vec::new();
        let mut ok = true;
        for b in bs {
            let lower = match &b.lower {
                Some(e) => self.fold_i32(module, scope, e),
                None => Some(base),
            };
            let upper = self.fold_i32(module, scope, &b.upper);
            match (lower, upper) {
                (Some(l), Some(u)) => {
                    if u < l {
                        self.error(
                            module,
                            codes::ARRAY_RULE,
                            b.span,
                            "the upper bound is less than the lower bound",
                        );
                        ok = false;
                    }
                    out.push((l, u));
                }
                _ => ok = false,
            }
        }
        (ok.then_some(out), rank)
    }

    fn fold_i32(&self, module: ModuleId, scope: &dyn ConstScope, e: &vba_ast::Expr) -> Option<i32> {
        if matches!(e.kind, vba_ast::ExprKind::Error) {
            return None;
        }
        match fold(scope, e) {
            Ok((c, _)) => match constant_to_i32(&c) {
                Ok(v) => Some(v),
                Err(m) => {
                    self.error(module, codes::CONSTANT_EXPRESSION, e.span, m);
                    None
                }
            },
            Err(er) => {
                self.error(module, codes::CONSTANT_EXPRESSION, er.span, er.message);
                None
            }
        }
    }

    /// Builds parameter infos and checks the parameter-list rules (§5.3).
    pub(crate) fn build_params(
        &self,
        module: ModuleId,
        params: &[Param],
        ctx: ParamCtx,
    ) -> Vec<ParamInfo> {
        let opts = &self.program.modules[module.index()].options;
        let scope = ModuleScope { sema: self, module };
        let mut out: Vec<ParamInfo> = Vec::new();
        let mut seen_optional = false;
        for (i, p) in params.iter().enumerate() {
            if out.iter().any(|q| q.name == p.name.name) && !p.name.is_missing() {
                self.error(
                    module,
                    codes::DUPLICATE_DECLARATION,
                    p.name.span,
                    format!("duplicate parameter `{}`", p.name.name),
                );
            }
            let elem = match &p.as_clause {
                Some(a) => {
                    if a.fixed_length.is_some() {
                        self.error(
                            module,
                            codes::PARAMETER_RULE,
                            a.span,
                            "a parameter cannot be a fixed-length string",
                        );
                    }
                    self.resolve_type_ref(module, &a.ty, ctx == ParamCtx::Declare)
                }
                None => match p.name.suffix {
                    Some(s) => self.suffix_type(s),
                    None => opts.default_type_for(&p.name.name),
                },
            };
            if p.param_array {
                if i + 1 != params.len() {
                    self.error(
                        module,
                        codes::PARAMETER_RULE,
                        p.span,
                        "`ParamArray` must be the last parameter",
                    );
                }
                if p.optional || p.pass_by == Some(PassBy::ByVal) {
                    self.error(
                        module,
                        codes::PARAMETER_RULE,
                        p.span,
                        "`ParamArray` cannot be `Optional` or `ByVal`",
                    );
                }
                if !elem.is_variant() && !elem.is_error() {
                    self.error(
                        module,
                        codes::PARAMETER_RULE,
                        p.span,
                        "a `ParamArray` must be an array of `Variant`",
                    );
                }
                if ctx != ParamCtx::Proc {
                    self.error(
                        module,
                        codes::PARAMETER_RULE,
                        p.span,
                        "`ParamArray` is only valid in procedures",
                    );
                }
            }
            if seen_optional && !p.optional && !p.param_array {
                self.error(
                    module,
                    codes::PARAMETER_RULE,
                    p.span,
                    "a required parameter cannot follow an `Optional` parameter",
                );
            }
            seen_optional |= p.optional;
            let is_array = p.is_array || p.param_array;
            let ty = if is_array {
                Type::array_of(elem.clone(), None)
            } else {
                elem.clone()
            };
            if p.optional && (matches!(elem, Type::Udt(_) | Type::LibRecord(_)) || is_array) {
                self.error(
                    module,
                    codes::PARAMETER_RULE,
                    p.span,
                    "an `Optional` parameter cannot be a user-defined type or an array",
                );
            }
            // MS-VBAL 5.3.1.5: an optional parameter whose type comes only from
            // a Def directive needs a default value.
            if p.optional
                && p.default.is_none()
                && p.as_clause.is_none()
                && p.name.suffix.is_none()
                && !elem.is_variant()
                && !elem.is_error()
            {
                self.spec_strict(
                    module,
                    codes::PARAMETER_RULE,
                    p.span,
                    "an `Optional` parameter typed only by a Def directive needs a default value",
                );
            }
            let mut default = None;
            if let Some(d) = &p.default {
                if !p.optional {
                    self.error(
                        module,
                        codes::PARAMETER_RULE,
                        d.span,
                        "only `Optional` parameters can have a default value",
                    );
                } else {
                    match fold(&scope, d) {
                        Ok((c, _)) => {
                            default = if elem.is_variant() {
                                Some(c)
                            } else if elem.is_error() {
                                None
                            } else {
                                match convert_constant(&c, &elem) {
                                    Ok(c) => Some(c),
                                    Err(m) => {
                                        self.error(
                                            module,
                                            codes::PARAMETER_RULE,
                                            d.span,
                                            format!("default value: {m}"),
                                        );
                                        None
                                    }
                                }
                            };
                        }
                        Err(e) => self.error(module, codes::CONSTANT_EXPRESSION, e.span, e.message),
                    }
                }
            }
            out.push(ParamInfo {
                name: p.name.name.clone(),
                ty,
                by_val: p.pass_by == Some(PassBy::ByVal),
                optional: p.optional,
                default,
                param_array: p.param_array,
                is_array,
                span: p.span,
            });
        }
        out
    }

    /// MS-VBAL 5.3.1: the declared type of a function may not be a private
    /// enum, nor a private user-defined type in a class module.
    fn check_result_type_visibility(&self, module: ModuleId, p: &vba_ast::ProcDecl, t: &Type) {
        if p.visibility == Some(vba_ast::Visibility::Private) {
            return;
        }
        let class_like = self.program.modules[module.index()].is_class_like();
        let what = match t {
            Type::Enum(e)
                if self.program.enums[e.index()].visibility == vba_ast::Visibility::Private =>
            {
                "private enum"
            }
            Type::Udt(u)
                if class_like
                    && self.program.udts[u.index()].visibility == vba_ast::Visibility::Private =>
            {
                "private user-defined type"
            }
            _ => return,
        };
        self.spec_strict(
            module,
            codes::PROCEDURE_RULE,
            p.name.span,
            format!("the type of a non-private function cannot be a {what}"),
        );
    }

    pub(crate) fn resolve_signatures(&mut self) {
        self.evaluate_all_constants();

        // User-defined types.
        let mut udt_fields = Vec::new();
        for (i, decl) in self.udt_src.iter().enumerate() {
            let module = self.program.udts[i].module;
            let scope = ModuleScope { sema: self, module };
            let mut fields: Vec<FieldInfo> = Vec::new();
            for d in &decl.members {
                if fields.iter().any(|f| f.name == d.name.name) {
                    self.error(
                        module,
                        codes::DUPLICATE_DECLARATION,
                        d.name.span,
                        format!("duplicate field `{}`", d.name.name),
                    );
                    continue;
                }
                let dt = self.decl_type(
                    module,
                    &d.name,
                    d.as_clause.as_ref(),
                    d.bounds.as_deref(),
                    &scope,
                );
                if let Some(a) = &d.as_clause
                    && a.is_new
                {
                    self.error(
                        module,
                        codes::TYPE_DECLARATION,
                        a.span,
                        "a field of a user-defined type cannot be `As New`",
                    );
                }
                if matches!(&dt.ty, Type::Array { rank: None, .. }) {
                    // Dynamic arrays are fine in a UDT.
                }
                fields.push(FieldInfo {
                    name: d.name.name.clone(),
                    ty: dt.ty,
                    fixed_bounds: dt.fixed_bounds,
                    span: d.span,
                });
            }
            udt_fields.push(fields);
        }
        for (i, f) in udt_fields.into_iter().enumerate() {
            self.program.udts[i].fields = f;
        }
        self.check_udt_cycles();

        // Module-level variables.
        let mut updates = Vec::new();
        for (i, src) in self.global_src.iter().enumerate() {
            let Some(d) = src else { continue };
            let g = &self.program.globals[i];
            let scope = ModuleScope {
                sema: self,
                module: g.module,
            };
            let dt = self.decl_type(
                g.module,
                &d.name,
                d.as_clause.as_ref(),
                d.bounds.as_deref(),
                &scope,
            );
            updates.push((i, dt));
        }
        for (i, dt) in updates {
            let g = &mut self.program.globals[i];
            g.ty = dt.ty;
            g.auto_new = dt.auto_new;
            g.fixed_bounds = dt.fixed_bounds;
        }

        // Declares.
        let mut dec = Vec::new();
        for (i, d) in self.declare_src.iter().enumerate() {
            let module = self.program.declares[i].module;
            let params = self.build_params(module, &d.params, ParamCtx::Declare);
            let ret = match (&d.return_type, d.kind) {
                (Some(a), _) => self.resolve_as_clause(module, a, false),
                (None, vba_ast::DeclareKind::Function) => match d.name.suffix {
                    Some(s) => self.suffix_type(s),
                    None => self.program.modules[module.index()]
                        .options
                        .default_type_for(&d.name.name),
                },
                (None, _) => Type::Variant,
            };
            dec.push((params, ret));
        }
        for (i, (p, r)) in dec.into_iter().enumerate() {
            self.program.declares[i].params = p;
            self.program.declares[i].ret = r;
        }

        // Events.
        let mut evs = Vec::new();
        for (i, e) in self.event_src.iter().enumerate() {
            let module = self.program.events[i].module;
            evs.push(self.build_params(module, &e.params, ParamCtx::Event));
        }
        for (i, p) in evs.into_iter().enumerate() {
            self.program.events[i].params = p;
        }

        // Procedures.
        let mut sigs = Vec::new();
        for (i, p) in self.proc_src.iter().enumerate() {
            let module = self.program.procs[i].module;
            let params = self.build_params(module, &p.params, ParamCtx::Proc);
            let (ret, has_result) = match p.kind {
                ProcKind::Function | ProcKind::PropertyGet => {
                    if p.return_type.is_some() && p.name.suffix.is_some() {
                        self.spec_strict(
                            module,
                            codes::PROCEDURE_RULE,
                            p.name.span,
                            "a function name with a type character cannot also have an `As` clause",
                        );
                    }
                    let t = match &p.return_type {
                        Some(a) => {
                            let t = self.resolve_as_clause(module, a, false);
                            self.check_result_type_visibility(module, p, &t);
                            if a.is_array && !t.is_error() {
                                Type::Array {
                                    elem: Box::new(t),
                                    rank: None,
                                }
                            } else {
                                t
                            }
                        }
                        None => match p.name.suffix {
                            Some(s) => self.suffix_type(s),
                            None => self.program.modules[module.index()]
                                .options
                                .default_type_for(&p.name.name),
                        },
                    };
                    (t, true)
                }
                _ => (Type::Variant, false),
            };
            sigs.push((params, ret, has_result));
        }
        for (i, (params, ret, has_result)) in sigs.into_iter().enumerate() {
            let pi = &mut self.program.procs[i];
            pi.params = params;
            pi.ret = ret;
            pi.has_result = has_result;
        }
    }

    fn check_udt_cycles(&self) {
        // A UDT must not contain itself by value: it is on a cycle of the
        // "contains" graph. Strongly connected components (Tarjan, iterative:
        // the chains can be as long as the source is big) find every such
        // type in time linear in the number of types and fields.
        let udts = &self.program.udts;
        let succ: Vec<Vec<usize>> = udts
            .iter()
            .map(|u| {
                u.fields
                    .iter()
                    .filter_map(|f| match &f.ty {
                        Type::Udt(t) => Some(t.index()),
                        _ => None,
                    })
                    .collect()
            })
            .collect();
        let cyclic = cyclic_nodes(&succ);
        for u in udts {
            if cyclic[u.id.index()] {
                self.error(
                    u.module,
                    codes::TYPE_DECLARATION,
                    u.span,
                    format!("user-defined type `{}` contains itself", u.name),
                );
            }
        }
    }

    /// Hoists `Static` locals into [`GlobalInfo`]s owned by their procedure.
    pub(crate) fn hoist_statics(&mut self) {
        let mut new_globals = Vec::new();
        for (pi, p) in self.proc_src.iter().enumerate() {
            let module = self.program.procs[pi].module;
            let scope = ModuleScope { sema: self, module };
            walk_block(&p.body, &mut |s: &Stmt| {
                if let StmtKind::Dim(v) = &s.kind
                    && (v.is_static || p.is_static)
                {
                    for d in &v.declarators {
                        let dt = self.decl_type(
                            module,
                            &d.name,
                            d.as_clause.as_ref(),
                            d.bounds.as_deref(),
                            &scope,
                        );
                        new_globals.push((pi, d.name.name.clone(), d.span, dt));
                    }
                }
            });
        }
        for (pi, name, span, dt) in new_globals {
            let id = GlobalId(self.program.globals.len() as u32);
            let module = self.program.procs[pi].module;
            self.program.globals.push(GlobalInfo {
                id,
                module,
                name: name.clone(),
                ty: dt.ty,
                visibility: vba_ast::Visibility::Private,
                with_events: false,
                auto_new: dt.auto_new,
                static_of: Some(ProcId(pi as u32)),
                fixed_bounds: dt.fixed_bounds,
                span,
            });
            self.global_src.push(None);
            self.statics.entry((ProcId(pi as u32), name)).or_insert(id);
        }
    }
}

/// The nodes of a directed graph that lie on a cycle (including self loops):
/// strongly connected components by an iterative Tarjan search, linear in
/// the size of the graph.
pub(crate) fn cyclic_nodes(succ: &[Vec<usize>]) -> Vec<bool> {
    let n = succ.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut cyclic = vec![false; n];
    let mut next = 0usize;
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        // Frames: (node, next successor to look at).
        let mut call: Vec<(usize, usize)> = vec![(root, 0)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&mut (v, ref mut i)) = call.last_mut() {
            if *i < succ[v].len() {
                let w = succ[v][*i];
                *i += 1;
                if w == v {
                    cyclic[v] = true;
                }
                if index[w] == usize::MAX {
                    index[w] = next;
                    low[w] = next;
                    next += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                call.pop();
                if let Some(&(parent, _)) = call.last() {
                    low[parent] = low[parent].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut members = Vec::new();
                    while let Some(w) = stack.pop() {
                        on_stack[w] = false;
                        members.push(w);
                        if w == v {
                            break;
                        }
                    }
                    if members.len() > 1 {
                        for m in members {
                            cyclic[m] = true;
                        }
                    }
                }
            }
        }
    }
    cyclic
}
