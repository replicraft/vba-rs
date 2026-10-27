//! The project-level analyzer: declaration collection, signature
//! resolution, constant evaluation and declaration-level checks. Bodies are
//! bound by [`crate::bind`].

mod check;
mod collect;
mod consts;
mod libtypes;
mod scope;
mod typeres;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use vba_ast::{
    ConstItem, Declarator, DeclareDecl, EnumDecl, EventDecl, ImplementsDecl, Module as AstModule,
    Name, ProcDecl, TypeDecl,
};
use vba_diagnostics::{Code, Diagnostic, Diagnostics, Severity};
use vba_profile::Profile;
use vba_source::Span;

use crate::bound::Constant;
use crate::program::Program;
use crate::provider::TypeProvider;
use crate::types::{ModuleId, Type};

pub(crate) use scope::{Found, ModuleScope};

/// Where a constant's value comes from.
pub(crate) struct ConstSrc<'a> {
    pub module: ModuleId,
    pub item: &'a ConstItem,
}

#[derive(Debug, Clone)]
pub(crate) enum CState {
    Pending,
    Running,
    Done(Constant, Type),
    Failed,
}

/// The analyzer state shared by all passes.
pub(crate) struct Sema<'a> {
    pub asts: &'a [AstModule],
    pub program: Program,
    pub diags: RefCell<Diagnostics>,
    pub provider: &'a dyn TypeProvider,
    pub project_name: Name,
    /// Lower-case names the host supplies to document and designer modules.
    pub host_members: std::collections::HashSet<String>,
    /// Spec-strict diagnostics are errors.
    pub strict_spec: bool,
    pub module_ids: HashMap<Name, ModuleId>,
    pub proc_src: Vec<&'a ProcDecl>,
    pub global_src: Vec<Option<&'a Declarator>>,
    pub const_src: Vec<ConstSrc<'a>>,
    pub const_state: RefCell<Vec<CState>>,
    pub enum_src: Vec<&'a EnumDecl>,
    pub enum_state: RefCell<Vec<Vec<CState>>>,
    /// The enum member (or `None` for a constant) whose value is being
    /// evaluated, innermost last.
    /// Module pairs (user, provider) linked by a constant or enum member
    /// expression that names a member of another module.
    pub module_deps: RefCell<std::collections::BTreeSet<(u32, u32)>>,
    pub eval_ctx: RefCell<Vec<Option<(crate::types::EnumId, usize)>>>,
    pub udt_src: Vec<&'a TypeDecl>,
    pub declare_src: Vec<&'a DeclareDecl>,
    pub event_src: Vec<&'a EventDecl>,
    pub implements_src: Vec<Vec<&'a ImplementsDecl>>,
    pub statics: HashMap<(crate::types::ProcId, Name), crate::types::GlobalId>,
    /// Library enums and records met while analysing (see `libtypes`).
    pub lib_types: RefCell<libtypes::LibTypes>,
    /// Bound statements and expressions allowed in all (`None`: no limit).
    pub node_limit: Option<u64>,
    /// Bound statements and expressions so far.
    pub nodes: Cell<u64>,
    /// Set once `nodes` passed `node_limit`; binding then stops descending.
    pub over_nodes: Cell<bool>,
}

impl<'a> Sema<'a> {
    pub(crate) fn new(
        asts: &'a [AstModule],
        program: Program,
        provider: &'a dyn TypeProvider,
        project_name: &str,
    ) -> Self {
        Sema {
            asts,
            program,
            diags: RefCell::new(Diagnostics::new()),
            provider,
            project_name: Name::new(project_name),
            host_members: std::collections::HashSet::new(),
            strict_spec: false,
            module_ids: HashMap::new(),
            proc_src: Vec::new(),
            global_src: Vec::new(),
            const_src: Vec::new(),
            const_state: RefCell::new(Vec::new()),
            enum_src: Vec::new(),
            enum_state: RefCell::new(Vec::new()),
            module_deps: RefCell::new(std::collections::BTreeSet::new()),
            eval_ctx: RefCell::new(Vec::new()),
            udt_src: Vec::new(),
            declare_src: Vec::new(),
            event_src: Vec::new(),
            implements_src: Vec::new(),
            statics: HashMap::new(),
            lib_types: RefCell::default(),
            node_limit: None,
            nodes: Cell::new(0),
            over_nodes: Cell::new(false),
        }
    }

    /// Counts one bound statement or expression. True when the node budget
    /// is spent: the caller must not descend further, so that a hostile
    /// program costs a bounded amount of work and memory.
    pub(crate) fn charge_node(&self) -> bool {
        if self.over_nodes.get() {
            return true;
        }
        let n = self.nodes.get() + 1;
        self.nodes.set(n);
        if self.node_limit.is_some_and(|l| n > l) {
            self.over_nodes.set(true);
            return true;
        }
        false
    }

    pub(crate) fn profile(&self) -> &Profile {
        &self.program.profile
    }

    /// Reports a diagnostic in `module`'s file.
    pub(crate) fn report(
        &self,
        severity: Severity,
        module: ModuleId,
        code: Code,
        span: Span,
        msg: impl Into<String>,
    ) {
        let msg: String = msg.into();
        if msg.is_empty() {
            return; // already reported where the problem originated
        }
        let file = self.program.modules[module.index()].file;
        self.diags
            .borrow_mut()
            .push(Diagnostic::new(severity, code, file, span, msg));
    }

    /// Reports an error.
    pub(crate) fn error(&self, module: ModuleId, code: Code, span: Span, msg: impl Into<String>) {
        self.report(Severity::Error, module, code, span, msg);
    }

    /// Reports a spec-strict diagnostic: a warning unless `strict_spec`.
    pub(crate) fn spec_strict(
        &self,
        module: ModuleId,
        code: Code,
        span: Span,
        msg: impl Into<String>,
    ) {
        self.report(self.spec_severity(), module, code, span, msg);
    }

    /// The severity of spec-strict diagnostics.
    pub(crate) fn spec_severity(&self) -> Severity {
        if self.strict_spec {
            Severity::Error
        } else {
            Severity::Warning
        }
    }

    /// Reports a diagnostic with a secondary label.
    pub(crate) fn error_with_label(
        &self,
        module: ModuleId,
        code: Code,
        span: Span,
        msg: impl Into<String>,
        label_span: Span,
        label: impl Into<String>,
    ) {
        let file = self.program.modules[module.index()].file;
        self.diags
            .borrow_mut()
            .push(Diagnostic::error(code, file, span, msg).with_label(label_span, label));
    }

    /// Like [`Self::error_with_label`] for a spec-strict diagnostic.
    pub(crate) fn spec_strict_with_label(
        &self,
        module: ModuleId,
        code: Code,
        span: Span,
        msg: impl Into<String>,
        label_span: Span,
        label: impl Into<String>,
    ) {
        let file = self.program.modules[module.index()].file;
        self.diags.borrow_mut().push(
            Diagnostic::new(self.spec_severity(), code, file, span, msg)
                .with_label(label_span, label),
        );
    }

    pub(crate) fn enum_id_count(&self) -> usize {
        self.program.enums.len()
    }

    pub(crate) fn const_count(&self) -> usize {
        self.program.consts.len()
    }
}

/// Runs the declaration passes (everything except bodies).
pub(crate) fn run_declaration_passes(sema: &mut Sema<'_>) {
    sema.collect();
    sema.resolve_signatures();
    sema.hoist_statics();
    sema.check_declarations();
}
