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
use vba_project::ModuleKind;
use vba_sema::{Options, ProcId, SourceModule};

use crate::error::LowerError;
use crate::model::{CompareStored, ModuleModel, scalar_type, type_spelling};

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
    match m.compare {
        CompareStored::Binary => {}
        CompareStored::Text => text.push_str("Option Compare Text\n"),
        CompareStored::Database => text.push_str("Option Compare Database\n"),
    }
    let mut ranges = Vec::new();
    for (i, p) in m.procs.iter().enumerate() {
        if !include[i] {
            continue;
        }
        let start = text.len();
        let kw = if p.info.is_function {
            "Function"
        } else {
            "Sub"
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
                a.name.spelling().unwrap_or("_"),
                type_spelling(&ty)
            );
            if let Some(Some(d)) = p.defaults.get(k) {
                let _ = write!(text, " = {d}");
            }
        }
        text.push(')');
        if p.info.is_function {
            let ty = scalar_type(&p.info.return_type).unwrap_or(vba_sema::Type::Variant);
            let _ = write!(text, " As {}", type_spelling(&ty));
        }
        text.push('\n');
        for l in &p.locals {
            let _ = writeln!(text, "Dim {} As {}", l.name, type_spelling(&l.ty));
        }
        let _ = writeln!(text, "End {kw}");
        ranges.push((start, text.len(), i));
    }
    Stub { text, ranges }
}

/// Builds the shell for the procedures marked in `include` (`[module][proc]`),
/// dropping procedures whose stub the analysis rejects (they are reported by
/// clearing their flag).
pub(crate) fn build_shell(
    models: &[ModuleModel],
    include: &mut [Vec<bool>],
) -> Result<Shell, LowerError> {
    // Only standard modules take part; class-like modules need instances.
    let module_list: Vec<usize> = (0..models.len())
        .filter(|&i| !models[i].class_like)
        .collect();
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
                kind: ModuleKind::Standard,
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
                        && let Some(pinfo) = ir.program.find_proc(&models[mi].name, &p.name)
                    {
                        proc_ids[mi][pi] = Some(pinfo.id);
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
