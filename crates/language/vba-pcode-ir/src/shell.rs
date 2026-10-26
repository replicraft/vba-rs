//! The program shell: module, procedure and slot tables that the lowered
//! bodies refer to.
//!
//! `vba-ir` functions point into a `vba_sema::Program` (procedure ids,
//! signatures, local slots). The shell obtains that program by analysing a
//! declaration-only text generated from the *cache* records of the project
//! (procedure headers, parameter and local declarations), with empty
//! bodies. Nothing of the original source text is used. The bodies of the
//! resulting IR are then replaced by the ones lowered from the compiled
//! instructions.

use std::fmt::Write as _;

use vba_ir::IrProgram;
use vba_sema::{Options, ProcId, SourceModule};

use crate::error::LowerError;
use vba_pcode::ProcKind;

use crate::model::{
    CompareStored, DeclScope, ModuleModel, ProcModel, local_spelling, returns_value, scalar_type,
    type_spelling,
};

/// The analysed shell.
pub(crate) struct Shell {
    /// IR with empty bodies for every included procedure.
    pub ir: IrProgram,
    /// `[module][proc]`: the id of an included procedure.
    pub proc_ids: Vec<Vec<Option<ProcId>>>,
}

struct Stub {
    text: String,
    /// `(start, end, proc index)` byte ranges.
    ranges: Vec<(usize, usize, usize)>,
}

fn stub_of(m: &ModuleModel, include: &[bool]) -> Stub {
    let mut text = String::new();
    if m.explicit {
        text.push_str("Option Explicit\n");
    }
    match m.compare {
        CompareStored::Binary => {}
        CompareStored::Text => text.push_str("Option Compare Text\n"),
        CompareStored::Database => text.push_str("Option Compare Database\n"),
    }
    for d in &m.decls {
        if d.problem.is_some() {
            continue;
        }
        let scope = match d.scope {
            DeclScope::Public => "Public",
            DeclScope::Private => "Private",
        };
        let as_clause =
            d.ty.as_ref()
                .map_or_else(String::new, |t| format!(" As {}", type_spelling(t)));
        if d.is_const {
            let _ = writeln!(
                text,
                "{scope} Const {}{as_clause} = {}",
                d.name,
                d.value.as_deref().unwrap_or("0")
            );
        } else {
            let _ = writeln!(text, "{scope} {}{as_clause}", d.name);
        }
    }
    let mut ranges = Vec::new();
    for (i, p) in m.procs.iter().enumerate() {
        if !include[i] {
            continue;
        }
        let start = text.len();
        let (kw, end_kw) = match p.info.kind {
            ProcKind::PropertyGet => ("Property Get", "Property"),
            ProcKind::PropertyLet => ("Property Let", "Property"),
            _ if p.info.is_function => ("Function", "Function"),
            _ => ("Sub", "Sub"),
        };
        let _ = write!(text, "Public {kw} {}(", p.name);
        for (k, a) in p.info.args.iter().enumerate() {
            if k > 0 {
                text.push_str(", ");
            }
            let ty = scalar_type(&a.ty).unwrap_or(vba_sema::Type::Variant);
            let _ = write!(
                text,
                "{}{} {} As {}",
                if a.optional { "Optional " } else { "" },
                if a.by_val { "ByVal" } else { "ByRef" },
                local_spelling(&a.name).unwrap_or_else(|| "_".into()),
                type_spelling(&ty)
            );
            if let Some(Some(d)) = p.defaults.get(k) {
                let _ = write!(text, " = {d}");
            }
        }
        text.push(')');
        if returns_value(&p.info) {
            let ty = scalar_type(&p.info.return_type).unwrap_or(vba_sema::Type::Variant);
            let _ = write!(text, " As {}", type_spelling(&ty));
        }
        text.push('\n');
        for l in &p.locals {
            let _ = writeln!(text, "Dim {} As {}", l.name, type_spelling(&l.ty));
        }
        let _ = writeln!(text, "End {end_kw}");
        ranges.push((start, text.len(), i));
    }
    Stub { text, ranges }
}

/// The procedure of the analysed program that was generated for `p`
/// (property procedures share a name; the kind tells them apart).
fn find_proc(program: &vba_sema::Program, module: &str, p: &ProcModel) -> Option<ProcId> {
    let want = match p.info.kind {
        ProcKind::PropertyGet => vba_ast::ProcKind::PropertyGet,
        ProcKind::PropertyLet => vba_ast::ProcKind::PropertyLet,
        _ if p.info.is_function => vba_ast::ProcKind::Function,
        _ => vba_ast::ProcKind::Sub,
    };
    let m = program.module_by_name(module)?;
    let n = vba_ast::Name::new(&p.name);
    m.procs
        .iter()
        .map(|id| program.proc(*id))
        .find(|q| q.name == n && q.kind == want)
        .map(|q| q.id)
}

/// Builds the shell for the procedures marked in `include` (`[module][proc]`),
/// dropping procedures whose stub the analysis rejects (they are reported by
/// clearing their flag).
pub(crate) fn build_shell(
    models: &[ModuleModel],
    include: &mut [Vec<bool>],
) -> Result<Shell, LowerError> {
    let module_list: Vec<usize> = (0..models.len()).collect();
    for _round in 0..16 {
        let stubs: Vec<Stub> = module_list
            .iter()
            .map(|&i| stub_of(&models[i], &include[i]))
            .collect();
        let sources: Vec<SourceModule> = module_list
            .iter()
            .zip(&stubs)
            .map(|(&i, s)| SourceModule {
                name: models[i].name.clone(),
                kind: models[i].kind,
                source: s.text.clone().into_bytes(),
            })
            .collect();
        let analysis = vba_sema::analyze_sources(
            &sources,
            vba_profile::default_profile(),
            &vba_sema::NoLibraries,
            &Options::default(),
        );
        let errors: Vec<_> = analysis
            .diagnostics
            .iter()
            .filter(|d| d.severity == vba_diagnostics::Severity::Error)
            .collect();
        if errors.is_empty() {
            let ir = vba_ir::lower(&analysis).map_err(|e| LowerError::Shell(e.to_string()))?;
            let mut proc_ids: Vec<Vec<Option<ProcId>>> =
                models.iter().map(|m| vec![None; m.procs.len()]).collect();
            for &mi in &module_list {
                if ir.program.module_by_name(&models[mi].name).is_none() {
                    continue;
                }
                for (pi, p) in models[mi].procs.iter().enumerate() {
                    if include[mi][pi]
                        && let Some(id) = find_proc(&ir.program, &models[mi].name, p)
                    {
                        proc_ids[mi][pi] = Some(id);
                    }
                }
            }
            return Ok(Shell { ir, proc_ids });
        }
        // Drop the procedures that own an error; if an error cannot be
        // attributed to a procedure, drop every procedure of that module.
        let mut changed = false;
        for d in errors {
            let Some(&mi) = module_list.get(d.file.0 as usize) else {
                return Err(LowerError::Shell(d.message.clone()));
            };
            let off = d.span.start as usize;
            let hit = stubs[d.file.0 as usize]
                .ranges
                .iter()
                .find(|(s, e, _)| off >= *s && off < *e)
                .map(|r| r.2);
            match hit {
                Some(pi) => {
                    changed |= include[mi][pi];
                    include[mi][pi] = false;
                }
                None => {
                    for f in &mut include[mi] {
                        changed |= *f;
                        *f = false;
                    }
                }
            }
        }
        if !changed {
            return Err(LowerError::Shell(
                "the generated declarations do not analyse".into(),
            ));
        }
    }
    Err(LowerError::Shell(
        "the generated declarations do not settle".into(),
    ))
}
