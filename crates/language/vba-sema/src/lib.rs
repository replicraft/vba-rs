//! Semantic analysis for VBA projects.
//!
//! [`analyze_sources`] / [`analyze_project`] parse the modules of a project
//! (through `vba-ast`), build module and project symbol tables, resolve
//! names case-insensitively following MS-VBAL §5.6, check
//! declarations (`Option Explicit`, properties, `Implements`, events,
//! `Declare`), fold constants, type every expression and bind every call.
//! The result is an [`Analysis`]: a [`Program`] whose procedure bodies are
//! typed [`bound`] trees, plus deterministic diagnostics with `SEM###`
//! codes.
//!
//! Library types (host object models, COM libraries) come from an injected
//! [`TypeProvider`]; anything the provider cannot answer is reported
//! explicitly, never guessed. See `docs/language/sema.md`.

pub mod bound;
pub mod builtins;
pub mod codes;
pub mod fold;
pub mod program;
pub mod provider;
pub mod types;
pub mod walk;

mod bind;
mod sema;

pub use program::{
    Analysis, ConstInfo, DeclareInfo, EnumInfo, EventHandler, EventInfo, EventRef, FieldInfo,
    GlobalInfo, ImplementsInfo, LabelInfo, LabelKey, LibEnumInfo, LibRecordInfo, LocalInfo,
    LocalKind, ModuleInfo, ModuleOptions, ParamInfo, ProcInfo, Program, PropGroup, Symbol, UdtInfo,
};
pub use provider::{
    Chain, Layers, LibClass, LibEntry, LibEntryKind, LibEvent, LibField, LibGlobal, LibMember,
    LibMemberKind, LibParam, LibRecord, NoLibraries, StaticLibrary, TypeProvider,
};
pub use types::*;

use std::collections::HashMap;

use vba_ast::{Name, parse_module};
use vba_diagnostics::{Diagnostic, Diagnostics};
use vba_profile::Profile;
use vba_project::{ModuleKind, VbaProject};
use vba_source::{FileId, SourceEncoding, SourceMap, Span};
use vba_syntax::ParseOptions;

/// One module to analyse.
#[derive(Debug, Clone)]
pub struct SourceModule {
    /// Module name.
    pub name: String,
    /// Module role.
    pub kind: ModuleKind,
    /// Source text as stored (without an export file header).
    pub source: Vec<u8>,
}

/// Size budgets of one analysis. `None` means unlimited.
///
/// They bound the work and memory a hostile project can cost before it
/// runs: the bytes of source (checked before anything is parsed), the
/// number of modules (before parsing too) and the number of statements and
/// expressions that get bound (checked while binding, which stops descending
/// as soon as the budget is spent). Exceeding one yields an
/// [`AnalysisLimit`] in [`Analysis::limit_exceeded`] and an error diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct AnalysisLimits {
    /// Total bytes of source over all modules.
    pub max_source_bytes: Option<u64>,
    /// Number of modules.
    pub max_modules: Option<u64>,
    /// Bound statements and expressions over the whole project.
    pub max_nodes: Option<u64>,
}

impl AnalysisLimits {
    /// No budget at all.
    pub const fn unlimited() -> Self {
        AnalysisLimits {
            max_source_bytes: None,
            max_modules: None,
            max_nodes: None,
        }
    }

    /// Sets [`max_source_bytes`](AnalysisLimits::max_source_bytes).
    #[must_use]
    pub const fn with_max_source_bytes(mut self, n: Option<u64>) -> Self {
        self.max_source_bytes = n;
        self
    }

    /// Sets [`max_modules`](AnalysisLimits::max_modules).
    #[must_use]
    pub const fn with_max_modules(mut self, n: Option<u64>) -> Self {
        self.max_modules = n;
        self
    }

    /// Sets [`max_nodes`](AnalysisLimits::max_nodes).
    #[must_use]
    pub const fn with_max_nodes(mut self, n: Option<u64>) -> Self {
        self.max_nodes = n;
        self
    }
}

/// Which analysis budget a project went over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalysisLimit {
    /// More bytes of source than [`AnalysisLimits::max_source_bytes`].
    SourceBytes {
        /// The budget.
        limit: u64,
        /// What the project has.
        actual: u64,
    },
    /// More modules than [`AnalysisLimits::max_modules`].
    Modules {
        /// The budget.
        limit: u64,
        /// What the project has.
        actual: u64,
    },
    /// More statements and expressions than [`AnalysisLimits::max_nodes`].
    Nodes {
        /// The budget.
        limit: u64,
    },
}

impl std::fmt::Display for AnalysisLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisLimit::SourceBytes { limit, actual } => write!(
                f,
                "the project has {actual} bytes of source; the limit is {limit}"
            ),
            AnalysisLimit::Modules { limit, actual } => {
                write!(f, "the project has {actual} modules; the limit is {limit}")
            }
            AnalysisLimit::Nodes { limit } => write!(
                f,
                "the project has more than {limit} statements and expressions; \
                 analysis was stopped"
            ),
        }
    }
}

/// Analysis options.
#[derive(Debug, Clone)]
pub struct Options {
    /// Report the deviations VBE tolerates as errors (see `vba-syntax`).
    pub strict_syntax: bool,
    /// Project name, for `Project.Module.Name` qualification.
    pub project_name: String,
    /// Project-level conditional compilation constants.
    pub constants: Vec<(String, i64)>,
    /// Names the embedding application provides as members of document and
    /// designer modules (controls, base-class properties). An unqualified
    /// use of one of these names, read or assigned, in such a module is
    /// bound late to the host base object of the instance, even when it
    /// would otherwise be an undeclared variable under `Option Explicit`.
    /// Matched case-insensitively. Names not listed keep the normal rules
    /// (a call, `Name(args)`, is always bound late).
    pub host_members: Vec<String>,
    /// Report the spec-strict diagnostics as errors. These are rules added
    /// from the specification text where real-world acceptance is uncertain
    /// (see `docs/language/spec-coverage.md`); by default they are warnings
    /// and do not block loading.
    pub strict_spec: bool,
    /// Size budgets; unlimited by default.
    pub limits: AnalysisLimits,
    /// `(friend, target)` module names. Unqualified names used in module
    /// `friend` also resolve among the declarations of module `target`,
    /// private ones included, before the project's public names. Meant for
    /// code evaluated in the context of a module (an immediate window):
    /// the declarations visible are the constants, enums and types of any
    /// target, and additionally the variables, procedures and declarations
    /// of a standard-module target. A class-like target contributes only
    /// constants, enums and types, because its other members need an
    /// instance.
    pub friend: Option<(String, String)>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            strict_syntax: true,
            project_name: "VBAProject".to_owned(),
            constants: Vec::new(),
            host_members: Vec::new(),
            strict_spec: false,
            limits: AnalysisLimits::unlimited(),
            friend: None,
        }
    }
}

/// Parses `a = 1 : b = 2` style project constants; malformed entries are
/// ignored.
pub fn parse_project_constants(text: &str) -> Vec<(String, i64)> {
    text.split(':')
        .filter_map(|part| {
            let (k, v) = part.split_once('=')?;
            Some((k.trim().to_owned(), v.trim().parse::<i64>().ok()?))
        })
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

/// Analyses the modules of a project.
pub fn analyze_project(
    project: &VbaProject,
    profile: &Profile,
    provider: &dyn TypeProvider,
) -> Analysis {
    analyze_project_with_host_members(project, profile, provider, &[])
}

/// Like [`analyze_project`], with the names the embedder provides as members
/// of document and designer modules (see [`Options::host_members`]).
pub fn analyze_project_with_host_members(
    project: &VbaProject,
    profile: &Profile,
    provider: &dyn TypeProvider,
    host_members: &[String],
) -> Analysis {
    analyze_project_with_options(project, profile, provider, host_members, false)
}

/// Like [`analyze_project_with_host_members`], with the
/// [`Options::strict_spec`] switch.
pub fn analyze_project_with_options(
    project: &VbaProject,
    profile: &Profile,
    provider: &dyn TypeProvider,
    host_members: &[String],
    strict_spec: bool,
) -> Analysis {
    let modules: Vec<SourceModule> = project
        .modules
        .iter()
        .map(|m| SourceModule {
            name: m.name.clone(),
            kind: m.kind,
            source: m.source.clone().into_bytes(),
        })
        .collect();
    let opts = Options {
        project_name: project.name.clone(),
        constants: parse_project_constants(&project.constants),
        host_members: host_members.to_vec(),
        strict_spec,
        ..Options::default()
    };
    analyze_sources(&modules, profile, provider, &opts)
}

/// Analyses a set of modules.
pub fn analyze_sources(
    modules: &[SourceModule],
    profile: &Profile,
    provider: &dyn TypeProvider,
    opts: &Options,
) -> Analysis {
    let mut sources = SourceMap::new();
    if let Some(limit) = check_input_size(modules, &opts.limits) {
        return refused(sources, limit, *profile);
    }
    let mut parse_opts = if opts.strict_syntax {
        ParseOptions::strict()
    } else {
        ParseOptions::tolerant()
    };
    parse_opts.constants = opts.constants.clone();
    parse_opts.strict_spec = opts.strict_spec;
    let mut all = Diagnostics::new();
    let mut asts = Vec::new();
    let mut infos = Vec::new();
    let mut ids: HashMap<Name, ModuleId> = HashMap::new();
    for (i, m) in modules.iter().enumerate() {
        let file = sources.add(
            m.name.clone(),
            m.source.clone(),
            if std::str::from_utf8(&m.source).is_ok() {
                SourceEncoding::Utf8
            } else {
                SourceEncoding::Unknown
            },
        );
        let parsed = parse_module(file, &m.source, profile, &parse_opts);
        let has_errors = parsed
            .diagnostics
            .iter()
            .any(|d| d.severity == vba_diagnostics::Severity::Error);
        all.extend(parsed.diagnostics);
        let name = Name::new(&m.name);
        let id = ModuleId(i as u32);
        if let Some(prev) = ids.get(&name) {
            all.push(Diagnostic::error(
                codes::DUPLICATE_DECLARATION,
                file,
                Span::empty(0),
                format!(
                    "module `{}` is defined twice (first module #{})",
                    m.name, prev.0
                ),
            ));
        } else {
            ids.insert(name.clone(), id);
        }
        infos.push(ModuleInfo {
            id,
            name,
            kind: m.kind,
            file,
            options: ModuleOptions::default(),
            symbols: HashMap::new(),
            procs: Vec::new(),
            globals: Vec::new(),
            consts: Vec::new(),
            udts: Vec::new(),
            enums: Vec::new(),
            declares: Vec::new(),
            events: Vec::new(),
            implements: Vec::new(),
            handlers: Vec::new(),
            predeclared: false,
            default_member: None,
            enumerator_member: None,
            initialize: None,
            terminate: None,
            has_syntax_errors: has_errors,
        });
        asts.push(parsed.module);
    }
    let program = Program {
        profile: *profile,
        modules: infos,
        procs: Vec::new(),
        globals: Vec::new(),
        consts: Vec::new(),
        udts: Vec::new(),
        enums: Vec::new(),
        declares: Vec::new(),
        events: Vec::new(),
        lib_enums: Vec::new(),
        lib_records: Vec::new(),
    };
    let builtin = builtins::BuiltinLibrary;
    let chained = Chain(provider, &builtin);
    let mut s = sema::Sema::new(&asts, program, &chained, &opts.project_name);
    s.host_members = opts
        .host_members
        .iter()
        .map(|n| n.to_ascii_lowercase())
        .collect();
    s.strict_spec = opts.strict_spec;
    s.friend = opts.friend.as_ref().and_then(|(f, t)| {
        Some((*ids.get(&Name::new(f))?, *ids.get(&Name::new(t))?))
    });
    s.module_ids = ids;
    s.node_limit = opts.limits.max_nodes;
    sema::run_declaration_passes(&mut s);
    let mut bound = Vec::new();
    for i in 0..s.program.procs.len() {
        bound.push(bind::bind_proc(&s, ProcId(i as u32)));
        if s.over_nodes.get() {
            break;
        }
    }
    for (i, b) in bound.into_iter().enumerate() {
        let p = &mut s.program.procs[i];
        p.locals = b.locals;
        p.result_local = b.result_local;
        p.labels = b.labels;
        p.body = b.body;
    }
    s.finish_library_types();
    let mut limit_exceeded = None;
    if s.over_nodes.get() {
        let limit = AnalysisLimit::Nodes {
            limit: opts.limits.max_nodes.unwrap_or(0),
        };
        limit_exceeded = Some(limit);
        if let Some(m) = s.program.modules.first() {
            all.push(Diagnostic::error(
                codes::LIMIT_EXCEEDED,
                m.file,
                Span::empty(0),
                limit.to_string(),
            ));
        }
    }
    all.extend(s.diags.into_inner().as_slice().iter().cloned());
    Analysis {
        program: s.program,
        diagnostics: all.into_sorted(),
        sources,
        limit_exceeded,
    }
}

/// The budget that the unparsed input already exceeds, if any.
fn check_input_size(modules: &[SourceModule], limits: &AnalysisLimits) -> Option<AnalysisLimit> {
    if let Some(limit) = limits.max_modules {
        let actual = modules.len() as u64;
        if actual > limit {
            return Some(AnalysisLimit::Modules { limit, actual });
        }
    }
    if let Some(limit) = limits.max_source_bytes {
        let actual: u64 = modules.iter().map(|m| m.source.len() as u64).sum();
        if actual > limit {
            return Some(AnalysisLimit::SourceBytes { limit, actual });
        }
    }
    None
}

/// The analysis of a project that was refused before anything was parsed:
/// no modules, one error.
fn refused(mut sources: SourceMap, limit: AnalysisLimit, profile: Profile) -> Analysis {
    let file = sources.add("(project)".to_owned(), Vec::new(), SourceEncoding::Utf8);
    let diagnostics = vec![Diagnostic::error(
        codes::LIMIT_EXCEEDED,
        file,
        Span::empty(0),
        limit.to_string(),
    )];
    Analysis {
        program: Program {
            profile,
            modules: Vec::new(),
            procs: Vec::new(),
            globals: Vec::new(),
            consts: Vec::new(),
            udts: Vec::new(),
            enums: Vec::new(),
            declares: Vec::new(),
            events: Vec::new(),
            lib_enums: Vec::new(),
            lib_records: Vec::new(),
        },
        diagnostics,
        sources,
        limit_exceeded: Some(limit),
    }
}

/// The file id of module `m` in an [`Analysis`].
pub fn file_of(a: &Analysis, m: ModuleId) -> FileId {
    a.program.modules[m.index()].file
}
