//! Names that belong to the described host class of a module (see
//! `Options::module_hosts`): the controls, properties and methods of the form
//! whose `Me` the embedding application describes.
//!
//! Such a member is bound like a member of a library type: typed, checked
//! against the description and early bound by id. The receiver is the
//! instance itself retyped as the host class; at run time the module's own
//! members answer first and the rest reaches the host base object by name
//! (or by id, when the description gives one).

use vba_ast::{Arg, Expr, ExprKind, Ident};
use vba_source::Span;

use crate::bound::*;
use crate::codes;
use crate::types::*;

use super::Binder;

impl Binder<'_, '_> {
    /// The described host class of module `m`, if it has one.
    pub(crate) fn host_class_of(&self, m: ModuleId) -> Option<LibraryType> {
        self.sema.module_hosts.get(&m).map(|h| h.ty.clone())
    }

    /// The described host class of the module being bound.
    pub(crate) fn host_class(&self) -> Option<LibraryType> {
        self.host_class_of(self.module)
    }

    /// The instance `recv` seen as its host class.
    fn as_host(&self, recv: BExpr, lt: &LibraryType) -> BExpr {
        BExpr::new(recv.span, Type::Library(lt.clone()), recv.kind)
    }

    /// `Me` of the current module as its host class.
    pub(crate) fn host_me(&self, span: Span) -> BExpr {
        let lt = self.host_class().expect("module has a host class");
        BExpr::new(span, Type::Library(lt), BExprKind::Me)
    }

    /// Whether the described host class of the current module has a member
    /// called `name`.
    pub(crate) fn host_has_member(&self, name: &Ident) -> bool {
        self.host_class().is_some_and(|lt| {
            self.sema
                .provider
                .member(&lt, &name.name.to_string_lossy())
                .is_some()
        })
    }

    /// A member of the described host class used through the instance `recv`
    /// of module `m` (`Me.Ctl`, `frm.Ctl`). A name the class does not have is
    /// an error that names the class.
    pub(crate) fn host_class_member(
        &mut self,
        recv: BExpr,
        m: ModuleId,
        lt: LibraryType,
        name: &Ident,
        args: Option<&[Arg]>,
        stmt: bool,
    ) -> BExpr {
        let span = recv.span.merge(name.span);
        let provider = self.sema.provider;
        let text = name.name.to_string_lossy();
        if provider.member(&lt, &text).is_none() && provider.describes_members(&lt) {
            self.err(
                codes::NO_SUCH_MEMBER,
                name.span,
                format!(
                    "`{}` is not a control or member of `{}` (host class `{lt}`)",
                    name.name,
                    self.sema.program.modules[m.index()].name
                ),
            );
            self.bind_args_for_errors(args);
            return self.error_expr(span);
        }
        let recv = self.as_host(recv, &lt);
        self.member_of_lib(recv, lt, name, args, stmt, span)
    }

    /// `Me!Name` and `Me("Name")` on an instance of module `m` with a
    /// described host class: a literal name must be a member of the class;
    /// any other key goes to the class's default member.
    pub(crate) fn host_class_index(
        &mut self,
        recv: BExpr,
        m: ModuleId,
        lt: LibraryType,
        args: &[Arg],
        span: Span,
    ) -> BExpr {
        if let [
            Arg {
                name: None,
                value:
                    Some(Expr {
                        kind: ExprKind::Literal(vba_ast::Literal::String(key)),
                        span: key_span,
                    }),
                ..
            },
        ] = args
        {
            let ident = Ident {
                name: vba_ast::Name::new(key),
                suffix: None,
                kind: vba_ast::IdentKind::Plain,
                span: *key_span,
            };
            return self.host_class_member(recv, m, lt, &ident, None, false);
        }
        let recv = self.as_host(recv, &lt);
        self.apply_args(recv, args, span)
    }

    pub(crate) fn bind_args_for_errors(&mut self, args: Option<&[Arg]>) {
        for a in args.unwrap_or(&[]) {
            if let Some(v) = &a.value {
                let _ = self.bind_expr(v);
            }
        }
    }

    /// The explanation appended to "not defined" errors of a module with a
    /// described host class.
    pub(crate) fn host_scope_note(&self) -> String {
        match self.host_class() {
            Some(lt) => format!(" and is not a control or member of its host class `{lt}`"),
            None => String::new(),
        }
    }
}
