//! Services for a development environment on top of a [`Session`]:
//! compiling again (hot edits, evaluation), evaluating expressions and
//! statements, and the debugger.
//!
//! None of it runs without [`Session::trust`]: compiling is harmless, but
//! everything that executes code is checked like a run request.

use std::cell::RefCell;
use std::rc::Rc;

use vba_ast::{ProcKind, Visibility};
use vba_diagnostics::{Severity, render_plain};
use vba_host::ResourceLimits;
use vba_ir::{FuncKind, IrProgram};
use vba_profile::Profile;
use vba_project::ModuleKind;
use vba_sema::{Options, SourceModule, TypeProvider};
use vba_vm::{
    BreakpointInfo, DebugHook, EVAL_ME, EVAL_MODULE, EVAL_PROC, EvalCompiler, EvalError,
    EvalOutput, EvalRequest, LimitKind, SwapError, SwapReport,
};

use crate::error::{Error, LoadError};
use crate::members::ParamSummary;
use crate::session::Session;

/// Turns sources into a verified program; kept by the session so it can
/// compile again.
pub(crate) struct Compiler {
    pub(crate) profile: Profile,
    pub(crate) provider: Rc<dyn TypeProvider>,
    pub(crate) options: Options,
    pub(crate) limits: ResourceLimits,
    /// The sources the running program was compiled from.
    pub(crate) modules: RefCell<Vec<SourceModule>>,
}

/// A compiled program with the tables the VM needs.
pub(crate) struct Compiled {
    pub(crate) ir: Rc<IrProgram>,
    pub(crate) lines: Vec<Vec<u32>>,
    pub(crate) proc_lines: Vec<(u32, u32)>,
    pub(crate) warnings: Vec<String>,
}

impl Compiler {
    /// Analyses and lowers `modules`. `friend` is `(module, target)` of an
    /// evaluation (see [`Options::friend`]).
    pub(crate) fn compile(
        &self,
        modules: &[SourceModule],
        friend: Option<(String, String)>,
    ) -> Result<Compiled, Error> {
        // The size budgets of the host's limits bound the work done before
        // anything runs: hostile source is refused with a typed limit.
        let limits = self.limits;
        let options = Options {
            limits: vba_sema::AnalysisLimits::unlimited()
                .with_max_source_bytes(limits.max_source_bytes)
                .with_max_modules(limits.max_modules)
                .with_max_nodes(limits.max_analysis_nodes),
            friend,
            ..self.options.clone()
        };
        let analysis = vba_sema::analyze_sources(modules, &self.profile, &*self.provider, &options);
        if let Some(limit) = analysis.limit_exceeded {
            use vba_sema::AnalysisLimit as L;
            return Err(Error::Limit(match limit {
                L::SourceBytes { .. } => LimitKind::SourceSize,
                L::Modules { .. } => LimitKind::ModuleCount,
                _ => LimitKind::AnalysisSize,
            }));
        }
        let render = |sev: Severity| -> Vec<String> {
            analysis
                .diagnostics
                .iter()
                .filter(|d| d.severity == sev)
                .map(|d| render_plain(d, &analysis.sources))
                .collect()
        };
        let errors = render(Severity::Error);
        if !errors.is_empty() {
            return Err(Error::Load(LoadError {
                diagnostics: errors,
            }));
        }
        let warnings = render(Severity::Warning);
        let lower_limits =
            vba_ir::LowerLimits::unlimited().with_max_instructions(limits.max_ir_instructions);
        let ir = vba_ir::lower_with_limits(&analysis, &lower_limits).map_err(|e| match e {
            vba_ir::LowerError::TooLarge { .. } => Error::Limit(LimitKind::ProgramSize),
            e => Error::Load(LoadError {
                diagnostics: vec![e.to_string()],
            }),
        })?;
        vba_ir::verify(&ir).map_err(|e| Error::Internal(format!("IR verification: {e:?}")))?;
        let (lines, proc_lines) = line_tables(&ir, &analysis.sources);
        Ok(Compiled {
            ir: Rc::new(ir),
            lines,
            proc_lines,
            warnings,
        })
    }
}

/// The 1-based source line of every statement of every function (for error
/// locations and breakpoints) and the first and last line of every function.
fn line_tables(
    ir: &IrProgram,
    sources: &vba_source::SourceMap,
) -> (Vec<Vec<u32>>, Vec<(u32, u32)>) {
    let mut lines = Vec::with_capacity(ir.functions.len());
    let mut ranges = Vec::with_capacity(ir.functions.len());
    for f in &ir.functions {
        let (module, span) = match f.kind {
            FuncKind::Proc(p) => {
                let pi = ir.program.proc(p);
                (pi.module, Some(pi.span))
            }
            FuncKind::ModuleInit(m) => (m, None),
        };
        let file = sources.get(ir.program.module(module).file);
        lines.push(
            f.stmts
                .iter()
                .map(|s| file.map_or(0, |file| file.line_col(s.span.start).line + 1))
                .collect(),
        );
        ranges.push(match (file, span) {
            (Some(file), Some(sp)) => {
                let start = file.line_col(sp.start).line + 1;
                let end = file.line_col(sp.end.saturating_sub(1).max(sp.start)).line + 1;
                (start, end)
            }
            _ => (0, 0),
        });
    }
    (lines, ranges)
}

impl EvalCompiler for Compiler {
    fn compile(&self, request: &EvalRequest) -> Result<IrProgram, Vec<String>> {
        let mut modules = self.modules.borrow().clone();
        modules.push(SourceModule {
            name: EVAL_MODULE.to_owned(),
            kind: ModuleKind::Standard,
            source: eval_source(request).into_bytes(),
        });
        let friend = request
            .module
            .clone()
            .map(|target| (EVAL_MODULE.to_owned(), target));
        match self.compile(&modules, friend) {
            Ok(c) => {
                // The VM owns the program for the duration of the call.
                Ok(Rc::try_unwrap(c.ir).unwrap_or_else(|rc| (*rc).clone()))
            }
            Err(Error::Load(l)) => Err(l.diagnostics.iter().map(|d| eval_diagnostic(d)).collect()),
            Err(e) => Err(vec![e.to_string()]),
        }
    }
}

/// The source of the evaluation module.
fn eval_source(r: &EvalRequest) -> String {
    let mut params: Vec<String> = r
        .params
        .iter()
        .map(|p| format!("ByRef {p} As Variant"))
        .collect();
    if r.me {
        params.push(format!("ByVal {EVAL_ME} As Object"));
    }
    let params = params.join(", ");
    if r.expression {
        format!(
            "Option Explicit\r\nPublic Function {EVAL_PROC}({params}) As Variant\r\n  {EVAL_PROC} = {}\r\nEnd Function\r\n",
            r.text
        )
    } else {
        format!(
            "Option Explicit\r\nPublic Sub {EVAL_PROC}({params})\r\n  {}\r\nEnd Sub\r\n",
            r.text
        )
    }
}

/// A rendered diagnostic about the evaluation module, reduced to its
/// message: the line and column of generated source mean nothing to the
/// person who typed one line.
fn eval_diagnostic(rendered: &str) -> String {
    let first = rendered.lines().next().unwrap_or(rendered);
    // `error[E0123]: message`
    match first.split_once("]: ") {
        Some((_, msg)) => msg.to_owned(),
        None => first.to_owned(),
    }
}

// ---- hot edits -----------------------------------------------------------------

/// A change to the modules of a loaded project.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModuleEdit {
    /// Replaces the source of an existing module (name case-insensitive).
    Replace {
        /// The module.
        name: String,
        /// The new source.
        source: String,
    },
    /// Adds a module after the existing ones.
    Add {
        /// The module name.
        name: String,
        /// The kind of module.
        kind: ModuleKind,
        /// The source.
        source: String,
    },
    /// Removes a module.
    Remove {
        /// The module.
        name: String,
    },
}

/// What [`Session::apply_edits`] did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct EditReport {
    /// Modules whose source was replaced with different text.
    pub replaced: Vec<String>,
    /// Modules added.
    pub added: Vec<String>,
    /// Modules removed.
    pub removed: Vec<String>,
    /// Nothing differed from the running sources: the program and the state
    /// were left alone.
    pub unchanged: bool,
    /// Warnings of the new program.
    pub warnings: Vec<String>,
    /// What happened to the state; `None` when `unchanged`.
    pub state: Option<SwapReport>,
}

/// A procedure that can be run by name (a "Run Macro" list entry).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CallableProcedure {
    /// The name to pass to `run`: `Module.Name`.
    pub qualified_name: String,
    /// The module.
    pub module: String,
    /// The procedure.
    pub name: String,
    /// `Sub` or `Function`.
    pub kind: ProcKind,
    /// The parameters in order.
    pub params: Vec<ParamSummary>,
    /// Whether the call needs at least one argument (a required, non-
    /// `ParamArray` parameter).
    pub requires_arguments: bool,
    /// Whether the bare name `Name` identifies it (no other standard module
    /// declares a callable procedure of that name).
    pub unique_bare_name: bool,
    /// The declared result type of a `Function`.
    pub return_type: Option<String>,
}

impl Session {
    /// Compiles the sources again with `edits` applied and, if that
    /// succeeds, replaces the program of the idle session.
    ///
    /// On any failure (diagnostics, a resource limit, a running procedure)
    /// the session is exactly as it was. The new program is always compiled
    /// as a whole (the analysis resolves names across modules); an edit
    /// that changes nothing is detected and costs nothing.
    ///
    /// The state policy is that of `Vm::replace_program`: when only
    /// procedure bodies changed, everything is kept; otherwise plain module
    /// variables with an unchanged declaration are kept, the rest returns
    /// to its initial value, and objects owned by the VM are dropped without
    /// `Class_Terminate`. The trust decision is kept: the new code runs
    /// with the trust given to the old.
    pub fn apply_edits(&mut self, edits: &[ModuleEdit]) -> Result<EditReport, Error> {
        if !self.vm.is_idle() {
            return Err(Error::Busy);
        }
        let mut modules = self.compiler.modules.borrow().clone();
        let mut report = EditReport::default();
        for e in edits {
            match e {
                ModuleEdit::Replace { name, source } => {
                    let m = modules
                        .iter_mut()
                        .find(|m| m.name.eq_ignore_ascii_case(name))
                        .ok_or_else(|| Error::BadEdit(format!("no module named `{name}`")))?;
                    if m.source != source.as_bytes() {
                        m.source = source.as_bytes().to_vec();
                        if !report.replaced.contains(&m.name) {
                            report.replaced.push(m.name.clone());
                        }
                    }
                }
                ModuleEdit::Add { name, kind, source } => {
                    if modules.iter().any(|m| m.name.eq_ignore_ascii_case(name)) {
                        return Err(Error::BadEdit(format!("module `{name}` already exists")));
                    }
                    if name.eq_ignore_ascii_case(EVAL_MODULE) {
                        return Err(Error::BadEdit(format!("`{name}` is a reserved name")));
                    }
                    modules.push(SourceModule {
                        name: name.clone(),
                        kind: *kind,
                        source: source.as_bytes().to_vec(),
                    });
                    report.added.push(name.clone());
                }
                ModuleEdit::Remove { name } => {
                    let at = modules
                        .iter()
                        .position(|m| m.name.eq_ignore_ascii_case(name))
                        .ok_or_else(|| Error::BadEdit(format!("no module named `{name}`")))?;
                    let m = modules.remove(at);
                    report.removed.push(m.name);
                }
            }
        }
        if report.replaced.is_empty() && report.added.is_empty() && report.removed.is_empty() {
            report.unchanged = true;
            return Ok(report);
        }
        let compiled = self.compiler.compile(&modules, None)?;
        let swap = self
            .vm
            .replace_program(compiled.ir.clone(), compiled.lines, compiled.proc_lines)
            .map_err(|e| match e {
                SwapError::Busy => Error::Busy,
                SwapError::Failed(s) => Error::Internal(s),
                _ => Error::Internal("program replacement failed".into()),
            })?;
        *self.compiler.modules.borrow_mut() = modules;
        self.ir = compiled.ir;
        self.warnings = compiled.warnings.clone();
        report.warnings = compiled.warnings;
        report.state = Some(swap);
        Ok(report)
    }

    /// Evaluates `text` in the context of `module` (`None`: only the
    /// project's public names). `?expr` returns the value of an
    /// expression; anything else runs as statements. See `Vm::eval` for
    /// scope and limits. Needs trust, like every run.
    pub fn eval(&mut self, module: Option<&str>, text: &str) -> Result<EvalOutput, Error> {
        self.require_trust()?;
        self.vm.eval(module, text).map_err(eval_error)
    }

    /// The procedures that can be run by name: public `Sub`s and
    /// `Function`s of standard modules, and of class-like modules that have
    /// a default instance, in project order. Property accessors, private
    /// procedures and procedures of plain class modules are left out, as
    /// `Application.Run` cannot reach them.
    pub fn callable_procedures(&self) -> Vec<CallableProcedure> {
        let p = &self.ir.program;
        let mut out: Vec<CallableProcedure> = Vec::new();
        for m in &p.modules {
            if m.is_class_like() && !m.predeclared {
                continue;
            }
            for id in &m.procs {
                let pr = p.proc(*id);
                if !matches!(pr.kind, ProcKind::Sub | ProcKind::Function)
                    || matches!(pr.visibility, Visibility::Private)
                {
                    continue;
                }
                let params: Vec<ParamSummary> = pr
                    .params
                    .iter()
                    .map(|q| ParamSummary {
                        name: q.name.to_string(),
                        type_name: self.vm_ref().type_name_of(&q.ty),
                        by_val: q.by_val,
                        optional: q.optional,
                        param_array: q.param_array,
                        is_array: q.is_array,
                        has_default: q.default.is_some(),
                    })
                    .collect();
                let requires_arguments = params.iter().any(|q| !q.optional && !q.param_array);
                out.push(CallableProcedure {
                    qualified_name: format!("{}.{}", m.name, pr.name),
                    module: m.name.to_string(),
                    name: pr.name.to_string(),
                    kind: pr.kind,
                    params,
                    requires_arguments,
                    unique_bare_name: false,
                    return_type: pr.has_result.then(|| self.vm_ref().type_name_of(&pr.ret)),
                });
            }
        }
        // A bare name works only for standard modules, and only when unique.
        let standard: Vec<String> = out
            .iter()
            .filter(|c| {
                p.module_by_name(&c.module)
                    .is_some_and(|m| !m.is_class_like())
            })
            .map(|c| c.name.to_ascii_lowercase())
            .collect();
        for c in &mut out {
            let is_standard = p
                .module_by_name(&c.module)
                .is_some_and(|m| !m.is_class_like());
            c.unique_bare_name = is_standard
                && standard
                    .iter()
                    .filter(|n| **n == c.name.to_ascii_lowercase())
                    .count()
                    == 1;
        }
        out
    }

    // ---- debugger ---------------------------------------------------------------

    /// Attaches (`Some`) or detaches (`None`) the pause callback; see
    /// [`vba_vm::DebugHook`].
    pub fn set_debug_hook(&mut self, hook: Option<Box<dyn DebugHook>>) {
        self.vm.set_debug_hook(hook);
    }

    /// Replaces the breakpoints; see [`Vm::set_breakpoints`](vba_vm::Vm::set_breakpoints).
    pub fn set_breakpoints(&mut self, wanted: &[(String, u32)]) -> Vec<BreakpointInfo> {
        self.vm.set_breakpoints(wanted)
    }

    /// Whether a pause callback is attached.
    pub fn debugger_attached(&self) -> bool {
        self.vm_ref().debug_hook_attached()
    }

    /// The lines of `module` a breakpoint can sit on.
    pub fn breakable_lines(&self, module: &str) -> Vec<u32> {
        self.vm_ref().breakable_lines(module)
    }
}

fn eval_error(e: EvalError) -> Error {
    match e {
        EvalError::Compile(d) => Error::Load(LoadError { diagnostics: d }),
        EvalError::Run(e) => Error::from(*e),
        EvalError::WriteBack(e) => Error::from(*e),
        EvalError::NoSuchModule(m) => Error::NoSuchProcedure(m),
        EvalError::NoSuchFrame => Error::BadEdit("no such frame".into()),
        EvalError::NoCompiler => Error::Unsupported("evaluation".into()),
        _ => Error::Internal("evaluation failed".into()),
    }
}
