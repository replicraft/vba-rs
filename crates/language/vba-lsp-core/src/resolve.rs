//! Binding of identifier tokens to declarations of the analysed project.
//!
//! `vba-sema` supplies the declaration tables (module symbols, procedure
//! locals, declared types, public/private visibility); this module walks
//! the editor's tolerant syntax tree and resolves each name with the same
//! rules (innermost scope, own module, other standard modules, module
//! names) and with declared types for member access. Working from the
//! declarations rather than from bound expressions keeps the answer
//! available while the statement being typed is still incomplete
//! (`x.`).

use vba_ast::Name;
use vba_lexer::{Keyword, TokenKind};
use vba_sema::{LibMember, LibraryType, LocalKind, ModuleId, Symbol, Type};
use vba_syntax::{NodeKind, SyntaxNode, SyntaxToken};

use crate::project::{Decl, Project, mid};
use crate::util::normalize;

/// What an identifier token is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Binding {
    /// A declaration of the project.
    Decl(Decl),
    /// A member access that VBA resolves at run time (receiver is a
    /// `Variant` or `Object`).
    Late,
    /// Not known to the project (library names, unresolved names, names in
    /// code the analysis could not follow).
    Unknown,
}

/// The static meaning of an expression.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Ty {
    Known(Type),
    /// A standard module used as a qualifier.
    Module(ModuleId),
    /// An enum used as a qualifier.
    Enum(vba_sema::EnumId),
    /// The project name used as a qualifier.
    Project,
    Unknown,
}

/// Resolution context: one project and one module's tokens.
pub(crate) struct Ctx<'a> {
    pub proj: &'a Project,
    pub m: usize,
}

impl<'a> Ctx<'a> {
    pub fn new(proj: &'a Project, m: usize) -> Self {
        Ctx { proj, m }
    }

    fn mid(&self) -> ModuleId {
        mid(self.m)
    }

    /// The procedure of this module containing `offset`.
    pub fn proc_at(&self, offset: u32) -> Option<&'a vba_sema::ProcInfo> {
        let prog = &self.proj.sema.program;
        prog.module(self.mid())
            .procs
            .iter()
            .map(|p| prog.proc(*p))
            .find(|p| p.span.start <= offset && offset < p.span.end)
    }

    fn symbol_decl(&self, sym: &Symbol, m: ModuleId) -> Option<Decl> {
        self.proj.decl_of_symbol(m, sym)
    }

    /// Resolves a simple name used at `offset`. With `types_only`, only
    /// type-like names are considered.
    pub fn resolve_simple(&self, norm: &str, offset: u32, types_only: bool) -> Option<Decl> {
        let prog = &self.proj.sema.program;
        let name = Name::new(norm);
        if !types_only && let Some(p) = self.proc_at(offset) {
            for (i, l) in p.locals.iter().enumerate() {
                if normalize(l.name.as_bytes()) != norm {
                    continue;
                }
                match l.kind {
                    // An implicit local named like a library qualifier
                    // or function comes from a statement the analysis
                    // could not complete (`x = VBA.`).
                    LocalKind::Implicit
                        if crate::completion::QUALIFIERS
                            .iter()
                            .any(|q| q.eq_ignore_ascii_case(norm))
                            || vba_stdlib::lookup(norm).is_some() => {}
                    LocalKind::Param(_) | LocalKind::Var | LocalKind::Implicit => {
                        return Some(Decl::Local(p.id, i as u32));
                    }
                    LocalKind::Result => {
                        return Some(Decl::Proc(
                            p.module,
                            p.name.to_string_lossy().to_ascii_lowercase(),
                        ));
                    }
                    LocalKind::Temp => {}
                }
            }
            if let Some(g) = prog
                .globals
                .iter()
                .find(|g| g.static_of == Some(p.id) && normalize(g.name.as_bytes()) == norm)
            {
                return Some(Decl::Global(g.id));
            }
        }
        let type_like = |s: &Symbol| matches!(s, Symbol::Udt(_) | Symbol::Enum(_));
        if let Some((sym, _)) = prog.module(self.mid()).symbols.get(&name)
            && (!types_only || type_like(sym))
            && let Some(d) = self.symbol_decl(sym, self.mid())
        {
            return Some(d);
        }
        let mut found: Vec<Decl> = Vec::new();
        for (i, o) in prog.modules.iter().enumerate() {
            if i == self.m || i >= self.proj.mods.len() {
                continue;
            }
            let Some((sym, _)) = o.symbols.get(&name) else {
                continue;
            };
            let global = o.kind == vba_project::ModuleKind::Standard
                || matches!(
                    sym,
                    Symbol::Udt(_) | Symbol::Enum(_) | Symbol::EnumMember(..)
                );
            let ok_kind = !types_only || type_like(sym);
            if global
                && ok_kind
                && !(o.options.private_module && o.kind == vba_project::ModuleKind::Standard)
                && self.proj.symbol_public(sym)
                && let Some(d) = self.symbol_decl(sym, o.id)
            {
                found.push(d);
            }
        }
        if found.len() == 1 {
            return found.pop();
        }
        if found.len() > 1 {
            return None;
        }
        // Module names: default instances and qualifiers.
        let m = prog
            .modules
            .iter()
            .take(self.proj.mods.len())
            .find(|m| m.name == name)?;
        if types_only && m.kind == vba_project::ModuleKind::Standard {
            return None;
        }
        Some(Decl::Module(m.id))
    }

    fn module_by_norm(&self, norm: &str) -> Option<ModuleId> {
        let name = Name::new(norm);
        self.proj
            .sema
            .program
            .modules
            .iter()
            .take(self.proj.mods.len())
            .find(|m| m.name == name)
            .map(|m| m.id)
    }

    /// The type of a declaration as an expression.
    pub fn decl_ty(&self, d: &Decl) -> Ty {
        let prog = &self.proj.sema.program;
        match d {
            Decl::Module(m) => {
                if prog.module(*m).kind == vba_project::ModuleKind::Standard {
                    Ty::Module(*m)
                } else {
                    Ty::Known(Type::Class(*m))
                }
            }
            Decl::Enum(e) => Ty::Enum(*e),
            d => self.proj.decl_type(d).map_or(Ty::Unknown, Ty::Known),
        }
    }

    /// Looks `norm` up as a member of a receiver.
    pub fn member(&self, recv: &Ty, norm: &str) -> Binding {
        let prog = &self.proj.sema.program;
        let name = Name::new(norm);
        let in_module = |m: ModuleId| -> Binding {
            match prog.module(m).symbols.get(&name) {
                Some((s, _)) => self
                    .symbol_decl(s, m)
                    .map_or(Binding::Unknown, Binding::Decl),
                None => Binding::Unknown,
            }
        };
        match recv {
            Ty::Module(m) => in_module(*m),
            Ty::Project => self
                .module_by_norm(norm)
                .map_or(Binding::Unknown, |m| Binding::Decl(Decl::Module(m))),
            Ty::Enum(e) => {
                let info = &prog.enums[e.index()];
                info.members
                    .iter()
                    .position(|(n, _)| normalize(n.as_bytes()) == norm)
                    .map_or(Binding::Unknown, |i| {
                        Binding::Decl(Decl::EnumMember(*e, i as u32))
                    })
            }
            Ty::Known(Type::Class(m)) => in_module(*m),
            Ty::Known(Type::Udt(u)) => prog.udts[u.index()]
                .fields
                .iter()
                .position(|f| normalize(f.name.as_bytes()) == norm)
                .map_or(Binding::Unknown, |i| {
                    Binding::Decl(Decl::Field(*u, i as u32))
                }),
            Ty::Known(Type::Variant | Type::Object | Type::Any) => Binding::Late,
            Ty::Known(_) | Ty::Unknown => Binding::Unknown,
        }
    }

    /// The described host class of this module, if it has one.
    pub fn host_class(&self) -> Option<&'a LibraryType> {
        self.proj.host_classes.get(&self.m)
    }

    /// The host class of module `m` as the type of its instances.
    fn host_class_of(&self, m: ModuleId) -> Option<&'a LibraryType> {
        self.proj.host_classes.get(&(m.0 as usize))
    }

    /// A member of this module's described host class: a control or
    /// property that a bare name denotes.
    pub fn host_member(&self, name: &str) -> Option<(LibraryType, LibMember)> {
        let lt = self.host_class()?;
        let p = self.proj.provider.as_deref()?;
        p.member(lt, name).map(|m| (lt.clone(), m))
    }

    /// The library type and member a member access denotes, when the
    /// receiver is a library type and the provider describes the member.
    /// An instance of a module with a described host class also answers for
    /// the members of that class.
    pub fn lib_member(&self, recv: &Ty, name: &str) -> Option<(LibraryType, LibMember)> {
        let host;
        let lt = match recv {
            Ty::Known(Type::Library(lt)) => lt,
            Ty::Known(Type::Class(m)) => {
                host = self.host_class_of(*m)?;
                host
            }
            _ => return None,
        };
        let p = self.proj.provider.as_deref()?;
        let direct = p.member(lt, name).map(|m| (lt.clone(), m));
        direct.or_else(|| {
            let inner = p.class_info(lt)?.default_interface?;
            p.member(&inner, name).map(|m| (inner, m))
        })
    }

    /// The type of a predeclared global object of a described library
    /// (`Application`), or `Unknown`.
    fn lib_global_ty(&self, name: &[u8]) -> Ty {
        let Some(p) = self.proj.provider.as_deref() else {
            return Ty::Unknown;
        };
        match p.global(&String::from_utf8_lossy(name)) {
            Some(vba_sema::LibGlobal::Object(t)) => Ty::Known(t),
            Some(vba_sema::LibGlobal::Constant(_, t)) => Ty::Known(t),
            _ => Ty::Unknown,
        }
    }

    /// Every member of a library type the provider can enumerate.
    pub fn lib_members(&self, lt: &LibraryType) -> Vec<LibMember> {
        let Some(p) = self.proj.provider.as_deref() else {
            return Vec::new();
        };
        p.interface_members(lt)
            .or_else(|| {
                let inner = p.class_info(lt)?.default_interface?;
                p.interface_members(&inner)
            })
            .unwrap_or_default()
    }

    fn binding_ty(&self, b: &Binding) -> Ty {
        match b {
            Binding::Decl(d) => self.decl_ty(d),
            Binding::Late => Ty::Known(Type::Variant),
            Binding::Unknown => Ty::Unknown,
        }
    }

    /// The expression a `With` block is about, for a member expression
    /// without a receiver.
    fn with_expr<'t>(&self, node: SyntaxNode<'t>) -> Option<SyntaxNode<'t>> {
        let mut cur = node;
        while let Some(p) = cur.parent() {
            if p.kind() == NodeKind::WithStmt {
                let first = p.child_nodes().next()?;
                // The node must be inside the body, not the expression.
                return (first.span().end <= node.span().start).then_some(first);
            }
            cur = p;
        }
        None
    }

    fn name_token<'t>(node: SyntaxNode<'t>) -> Option<SyntaxToken<'t>> {
        node.child_tokens().find(|t| {
            matches!(
                t.kind(),
                TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_)
            )
        })
    }

    /// The static meaning of an expression node.
    pub fn expr_ty(&self, node: SyntaxNode<'_>) -> Ty {
        match node.kind() {
            NodeKind::NameExpr => {
                let Some(t) = Self::name_token(node) else {
                    return Ty::Unknown;
                };
                if t.kind() == TokenKind::Keyword(Keyword::Me) {
                    return if self.proj.mods[self.m].kind != vba_project::ModuleKind::Standard {
                        Ty::Known(Type::Class(self.mid()))
                    } else {
                        Ty::Unknown
                    };
                }
                let norm = normalize(t.text());
                match self.resolve_simple(&norm, t.span().start, false) {
                    Some(d) => self.decl_ty(&d),
                    None if norm == normalize(self.proj.project_name.as_bytes()) => Ty::Project,
                    None => match self.host_member(&String::from_utf8_lossy(t.text())) {
                        Some((_, m)) => Ty::Known(m.ty),
                        None => self.lib_global_ty(t.text()),
                    },
                }
            }
            NodeKind::ParenExpr => node
                .child_nodes()
                .next()
                .map_or(Ty::Unknown, |n| self.expr_ty(n)),
            NodeKind::MemberExpr => {
                let b = self.member_binding(node);
                if b == Binding::Unknown
                    && let Some(name) = Self::name_token(node)
                    && let Some((_, m)) = self.lib_member(
                        &self.receiver_ty(node),
                        &String::from_utf8_lossy(name.text()),
                    )
                {
                    return Ty::Known(m.ty);
                }
                self.binding_ty(&b)
            }
            NodeKind::CallExpr => {
                let Some(callee) = node.child_nodes().next() else {
                    return Ty::Unknown;
                };
                match self.expr_ty(callee) {
                    Ty::Known(Type::Array { elem, .. }) => Ty::Known(*elem),
                    Ty::Known(Type::Variant | Type::Object) => Ty::Known(Type::Variant),
                    Ty::Known(t) => Ty::Known(t),
                    _ => Ty::Unknown,
                }
            }
            NodeKind::NewExpr => node
                .child_node(NodeKind::TypeRef)
                .and_then(|t| {
                    let tok = t.child_tokens().last()?;
                    match self.type_binding(&tok) {
                        Binding::Decl(Decl::Module(m)) => Some(Ty::Known(Type::Class(m))),
                        _ => None,
                    }
                })
                .unwrap_or(Ty::Unknown),
            _ => Ty::Unknown,
        }
    }

    /// The meaning of the receiver of a `MemberExpr`: its expression, or the
    /// object of the enclosing `With`.
    pub fn receiver_ty(&self, node: SyntaxNode<'_>) -> Ty {
        match node.child_nodes().next() {
            Some(r) => self.expr_ty(r),
            None => match self.with_expr(node) {
                Some(w) => self.expr_ty(w),
                None => Ty::Unknown,
            },
        }
    }

    /// Binding of the member name of a `MemberExpr`.
    pub fn member_binding(&self, node: SyntaxNode<'_>) -> Binding {
        let Some(name) = Self::name_token(node) else {
            return Binding::Unknown;
        };
        self.member(&self.receiver_ty(node), &normalize(name.text()))
    }

    /// Binding of a name in a type position (`As Foo`, `Mod.Foo`).
    pub fn type_binding(&self, tok: &SyntaxToken<'_>) -> Binding {
        let norm = normalize(tok.text());
        let parent = tok.parent();
        let segs: Vec<SyntaxToken<'_>> = parent
            .child_tokens()
            .filter(|t| {
                matches!(
                    t.kind(),
                    TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_)
                )
            })
            .collect();
        let Some(k) = segs.iter().position(|t| t.span() == tok.span()) else {
            return Binding::Unknown;
        };
        if k == 0 {
            return self
                .resolve_simple(&norm, tok.span().start, true)
                .map_or(Binding::Unknown, Binding::Decl);
        }
        // Qualified: the previous segment is a module (optionally after
        // the project name).
        let q = normalize(segs[k - 1].text());
        let Some(m) = self.module_by_norm(&q) else {
            return Binding::Unknown;
        };
        match self.member(&Ty::Module(m), &norm) {
            Binding::Decl(d @ (Decl::Udt(_) | Decl::Enum(_))) => Binding::Decl(d),
            _ => Binding::Unknown,
        }
    }

    /// The binding of an identifier token of this module.
    pub fn binding(&self, tok: &SyntaxToken<'_>) -> Binding {
        if !matches!(
            tok.kind(),
            TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_)
        ) {
            return Binding::Unknown;
        }
        let pm = &self.proj.mods[self.m];
        if let Some((span, _)) = pm.ident_at(tok.span().start)
            && let Some(d) = self.proj.decl_at.get(&(self.m as u32, span.start))
        {
            return Binding::Decl(d.clone());
        }
        let parent = tok.parent();
        let norm = normalize(tok.text());
        match parent.kind() {
            NodeKind::NameExpr => {
                if tok.kind() == TokenKind::Keyword(Keyword::Me) {
                    return Binding::Unknown;
                }
                self.resolve_simple(&norm, tok.span().start, false)
                    .map_or(Binding::Unknown, Binding::Decl)
            }
            NodeKind::MemberExpr => {
                let is_member_name = parent
                    .child_tokens()
                    .find(|t| {
                        matches!(
                            t.kind(),
                            TokenKind::Ident | TokenKind::ForeignName | TokenKind::Keyword(_)
                        )
                    })
                    .is_some_and(|t| t.span() == tok.span());
                if is_member_name {
                    self.member_binding(parent)
                } else {
                    Binding::Unknown
                }
            }
            NodeKind::TypeRef => self.type_binding(tok),
            NodeKind::RaiseEventStmt => self
                .resolve_simple(&norm, tok.span().start, false)
                .map_or(Binding::Unknown, Binding::Decl),
            _ => Binding::Unknown,
        }
    }
}
