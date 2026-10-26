//! Module-level state and metadata: public variables and constants,
//! and a description of the modules, procedures and variables of a project
//! for hosts that discover event handlers.

use vba_ast::{ProcKind, Visibility};
use vba_project::ModuleKind;

use crate::session::Session;

/// One parameter of a procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParamSummary {
    /// The parameter name.
    pub name: String,
    /// The declared type as written in VBA (`Long`, `String`, `Variant`,
    /// a class name...).
    pub type_name: String,
    /// Declared `ByVal`. Otherwise it is `ByRef` (the VBA default) and the
    /// final value can be read back with a `ByRef` argument.
    pub by_val: bool,
    /// `Optional`.
    pub optional: bool,
    /// `ParamArray`.
    pub param_array: bool,
    /// Declared with `()`.
    pub is_array: bool,
    /// An `Optional` parameter with a declared default value.
    pub has_default: bool,
}

/// One procedure (`Sub`, `Function` or a property accessor).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProcedureSummary {
    /// The procedure name.
    pub name: String,
    /// `Sub`, `Function`, `Property Get`, `Let` or `Set`.
    pub kind: ProcKind,
    /// `Public` or `Friend` (callable by name); event handlers such as
    /// `Form_Current` are usually private.
    pub public: bool,
    /// The parameters in order.
    pub params: Vec<ParamSummary>,
    /// The declared result type of a `Function` or `Property Get`.
    pub return_type: Option<String>,
}

/// A module-level variable or constant.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VariableSummary {
    /// The name.
    pub name: String,
    /// The declared type as written in VBA.
    pub type_name: String,
    /// Declared `Public`.
    pub public: bool,
    /// A `Const`.
    pub constant: bool,
    /// Declared `WithEvents`.
    pub with_events: bool,
}

/// A module of the project and what it declares.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModuleSummary {
    /// The module name.
    pub name: String,
    /// Standard, class, document or designer module.
    pub kind: ModuleKind,
    /// Whether the module has instances (everything but standard modules).
    pub class_like: bool,
    /// Whether a default instance exists (`Form_X` / `Forms!X` style).
    pub predeclared: bool,
    /// The class default member, if declared.
    pub default_member: Option<String>,
    /// The procedures in source order.
    pub procedures: Vec<ProcedureSummary>,
    /// The variables and constants in source order. In a class-like module
    /// they are fields of each instance, in a standard module they are
    /// project-wide state.
    pub variables: Vec<VariableSummary>,
}

impl ModuleSummary {
    /// The procedure called `name` (case-insensitive), if the module
    /// defines it.
    pub fn procedure(&self, name: &str) -> Option<&ProcedureSummary> {
        self.procedures
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
    }
}

impl Session {
    /// Describes every module, procedure and module-level variable of the
    /// project, whatever its visibility (an event handler such as
    /// `Form_Current` is private, and a host still wants to find it).
    pub fn modules(&self) -> Vec<ModuleSummary> {
        let p = &self.program().program;
        p.modules
            .iter()
            .map(|m| ModuleSummary {
                name: m.name.to_string(),
                kind: m.kind,
                class_like: m.is_class_like(),
                predeclared: m.predeclared,
                default_member: m.default_member.as_ref().map(|n| n.to_string()),
                procedures: m
                    .procs
                    .iter()
                    .map(|id| {
                        let pr = p.proc(*id);
                        ProcedureSummary {
                            name: pr.name.to_string(),
                            kind: pr.kind,
                            public: !matches!(pr.visibility, Visibility::Private),
                            params: pr
                                .params
                                .iter()
                                .map(|q| ParamSummary {
                                    name: q.name.to_string(),
                                    type_name: q.ty.to_string(),
                                    by_val: q.by_val,
                                    optional: q.optional,
                                    param_array: q.param_array,
                                    is_array: q.is_array,
                                    has_default: q.default.is_some(),
                                })
                                .collect(),
                            return_type: pr.has_result.then(|| pr.ret.to_string()),
                        }
                    })
                    .collect(),
                variables: {
                    let mut v = Vec::new();
                    for g in &m.globals {
                        let g = &p.globals[g.index()];
                        if g.static_of.is_some() {
                            continue;
                        }
                        v.push((
                            g.span,
                            VariableSummary {
                                name: g.name.to_string(),
                                type_name: g.ty.to_string(),
                                public: !matches!(g.visibility, Visibility::Private),
                                constant: false,
                                with_events: g.with_events,
                            },
                        ));
                    }
                    for c in &m.consts {
                        let c = &p.consts[c.index()];
                        v.push((
                            c.span,
                            VariableSummary {
                                name: c.name.to_string(),
                                type_name: c.ty.to_string(),
                                public: !matches!(c.visibility, Visibility::Private),
                                constant: true,
                                with_events: false,
                            },
                        ));
                    }
                    v.sort_by_key(|(span, _)| span.start);
                    v.into_iter().map(|(_, s)| s).collect()
                },
            })
            .collect()
    }

    /// The public surface of the project: like [`modules`](Session::modules)
    /// without the private procedures and variables. Modules stay listed
    /// even when nothing in them is public.
    pub fn list_public_members(&self) -> Vec<ModuleSummary> {
        let mut modules = self.modules();
        for m in &mut modules {
            m.procedures.retain(|p| p.public);
            m.variables.retain(|v| v.public);
        }
        modules
    }

    /// The class-like modules that define a procedure called `name`, of any
    /// visibility: which forms define `Form_Current`.
    pub fn modules_defining(&self, name: &str) -> Vec<String> {
        self.modules()
            .into_iter()
            .filter(|m| m.class_like && m.procedure(name).is_some())
            .map(|m| m.name)
            .collect()
    }
}
