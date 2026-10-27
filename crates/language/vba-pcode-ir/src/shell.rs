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
    CompareStored, DeclScope, ModuleModel, ProcModel, TyRef, local_spelling, returns_value,
};

/// The analysed shell.
pub(crate) struct Shell {
    /// IR with empty bodies for every included procedure.
    pub ir: IrProgram,
    /// `[module][proc]`: the id of an included procedure.
    pub proc_ids: Vec<Vec<Option<ProcId>>>,
    /// `[module][proc]`: the id of an included `Declare`.
    pub declare_ids: Vec<Vec<Option<vba_sema::DeclareId>>>,
}

/// What a byte range of a stub text was generated from.
#[derive(Debug, Clone, Copy)]
enum Origin {
    Proc(usize),
    Decl(usize),
    Type(usize),
    Implements(usize),
}

struct Stub {
    text: String,
    /// `(start, end, origin)` byte ranges.
    ranges: Vec<(usize, usize, Origin)>,
}

fn stub_of(m: &ModuleModel, include: &[bool]) -> Stub {
    let mut ranges = Vec::new();
    let mut text = String::new();
    if m.explicit {
        text.push_str("Option Explicit\n");
    }
    if m.predeclared {
        text.push_str("Attribute VB_PredeclaredId = True\n");
    }
    match m.compare {
        CompareStored::Binary => {}
        CompareStored::Text => text.push_str("Option Compare Text\n"),
        CompareStored::Database => text.push_str("Option Compare Database\n"),
    }
    if m.option_base == Some(1) {
        text.push_str("Option Base 1\n");
    }
    for (ii, i) in m.implements.iter().enumerate() {
        if let Ok(n) = &i.class {
            let istart = text.len();
            let _ = writeln!(text, "Implements {n}");
            ranges.push((istart, text.len(), Origin::Implements(ii)));
        }
    }
    for (ti, t) in m.types.iter().enumerate() {
        if t.problem.is_some() {
            continue;
        }
        let tstart = text.len();
        let scope = if t.public { "Public" } else { "Private" };
        if t.is_enum {
            let _ = writeln!(text, "{scope} Enum {}", t.name);
            for (n, v) in &t.members {
                match v {
                    Some(v) => {
                        let _ = writeln!(text, "    {n} = {v}");
                    }
                    None => {
                        let _ = writeln!(text, "    {n}");
                    }
                }
            }
            text.push_str("End Enum\n");
        } else {
            let _ = writeln!(text, "{scope} Type {}", t.name);
            for (n, ty) in &t.fields {
                let _ = writeln!(text, "    {n} As {}", ty.spelling());
            }
            text.push_str("End Type\n");
        }
        ranges.push((tstart, text.len(), Origin::Type(ti)));
    }
    for (di, d) in m.decls.iter().enumerate() {
        if d.problem.is_some() {
            continue;
        }
        let dstart = text.len();
        let scope = match d.scope {
            DeclScope::Public => "Public",
            DeclScope::Private => "Private",
        };
        let as_clause =
            d.ty.as_ref()
                .map_or_else(String::new, |t| format!(" As {}", t.spelling()));
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
        ranges.push((dstart, text.len(), Origin::Decl(di)));
    }
    for (i, p) in m.procs.iter().enumerate() {
        if !include[i] {
            continue;
        }
        let start = text.len();
        if p.info.is_declare {
            write_declare(&mut text, p);
            ranges.push((start, text.len(), Origin::Proc(i)));
            continue;
        }
        let (kw, end_kw) = match p.info.kind {
            ProcKind::PropertyGet => ("Property Get", "Property"),
            ProcKind::PropertyLet => ("Property Let", "Property"),
            ProcKind::PropertySet => ("Property Set", "Property"),
            _ if p.info.is_function => ("Function", "Function"),
            _ => ("Sub", "Sub"),
        };
        let _ = write!(text, "Public {kw} {}(", p.name);
        for (k, a) in p.info.args.iter().enumerate() {
            if k > 0 {
                text.push_str(", ");
            }
            let ty = p
                .arg_tys
                .get(k)
                .map_or_else(|| "Variant".to_owned(), |t| t.spelling());
            let array = matches!(p.arg_tys.get(k), Some(TyRef::Array(_)));
            let _ = write!(
                text,
                "{}{} {}{} As {}",
                if a.optional { "Optional " } else { "" },
                if a.param_array {
                    "ParamArray"
                } else if a.by_val && !array {
                    "ByVal"
                } else {
                    "ByRef"
                },
                local_spelling(&a.name).unwrap_or_else(|| "_".into()),
                if array { "()" } else { "" },
                ty
            );
            if let Some(Some(d)) = p.defaults.get(k) {
                let _ = write!(text, " = {d}");
            }
        }
        text.push(')');
        if returns_value(&p.info) {
            let ty = p
                .ret_ty
                .as_ref()
                .map_or_else(|| "Variant".to_owned(), |t| t.spelling());
            let array = if matches!(p.ret_ty, Some(TyRef::Array(_))) {
                "()"
            } else {
                ""
            };
            let _ = write!(text, " As {ty}{array}");
        }
        text.push('\n');
        for l in &p.locals {
            let dims = l.dims.as_ref().map_or_else(String::new, |d| format!("({d})"));
            let _ = writeln!(text, "Dim {}{dims} As {}", l.name, l.ty.spelling());
        }
        let _ = writeln!(text, "End {end_kw}");
        ranges.push((start, text.len(), Origin::Proc(i)));
    }
    Stub { text, ranges }
}

/// The `Declare` statement of a procedure record.
fn write_declare(text: &mut String, p: &ProcModel) {
    let lib = p
        .info
        .library
        .as_ref()
        .and_then(local_spelling)
        .unwrap_or_default();
    let entry = p
        .info
        .entry_point
        .as_ref()
        .map(|e| String::from_utf8_lossy(e).into_owned())
        .unwrap_or_default();
    let _ = write!(
        text,
        "{} Declare {}{} {} Lib \"{lib}\"",
        if p.info.public { "Public" } else { "Private" },
        // The target is 64-bit VBA 7 whatever produced the cache: the
        // keyword has no other effect on the declaration.
        "PtrSafe ",
        if p.info.is_function { "Function" } else { "Sub" },
        p.name
    );
    if entry != p.name {
        let _ = write!(text, " Alias \"{entry}\"");
    }
    text.push('(');
    for (k, a) in p.info.args.iter().enumerate() {
        if k > 0 {
            text.push_str(", ");
        }
        let ty = p
            .arg_tys
            .get(k)
            .map_or_else(|| "Variant".to_owned(), |t| t.spelling());
        let array = matches!(p.arg_tys.get(k), Some(TyRef::Array(_)));
        let _ = write!(
            text,
            "{}{} {}{} As {}",
            if a.optional { "Optional " } else { "" },
            if a.by_val && !array { "ByVal" } else { "ByRef" },
            local_spelling(&a.name).unwrap_or_else(|| "_".into()),
            if array { "()" } else { "" },
            ty
        );
    }
    text.push(')');
    if returns_value(&p.info) {
        let ty = p
            .ret_ty
            .as_ref()
            .map_or_else(|| "Variant".to_owned(), |t| t.spelling());
        let _ = write!(text, " As {ty}");
    }
    text.push('\n');
}

/// The procedure of the analysed program that was generated for `p`
/// (property procedures share a name; the kind tells them apart).
fn find_proc(program: &vba_sema::Program, module: &str, p: &ProcModel) -> Option<ProcId> {
    let want = match p.info.kind {
        ProcKind::PropertyGet => vba_ast::ProcKind::PropertyGet,
        ProcKind::PropertyLet => vba_ast::ProcKind::PropertyLet,
        ProcKind::PropertySet => vba_ast::ProcKind::PropertySet,
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

/// The `Declare` of the analysed program that was generated for `p`.
fn find_declare(
    program: &vba_sema::Program,
    module: &str,
    p: &ProcModel,
) -> Option<vba_sema::DeclareId> {
    let m = program.module_by_name(module)?;
    let n = vba_ast::Name::new(&p.name);
    m.declares
        .iter()
        .map(|id| &program.declares[id.index()])
        .find(|d| d.name == n)
        .map(|d| d.id)
}

/// Builds the shell for the procedures marked in `include` (`[module][proc]`),
/// dropping procedures whose stub the analysis rejects (they are reported by
/// clearing their flag).
pub(crate) fn build_shell(
    models: &mut [ModuleModel],
    include: &mut [Vec<bool>],
) -> Result<Shell, LowerError> {
    let module_list: Vec<usize> = (0..models.len()).collect();
    for _round in 0..64 {
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
            let mut declare_ids: Vec<Vec<Option<vba_sema::DeclareId>>> =
                models.iter().map(|m| vec![None; m.procs.len()]).collect();
            for &mi in &module_list {
                if ir.program.module_by_name(&models[mi].name).is_none() {
                    continue;
                }
                for (pi, p) in models[mi].procs.iter().enumerate() {
                    if !include[mi][pi] {
                        continue;
                    }
                    if p.info.is_declare {
                        declare_ids[mi][pi] = find_declare(&ir.program, &models[mi].name, p);
                    } else if let Some(id) = find_proc(&ir.program, &models[mi].name, p) {
                        proc_ids[mi][pi] = Some(id);
                    }
                }
            }
            return Ok(Shell {
                ir,
                proc_ids,
                declare_ids,
            });
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
                Some(Origin::Proc(pi)) => {
                    changed |= include[mi][pi];
                    include[mi][pi] = false;
                }
                Some(Origin::Decl(di)) => {
                    let decl = &mut models[mi].decls[di];
                    if decl.problem.is_none() {
                        decl.problem = Some(format!(
                            "rejected by the analysis of the generated declarations: {}",
                            d.message
                        ));
                        changed = true;
                    }
                }
                Some(Origin::Implements(ii)) => {
                    let imp = &mut models[mi].implements[ii];
                    if imp.class.is_ok() {
                        imp.class =
                            Err("rejected by the analysis of the generated declarations".into());
                        changed = true;
                    }
                }
                Some(Origin::Type(ti)) => {
                    let t = &mut models[mi].types[ti];
                    if t.problem.is_none() {
                        t.problem = Some(format!(
                            "rejected by the analysis of the generated declarations: {}",
                            d.message
                        ));
                        changed = true;
                    }
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
            return Err(LowerError::Shell(format!(
                "the generated declarations do not analyse: {}",
                analysis
                    .diagnostics
                    .iter()
                    .find(|d| d.severity == vba_diagnostics::Severity::Error)
                    .map_or("", |d| d.message.as_str())
            )));
        }
    }
    Err(LowerError::Shell(
        "the generated declarations do not settle".into(),
    ))
}
