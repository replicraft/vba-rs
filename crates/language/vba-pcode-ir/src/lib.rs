//! Lowering of compiled-code ("p-code") procedures to `vba-ir`.
//!
//! The instruction streams decoded by `vba-pcode` are lowered, procedure by
//! procedure, to the basic-block IR that `vba-vm` executes. The subset is
//! deliberately conservative: a procedure is lowered only when every
//! instruction it contains has [`vba_pcode::Evidence::Verified`] semantics
//! (plus comments and line-layout markers, which have no effect) and every
//! construct it uses is in the list below. Everything else makes the
//! procedure [`Outcome::Unsupported`] with the opcode, procedure and line at
//! which lowering stopped; nothing is ever guessed.
//!
//! Covered: parameters and locals of plain scalar types, literals,
//! arithmetic, comparison, logic and concatenation operators, assignment,
//! block `If`/`ElseIf`/`Else`, `For`/`Next` (with `Step`), `Do`/`Loop` in all
//! forms, `While`/`Wend`, `Select Case` with values and ranges, `Exit`
//! statements, calls of other procedures of the project and of library
//! functions by resolved name, and `Debug.Print`.
//!
//! The IR refers to a semantic program (procedure ids, signatures, local
//! slots). It is derived from a declaration-only text that this crate
//! generates from the *cache records* (headers, parameter and local
//! declarations); no original source text is read.
//!
//! Only VBA 7.0 x86 caches (the verified layout) are accepted.

mod error;
mod lower;
mod model;
mod shell;

use std::collections::HashMap;

use vba_ir::{Block, Function, IrProgram, StmtId, StmtInfo, Terminator};
use vba_pcode::{ModuleCache, ProjectCache, Verification};
use vba_project::ModuleKind;
use vba_sema::ProcId;
use vba_source::Span;

pub use error::{LowerError, Outcome, Unsupported};

/// One module cache to lower.
#[derive(Debug, Clone, Copy)]
pub struct ModuleInput<'a> {
    /// Module name.
    pub name: &'a str,
    /// The role of the module. Class, document and designer modules are
    /// lowered as members of instances; the members of a document or
    /// designer module that the module does not declare (controls, base
    /// class members) are late bound on the host base.
    pub kind: ModuleKind,
    /// The performance cache (module stream bytes before the text offset).
    pub cache: &'a [u8],
}

/// What happened to one procedure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedureReport {
    /// Module name.
    pub module: String,
    /// Procedure name.
    pub procedure: String,
    /// The id in the produced program, when the procedure has a signature
    /// there.
    pub id: Option<ProcId>,
    /// Line indexes (in the module cache) of the header and of the end
    /// instruction.
    pub first_line: usize,
    /// See [`ProcedureReport::first_line`].
    pub last_line: usize,
    /// The kind recorded for the procedure.
    pub kind: vba_pcode::ProcKind,
    /// Lowered or not.
    pub outcome: Outcome,
}

/// A module-level variable or constant found in a cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationReport {
    /// Module name.
    pub module: String,
    /// Name of the variable or constant.
    pub name: String,
    /// Line index in the module cache.
    pub line: usize,
    /// A constant (otherwise a variable).
    pub is_const: bool,
    /// Why it is not part of the program, `None` when it is.
    pub unsupported: Option<String>,
}

/// The result of lowering a project.
#[derive(Debug, Clone)]
pub struct LoweredProject {
    /// The program. Procedures that are not lowered have a body that
    /// raises an error when executed; they must not be run.
    pub ir: IrProgram,
    /// One entry per procedure found in the caches.
    pub procedures: Vec<ProcedureReport>,
    /// One entry per module-level variable or constant found in the caches.
    pub declarations: Vec<DeclarationReport>,
}

impl LoweredProject {
    /// Number of procedures that were lowered.
    pub fn lowered_count(&self) -> usize {
        self.procedures
            .iter()
            .filter(|p| p.outcome.is_lowered())
            .count()
    }

    /// The report of a procedure.
    pub fn report(&self, module: &str, procedure: &str) -> Option<&ProcedureReport> {
        self.procedures.iter().find(|p| {
            p.module.eq_ignore_ascii_case(module) && p.procedure.eq_ignore_ascii_case(procedure)
        })
    }
}

fn trap_function(template: &Function) -> Function {
    Function {
        kind: template.kind,
        name: template.name.clone(),
        slots: template.slots.clone(),
        regs: Vec::new(),
        blocks: vec![Block {
            insts: Vec::new(),
            term: Terminator::Unreachable,
            term_stmt: StmtId(0),
            term_span: Span::empty(0),
        }],
        stmts: vec![StmtInfo {
            span: Span::empty(0),
            line: None,
            resume_next: None,
        }],
    }
}

/// Lowers the procedures of the given module caches.
///
/// `project_stream` is the `_VBA_PROJECT` stream and `sys_kind` the
/// `PROJECTSYSKIND` value, as for [`ProjectCache::parse`].
///
/// # Errors
///
/// [`LowerError`] when the project stream cannot be decoded, the layout is
/// not the verified one, or the program shell cannot be built. Problems
/// inside single procedures are reported per procedure instead.
pub fn lower_project(
    project_stream: &[u8],
    sys_kind: u32,
    modules: &[ModuleInput<'_>],
) -> Result<LoweredProject, LowerError> {
    lower_project_with_options(project_stream, sys_kind, modules, &LowerOptions::default())
}

/// Settings of the embedding application that the compiled code does not
/// carry.
#[derive(Debug, Clone, Default)]
pub struct LowerOptions {
    /// Names the application provides as members of document and designer
    /// modules (controls, base-class properties); the same list the source
    /// analysis takes ([`vba_sema::Options::host_members`]). A bare read of
    /// one of these names, and every call, statement or member access on an
    /// unresolved name, is bound late on the instance's host base object.
    pub host_members: Vec<String>,
}

/// Like [`lower_project`], with the settings of the embedding application.
///
/// # Errors
///
/// As for [`lower_project`].
pub fn lower_project_with_options(
    project_stream: &[u8],
    sys_kind: u32,
    modules: &[ModuleInput<'_>],
    options: &LowerOptions,
) -> Result<LoweredProject, LowerError> {
    let host_members: std::collections::HashSet<String> = options
        .host_members
        .iter()
        .map(|n| n.to_ascii_lowercase())
        .collect();
    let pc = ProjectCache::parse(project_stream, sys_kind)?;
    if pc.layout.verification() != Verification::Verified {
        return Err(LowerError::Shell(
            "only the verified layout (VBA 7.0, 32-bit) is lowered".into(),
        ));
    }
    let mut models = Vec::new();
    let mut reports: Vec<Vec<Option<Outcome>>> = Vec::new();
    for m in modules {
        match ModuleCache::parse(m.cache, &pc.layout) {
            Ok(cache) => models.push(model::extract_module(m.name, m.kind, &cache, &pc)),
            Err(_) => models.push(model::ModuleModel {
                name: m.name.to_owned(),
                kind: m.kind,
                compare: model::CompareStored::Binary,
                explicit: false,
                decls: Vec::new(),
                procs: Vec::new(),
            }),
        }
    }
    for m in &models {
        reports.push(vec![None; m.procs.len()]);
    }
    // Eligibility from the signature alone.
    let mut include: Vec<Vec<bool>> = Vec::new();
    for (mi, m) in models.iter().enumerate() {
        let mut row = Vec::new();
        for (pi, p) in m.procs.iter().enumerate() {
            if let Some(u) = &p.signature_problem {
                reports[mi][pi] = Some(Outcome::Unsupported(u.clone()));
                row.push(false);
            } else {
                row.push(true);
            }
        }
        include.push(row);
    }
    let before = include.clone();
    let shell = shell::build_shell(&models, &mut include)?;
    for (mi, m) in models.iter().enumerate() {
        for (pi, p) in m.procs.iter().enumerate() {
            if before[mi][pi] && !include[mi][pi] {
                reports[mi][pi] = Some(Outcome::Unsupported(Unsupported {
                    opcode: vba_pcode::Opcode::ProcHeader,
                    procedure: p.name.clone(),
                    line: p.header_line,
                    reason: "the declarations generated for it were rejected".into(),
                }));
            }
        }
    }
    let mut ir = shell.ir;
    // Name index of callable procedures.
    let mut index = lower::ProcIndex::default();
    let mut by_id: HashMap<ProcId, (usize, usize)> = HashMap::new();
    for m in &models {
        index
            .known
            .extend(m.procs.iter().map(|p| p.name.to_ascii_lowercase()));
        index
            .known
            .extend(m.decls.iter().map(|d| d.name.to_ascii_lowercase()));
    }
    for (mi, m) in models.iter().enumerate() {
        for (pi, p) in m.procs.iter().enumerate() {
            if let Some(id) = shell.proc_ids[mi][pi] {
                index
                    .by_name
                    .entry(p.name.to_ascii_lowercase())
                    .or_default()
                    .push((mi, pi, id));
                by_id.insert(id, (mi, pi));
            }
        }
    }
    let class_like: Vec<bool> = models.iter().map(model::ModuleModel::class_like).collect();
    // Lower every included procedure.
    let mut lowered: HashMap<(usize, usize), lower::Lowered> = HashMap::new();
    for (mi, m) in models.iter().enumerate() {
        for (pi, p) in m.procs.iter().enumerate() {
            let Some(id) = shell.proc_ids[mi][pi] else {
                if reports[mi][pi].is_none() && include[mi][pi] {
                    reports[mi][pi] = Some(Outcome::Unsupported(Unsupported {
                        opcode: vba_pcode::Opcode::ProcHeader,
                        procedure: p.name.clone(),
                        line: p.header_line,
                        reason: "no procedure of that name in the program shell".into(),
                    }));
                }
                continue;
            };
            let cx = lower::ProcCtx {
                program: &ir.program,
                proc_id: id,
                index: &index,
                module: mi,
                class_like: &class_like,
                host_backed: matches!(m.kind, ModuleKind::Designer | ModuleKind::Document),
                host_members: &host_members,
                compare: m.compare,
                name: &p.name,
            };
            match lower::lower_body(&cx, &p.body, p.header_line, ir.function_of(id)) {
                Ok(l) => {
                    lowered.insert((mi, pi), l);
                }
                Err(u) => reports[mi][pi] = Some(Outcome::Unsupported(u)),
            }
        }
    }
    // A procedure that calls a procedure that is not lowered is not lowered.
    loop {
        let mut dropped = Vec::new();
        for (&(mi, pi), l) in &lowered {
            for c in &l.callees {
                let ok = by_id.get(c).is_some_and(|k| lowered.contains_key(k));
                if !ok {
                    dropped.push((mi, pi, *c));
                    break;
                }
            }
        }
        // The run time runs `Class_Initialize` and `Class_Terminate` when it
        // creates and destroys an instance: a module whose one is not
        // lowered cannot be instantiated faithfully, so none of its
        // procedures is offered.
        let mut blocked: Vec<(usize, usize, String)> = Vec::new();
        for (mi, m) in models.iter().enumerate() {
            if !m.class_like() {
                continue;
            }
            let missing = m.procs.iter().enumerate().find(|(pi, p)| {
                p.info.kind == vba_pcode::ProcKind::SubOrFunction
                    && (p.name.eq_ignore_ascii_case("Class_Initialize")
                        || p.name.eq_ignore_ascii_case("Class_Terminate"))
                    && !lowered.contains_key(&(mi, *pi))
            });
            if let Some((_, bad)) = missing {
                for pi in 0..m.procs.len() {
                    if lowered.contains_key(&(mi, pi)) {
                        blocked.push((mi, pi, bad.name.clone()));
                    }
                }
            }
        }
        if dropped.is_empty() && blocked.is_empty() {
            break;
        }
        for (mi, pi, callee) in dropped {
            lowered.remove(&(mi, pi));
            let p = &models[mi].procs[pi];
            let callee_name = by_id
                .get(&callee)
                .map(|&(cm, cp)| models[cm].procs[cp].name.clone())
                .unwrap_or_default();
            reports[mi][pi] = Some(Outcome::Unsupported(Unsupported {
                opcode: vba_pcode::Opcode::InvokeSimple,
                procedure: p.name.clone(),
                line: p.header_line,
                reason: format!("calls `{callee_name}`, which is not lowered"),
            }));
        }
        for (mi, pi, init) in blocked {
            lowered.remove(&(mi, pi));
            let p = &models[mi].procs[pi];
            reports[mi][pi] = Some(Outcome::Unsupported(Unsupported {
                opcode: vba_pcode::Opcode::ProcHeader,
                procedure: p.name.clone(),
                line: p.header_line,
                reason: format!("instances run `{init}`, which is not lowered"),
            }));
        }
    }
    // Install bodies; everything else traps.
    for (mi, row) in reports.iter_mut().enumerate() {
        for (pi, report) in row.iter_mut().enumerate() {
            if let Some(id) = shell.proc_ids[mi][pi] {
                let template = ir.function_of(id).clone();
                ir.functions[id.index()] = match lowered.remove(&(mi, pi)) {
                    Some(l) => {
                        *report = Some(Outcome::Lowered);
                        l.function
                    }
                    None => trap_function(&template),
                };
            }
        }
    }
    // The verifier has the last word.
    for _ in 0..4 {
        match vba_ir::verify(&ir) {
            Ok(()) => break,
            Err(errs) => {
                let mut demoted = false;
                for e in &errs {
                    for (mi, m) in models.iter().enumerate() {
                        for (pi, p) in m.procs.iter().enumerate() {
                            let full = format!("{}.{}", m.name, p.name);
                            if e.function.eq_ignore_ascii_case(&full)
                                && let Some(id) = shell.proc_ids[mi][pi]
                                && matches!(reports[mi][pi], Some(Outcome::Lowered))
                            {
                                let template = ir.function_of(id).clone();
                                ir.functions[id.index()] = trap_function(&template);
                                reports[mi][pi] = Some(Outcome::Unsupported(Unsupported {
                                    opcode: vba_pcode::Opcode::ProcHeader,
                                    procedure: p.name.clone(),
                                    line: p.header_line,
                                    reason: format!("verifier: {}", e.message),
                                }));
                                demoted = true;
                            }
                        }
                    }
                }
                if !demoted {
                    return Err(LowerError::Verify(
                        errs.first().map(ToString::to_string).unwrap_or_default(),
                    ));
                }
            }
        }
    }
    let mut procedures = Vec::new();
    for (mi, m) in models.iter().enumerate() {
        for (pi, p) in m.procs.iter().enumerate() {
            procedures.push(ProcedureReport {
                module: m.name.clone(),
                procedure: p.name.clone(),
                id: shell.proc_ids[mi][pi],
                first_line: p.header_line,
                last_line: p.end_line,
                kind: p.info.kind,
                outcome: reports[mi][pi].clone().unwrap_or_else(|| {
                    Outcome::Unsupported(Unsupported {
                        opcode: vba_pcode::Opcode::ProcHeader,
                        procedure: p.name.clone(),
                        line: p.header_line,
                        reason: "not processed".into(),
                    })
                }),
            });
        }
    }
    let declarations = models
        .iter()
        .flat_map(|m| {
            m.decls.iter().map(|d| DeclarationReport {
                module: m.name.clone(),
                name: d.name.clone(),
                line: d.line,
                is_const: d.is_const,
                unsupported: d.problem.map(str::to_owned),
            })
        })
        .collect();
    Ok(LoweredProject {
        ir,
        procedures,
        declarations,
    })
}
