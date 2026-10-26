//! Procedure body binding: name resolution, typing, call classification
//! and statement checks (pass 3).

mod call;
mod expr;
mod place;
mod stmt;

use std::collections::HashMap;

use vba_ast::{ExitKind, Name, ProcDecl, Stmt, StmtKind};
use vba_profile::Profile;
use vba_source::Span;

use crate::bound::{BExpr, BExprKind, BStmt, Constant};
use crate::codes;
use crate::fold::{ConstLookup, ConstScope, convert_constant, fold};
use crate::program::*;
use crate::sema::{ModuleScope, Sema};
use crate::types::*;
use crate::walk::walk_block;

/// A name in procedure scope.
#[derive(Debug, Clone)]
pub(crate) enum LocalSym {
    Local(LocalId),
    Static(GlobalId),
    Const(Constant, Type),
}

/// An enclosing `With`.
pub(crate) struct WithEntry {
    /// The expression that denotes the With object at each use.
    pub receiver: BExpr,
}

/// Enclosing loop kinds, for `Exit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopKind {
    For,
    Do,
    While,
}

/// The result of binding one procedure.
pub(crate) struct BoundProc {
    pub locals: Vec<LocalInfo>,
    pub result_local: Option<LocalId>,
    pub labels: Vec<LabelInfo>,
    pub body: Vec<BStmt>,
}

pub(crate) struct Binder<'s, 'a> {
    pub sema: &'s Sema<'a>,
    pub module: ModuleId,
    pub proc: ProcId,
    pub ast: &'a ProcDecl,
    pub locals: Vec<LocalInfo>,
    pub scope: HashMap<Name, LocalSym>,
    pub labels: Vec<LabelInfo>,
    pub label_map: HashMap<LabelKey, LabelId>,
    pub withs: Vec<WithEntry>,
    pub loops: Vec<LoopKind>,
    pub for_counters: Vec<Option<Name>>,
    pub result_local: Option<LocalId>,
    pub explicit: bool,
    pub class_like: bool,
}

/// Constant scope of a procedure: its local constants, then module scope.
pub(crate) struct LocalScope<'b, 's, 'a> {
    pub scope: &'b HashMap<Name, LocalSym>,
    pub inner: ModuleScope<'s, 'a>,
}

impl ConstScope for LocalScope<'_, '_, '_> {
    fn lookup(&self, path: &[&vba_ast::Ident]) -> ConstLookup {
        if let [one] = path
            && let Some(sym) = self.scope.get(&one.name)
        {
            return match sym {
                LocalSym::Const(c, t) => ConstLookup::Found(c.clone(), t.clone()),
                LocalSym::Local(_) | LocalSym::Static(_) => {
                    ConstLookup::NotConstant("a variable".to_owned())
                }
            };
        }
        self.inner.lookup(path)
    }

    fn profile(&self) -> &Profile {
        self.inner.profile()
    }

    fn compare(&self) -> vba_ast::CompareMode {
        self.inner.compare()
    }
}

pub(crate) fn bind_proc(sema: &Sema<'_>, id: ProcId) -> BoundProc {
    let info = &sema.program.procs[id.index()];
    let module = info.module;
    let mi = &sema.program.modules[module.index()];
    let mut b = Binder {
        sema,
        module,
        proc: id,
        ast: sema.proc_src[id.index()],
        locals: Vec::new(),
        scope: HashMap::new(),
        labels: Vec::new(),
        label_map: HashMap::new(),
        withs: Vec::new(),
        loops: Vec::new(),
        for_counters: Vec::new(),
        result_local: None,
        explicit: mi.options.explicit,
        class_like: mi.is_class_like(),
    };
    b.declare_params();
    b.prescan();
    let block = b.ast.body.clone();
    let body = b.bind_block(&block);
    BoundProc {
        locals: b.locals,
        result_local: b.result_local,
        labels: b.labels,
        body,
    }
}

impl<'s, 'a> Binder<'s, 'a> {
    pub(crate) fn info(&self) -> &'s ProcInfo {
        &self.sema.program.procs[self.proc.index()]
    }

    pub(crate) fn options(&self) -> &'s ModuleOptions {
        &self.sema.program.modules[self.module.index()].options
    }

    pub(crate) fn err(&self, code: vba_diagnostics::Code, span: Span, msg: impl Into<String>) {
        self.sema.error(self.module, code, span, msg);
    }

    pub(crate) fn tn(&self, t: &Type) -> String {
        self.sema.program.type_name(t)
    }

    pub(crate) fn error_expr(&self, span: Span) -> BExpr {
        BExpr::new(
            span,
            Type::Error,
            BExprKind::Const(Constant::Value(vba_value::Variant::Empty)),
        )
    }

    pub(crate) fn local_ty(&self, l: LocalId) -> Type {
        self.locals[l.index()].ty.clone()
    }

    pub(crate) fn new_local(&mut self, info: LocalInfo) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(info);
        id
    }

    pub(crate) fn me_type(&self) -> Type {
        Type::Class(self.module)
    }

    fn declare_params(&mut self) {
        let info = self.info();
        for (i, p) in info.params.iter().enumerate() {
            let id = self.new_local(LocalInfo {
                name: p.name.clone(),
                ty: p.ty.clone(),
                kind: LocalKind::Param(i as u32),
                fixed_bounds: None,
                auto_new: false,
                span: p.span,
            });
            self.scope
                .entry(p.name.clone())
                .or_insert(LocalSym::Local(id));
        }
        if info.has_result {
            let id = self.new_local(LocalInfo {
                name: info.name.clone(),
                ty: info.ret.clone(),
                kind: LocalKind::Result,
                fixed_bounds: None,
                auto_new: false,
                span: info.name_span,
            });
            self.result_local = Some(id);
        }
    }

    fn local_scope(&self) -> LocalScope<'_, 's, 'a> {
        LocalScope {
            scope: &self.scope,
            inner: ModuleScope {
                sema: self.sema,
                module: self.module,
            },
        }
    }

    /// Declares locals, local constants and labels before binding, because
    /// procedure scope is not ordered.
    fn prescan(&mut self) {
        let ast = self.ast;
        let is_static_proc = ast.is_static;
        walk_block(&ast.body, &mut |s: &Stmt| match &s.kind {
            StmtKind::Dim(v) => {
                for d in &v.declarators {
                    if d.name.is_missing() {
                        continue;
                    }
                    if self.scope.contains_key(&d.name.name) {
                        self.err(
                            codes::DUPLICATE_DECLARATION,
                            d.name.span,
                            format!(
                                "duplicate declaration of `{}` in this procedure",
                                d.name.name
                            ),
                        );
                        continue;
                    }
                    if v.is_static || is_static_proc {
                        if let Some(g) = self.sema.statics.get(&(self.proc, d.name.name.clone())) {
                            self.scope.insert(d.name.name.clone(), LocalSym::Static(*g));
                        }
                        continue;
                    }
                    let dt = {
                        let ls = self.local_scope();
                        self.sema.decl_type(
                            self.module,
                            &d.name,
                            d.as_clause.as_ref(),
                            d.bounds.as_deref(),
                            &ls,
                        )
                    };
                    let id = self.new_local(LocalInfo {
                        name: d.name.name.clone(),
                        ty: dt.ty,
                        kind: LocalKind::Var,
                        fixed_bounds: dt.fixed_bounds,
                        auto_new: dt.auto_new,
                        span: d.span,
                    });
                    self.scope.insert(d.name.name.clone(), LocalSym::Local(id));
                }
            }
            StmtKind::Const(c) => {
                for it in &c.items {
                    if it.name.is_missing() {
                        continue;
                    }
                    if self.scope.contains_key(&it.name.name) {
                        self.err(
                            codes::DUPLICATE_DECLARATION,
                            it.name.span,
                            format!(
                                "duplicate declaration of `{}` in this procedure",
                                it.name.name
                            ),
                        );
                        continue;
                    }
                    let folded = {
                        let ls = self.local_scope();
                        fold(&ls, &it.value)
                    };
                    match folded {
                        Err(e) => self.err(codes::CONSTANT_EXPRESSION, e.span, e.message),
                        Ok((c, t)) => {
                            let declared = self.sema.const_declared_type(self.module, it);
                            let (c, t) = match declared {
                                Some(dt) if !dt.is_error() => match convert_constant(&c, &dt) {
                                    Ok(c2) => (c2, dt),
                                    Err(m) => {
                                        self.err(
                                            codes::CONSTANT_EXPRESSION,
                                            it.value.span,
                                            format!("constant `{}`: {m}", it.name.name),
                                        );
                                        continue;
                                    }
                                },
                                Some(_) => continue,
                                None => (c, t),
                            };
                            self.scope
                                .insert(it.name.name.clone(), LocalSym::Const(c, t));
                        }
                    }
                }
            }
            StmtKind::Label(l) => {
                let key = match &l.label {
                    vba_ast::LabelRef::Name(i) => LabelKey::Name(i.name.clone()),
                    vba_ast::LabelRef::LineNumber { value: Some(v), .. } => LabelKey::Line(*v),
                    vba_ast::LabelRef::LineNumber { span, .. } => {
                        self.err(codes::LABEL, *span, "line number is too large");
                        return;
                    }
                };
                if let Some(prev) = self.label_map.get(&key) {
                    let prev_span = self.labels[prev.index()].span;
                    self.sema.error_with_label(
                        self.module,
                        codes::LABEL,
                        l.span,
                        "duplicate label",
                        prev_span,
                        "first defined here",
                    );
                    return;
                }
                let id = LabelId(self.labels.len() as u32);
                let line = match &key {
                    LabelKey::Line(v) => Some(*v),
                    LabelKey::Name(_) => None,
                };
                self.labels.push(LabelInfo {
                    key: key.clone(),
                    span: l.span,
                    line,
                });
                self.label_map.insert(key, id);
            }
            _ => {}
        });
    }

    /// Resolves a label reference.
    pub(crate) fn label_id(&self, r: &vba_ast::LabelRef) -> Option<LabelId> {
        let key = match r {
            vba_ast::LabelRef::Name(i) => LabelKey::Name(i.name.clone()),
            vba_ast::LabelRef::LineNumber { value: Some(v), .. } => LabelKey::Line(*v),
            vba_ast::LabelRef::LineNumber { span, .. } => {
                self.err(codes::LABEL, *span, "line number is too large");
                return None;
            }
        };
        match self.label_map.get(&key) {
            Some(id) => Some(*id),
            None => {
                self.err(
                    codes::LABEL,
                    r.span(),
                    "label is not defined in this procedure",
                );
                None
            }
        }
    }

    pub(crate) fn exit_allowed(&self, k: ExitKind) -> bool {
        use vba_ast::ProcKind as PK;
        match k {
            ExitKind::Sub => self.info().kind == PK::Sub,
            ExitKind::Function => self.info().kind == PK::Function,
            ExitKind::Property => matches!(
                self.info().kind,
                PK::PropertyGet | PK::PropertyLet | PK::PropertySet
            ),
            ExitKind::For => self.loops.contains(&LoopKind::For),
            ExitKind::Do => self.loops.contains(&LoopKind::Do),
        }
    }
}
