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
            TypeRefKind::Named(idents) => self.resolve_named_type(module, idents, t.span),
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
            if auto_new {
                self.error(
                    module,
                    codes::ARRAY_RULE,
                    name.span,
                    "an array cannot be declared `As New`",
                );
                auto_new = false;
            }
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
                    let t = match &p.return_type {
                        Some(a) => {
                            let t = self.resolve_as_clause(module, a, false);
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
        // A UDT must not contain itself by value.
        for u in &self.program.udts {
            let mut stack = vec![u.id];
            let mut seen = std::collections::HashSet::new();
            let mut cyclic = false;
            while let Some(cur) = stack.pop() {
                for f in &self.program.udts[cur.index()].fields {
                    if let Type::Udt(n) = &f.ty {
                        if *n == u.id {
                            cyclic = true;
                        } else if seen.insert(*n) {
                            stack.push(*n);
                        }
                    }
                }
            }
            if cyclic {
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
