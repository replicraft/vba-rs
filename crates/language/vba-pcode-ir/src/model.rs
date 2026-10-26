//! Extraction of procedures from decoded module caches.

use vba_pcode::{
    Disassembler, Listed, ModuleCache, Name, Opcode, Operand, ProcKind, ProcedureInfo,
    ProjectCache, ResolvedType,
};

use crate::error::Unsupported;

/// The `Option Compare` mode of a module as stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompareStored {
    /// `Option Compare Binary` or none.
    Binary,
    /// `Option Compare Text`.
    Text,
    /// `Option Compare Database`.
    Database,
}

/// A declared local of a procedure (a `VAR_DECL`).
#[derive(Debug, Clone)]
pub(crate) struct LocalDecl {
    pub name: String,
    pub ty: vba_sema::Type,
}

/// One decoded procedure.
#[derive(Debug, Clone)]
pub(crate) struct ProcModel {
    pub name: String,
    pub info: ProcedureInfo,
    pub header_line: usize,
    /// The line of the end instruction (the header line for `Declare`).
    pub end_line: usize,
    /// Instructions after the header up to (excluding) the end instruction,
    /// each with its line index.
    pub body: Vec<(usize, Listed)>,
    /// Why the procedure cannot even get a signature in the shell.
    pub signature_problem: Option<Unsupported>,
    pub locals: Vec<LocalDecl>,
    /// Default values of the arguments (VBA spelling), aligned with
    /// `info.args`; `None` for arguments without one.
    pub defaults: Vec<Option<String>>,
}

/// One decoded module.
#[derive(Debug, Clone)]
pub(crate) struct ModuleModel {
    pub name: String,
    pub class_like: bool,
    pub compare: CompareStored,
    pub procs: Vec<ProcModel>,
}

/// Maps a stored scalar type to the semantic type, `None` when it is not a
/// plain scalar this crate handles.
pub(crate) fn scalar_type(t: &ResolvedType) -> Option<vba_sema::Type> {
    use vba_sema::Type;
    match t {
        ResolvedType::None => Some(Type::Variant),
        ResolvedType::Intrinsic(id) => {
            if id.flags != 0 {
                return None;
            }
            Some(match id.intrinsic.name()? {
                "Integer" => Type::Integer,
                "Long" => Type::Long,
                "Single" => Type::Single,
                "Double" => Type::Double,
                "Currency" => Type::Currency,
                "Date" => Type::Date,
                "String" => Type::String,
                "Boolean" => Type::Boolean,
                "Byte" => Type::Byte,
                "Variant" => Type::Variant,
                _ => return None,
            })
        }
        _ => None,
    }
}

/// The VBA spelling of a scalar type for the shell source.
pub(crate) fn type_spelling(t: &vba_sema::Type) -> &'static str {
    use vba_sema::Type;
    match t {
        Type::Integer => "Integer",
        Type::Long => "Long",
        Type::Single => "Single",
        Type::Double => "Double",
        Type::Currency => "Currency",
        Type::Date => "Date",
        Type::String => "String",
        Type::Boolean => "Boolean",
        Type::Byte => "Byte",
        _ => "Variant",
    }
}

fn unsup(opcode: Opcode, procedure: &str, line: usize, reason: impl Into<String>) -> Unsupported {
    Unsupported {
        opcode,
        procedure: procedure.to_owned(),
        line,
        reason: reason.into(),
    }
}

fn is_end(op: Opcode) -> bool {
    matches!(
        op,
        Opcode::ProcEndSub | Opcode::ProcEndFunction | Opcode::ProcEndProperty
    )
}

/// Decodes the procedures of one module.
pub(crate) fn extract_module(
    name: &str,
    class_like: bool,
    cache: &ModuleCache<'_>,
    pc: &ProjectCache,
) -> ModuleModel {
    let dis = Disassembler::new(cache, &pc.idents);
    let mut model = ModuleModel {
        name: name.to_owned(),
        class_like,
        compare: CompareStored::Binary,
        procs: Vec::new(),
    };
    let mut cur: Option<ProcModel> = None;
    for line in 0..cache.lines.lines.len() {
        let listing = dis.line(line);
        let decode_failed = listing.error.is_some();
        // Default-value expressions of the header line precede the header
        // instruction, after a `CONST_EXPR_WITH_CALL` marker.
        let mut pending: Option<Vec<Listed>> = None;
        for ins in listing.instructions {
            if cur.is_some() && is_end(ins.opcode) {
                if let Some(mut done) = cur.take() {
                    done.end_line = line;
                    finish_signature(&mut done);
                    model.procs.push(done);
                }
                continue;
            }
            if let Some(p) = cur.as_mut() {
                p.body.push((line, ins));
                continue;
            }
            if ins.opcode == Opcode::ConstExprWithCall {
                pending = Some(Vec::new());
                continue;
            }
            if !matches!(ins.opcode, Opcode::ProcHeader | Opcode::ProcHeaderAlt)
                && let Some(list) = pending.as_mut()
            {
                if ins.opcode != Opcode::LineContinuation {
                    list.push(ins);
                }
                continue;
            }
            match ins.opcode {
                Opcode::OptionStmt => {
                    model.compare = match ins.modifier {
                        2 => CompareStored::Text,
                        3 => CompareStored::Binary,
                        7 => CompareStored::Database,
                        _ => model.compare,
                    };
                }
                Opcode::ProcHeader | Opcode::ProcHeaderAlt => {
                    let info = match ins.operands.first() {
                        Some(Operand::Procedure(p)) => (**p).clone(),
                        _ => continue,
                    };
                    let pname = info
                        .name
                        .spelling()
                        .map_or_else(|| format!("proc@{line}"), str::to_owned);
                    let mut p = ProcModel {
                        name: pname,
                        info,
                        header_line: line,
                        end_line: line,
                        body: Vec::new(),
                        signature_problem: None,
                        locals: Vec::new(),
                        defaults: Vec::new(),
                    };
                    match default_values(&p.info, pending.take()) {
                        Ok(d) => p.defaults = d,
                        Err(reason) => {
                            p.signature_problem = Some(unsup(ins.opcode, &p.name, line, reason));
                        }
                    }
                    if ins.opcode == Opcode::ProcHeaderAlt {
                        p.signature_problem = Some(unsup(
                            ins.opcode,
                            &p.name,
                            line,
                            "PROC_HEADER_ALT is unseen",
                        ));
                    }
                    if p.info.is_declare {
                        // A `Declare` statement has no body and no end.
                        finish_signature(&mut p);
                        model.procs.push(p);
                    } else {
                        cur = Some(p);
                    }
                }
                _ => {}
            }
        }
        if decode_failed && let Some(p) = &mut cur {
            p.signature_problem.get_or_insert_with(|| Unsupported {
                opcode: Opcode::InvalidSentinel,
                procedure: p.name.clone(),
                line,
                reason: "the line does not decode".into(),
            });
        }
    }
    if let Some(mut p) = cur.take() {
        // A procedure without an end instruction: keep it, unsupported.
        p.signature_problem.get_or_insert_with(|| Unsupported {
            opcode: Opcode::InvalidSentinel,
            procedure: p.name.clone(),
            line: p.header_line,
            reason: "no end-of-procedure instruction".into(),
        });
        model.procs.push(p);
    }
    model
}

/// Renders the default-value expressions of a header line as VBA text, one
/// per argument that declares a default. The expressions are stored in
/// reverse argument order (observed: the last defaulted argument first).
fn default_values(
    info: &ProcedureInfo,
    exprs: Option<Vec<Listed>>,
) -> Result<Vec<Option<String>>, &'static str> {
    let wanted = info.args.iter().filter(|a| a.has_default).count();
    let mut texts: Vec<String> = Vec::new();
    for ins in exprs.unwrap_or_default() {
        let t = match (ins.opcode, ins.operands.first()) {
            (
                Opcode::ConstInt16Decimal | Opcode::ConstInt32Decimal,
                Some(Operand::Int { value, .. }),
            ) => value.to_string(),
            (Opcode::ConstFloat64, Some(Operand::Float64(v))) if v.is_finite() => format!("{v:?}"),
            (Opcode::ConstText, Some(Operand::Text(t))) if t.is_ascii() => {
                let body = String::from_utf8_lossy(t).replace('\"', "\"\"");
                format!("\"{body}\"")
            }
            (Opcode::ConstKeywordValue, _) => match ins.modifier {
                0 => "False".to_owned(),
                1 => "True".to_owned(),
                _ => return Err("default value expression is not supported"),
            },
            (
                Opcode::ReadSimple,
                Some(Operand::Name {
                    name,
                    suffix: None,
                    bracketed: false,
                }),
            ) if !matches!(name, Name::Me) && name.spelling().is_some() => {
                name.spelling().unwrap_or_default().to_owned()
            }
            _ => return Err("default value expression is not supported"),
        };
        texts.push(t);
    }
    if texts.len() != wanted {
        return Err("default values do not match the arguments");
    }
    texts.reverse();
    let mut it = texts.into_iter();
    Ok(info
        .args
        .iter()
        .map(|a| if a.has_default { it.next() } else { None })
        .collect())
}

/// Checks the signature and collects the declared locals.
fn finish_signature(p: &mut ProcModel) {
    let line = p.header_line;
    let name = p.name.clone();
    let fail = |reason: &str| unsup(Opcode::ProcHeader, &name, line, reason);
    if p.signature_problem.is_some() {
        return;
    }
    let info = &p.info;
    if info.is_declare {
        p.signature_problem = Some(fail("Declare"));
        return;
    }
    if info.kind != ProcKind::SubOrFunction {
        p.signature_problem = Some(fail("Property procedure"));
        return;
    }
    if info.is_static {
        p.signature_problem = Some(fail("Static procedure"));
        return;
    }
    if !matches!(info.name, Name::User(_) | Name::Builtin(_)) {
        p.signature_problem = Some(fail("procedure name does not resolve"));
        return;
    }
    for a in &info.args {
        if a.param_array {
            p.signature_problem = Some(fail("ParamArray parameter"));
            return;
        }
        if a.has_default && !a.optional {
            p.signature_problem = Some(fail("default value on a required parameter"));
            return;
        }
        if a.name.spelling().is_none() || scalar_type(&a.ty).is_none() {
            p.signature_problem = Some(fail("parameter of a type that is not a plain scalar"));
            return;
        }
    }
    if info.is_function && scalar_type(&info.return_type).is_none() {
        p.signature_problem = Some(fail("return type is not a plain scalar"));
        return;
    }
    // Locals.
    let mut locals = Vec::new();
    for (l, ins) in &p.body {
        if ins.opcode != Opcode::VarDecl {
            continue;
        }
        let v = match ins.operands.first() {
            Some(Operand::Variable(v)) => v,
            _ => {
                p.signature_problem = Some(unsup(ins.opcode, &name, *l, "unresolved variable"));
                return;
            }
        };
        // Bits 0x01/0x02 of the modifier distinguish `Dim` from `Const`;
        // 0x20 is WithEvents; 0x10 only adds a layout word (seen on
        // declarations whose `As` is aligned with extra spaces).
        let m = ins.modifier;
        if m & 0x20 != 0 || m & 0x03 != 1 {
            p.signature_problem = Some(unsup(
                ins.opcode,
                &name,
                *l,
                "declaration other than a plain Dim",
            ));
            return;
        }
        let (Some(n), Some(ty)) = (v.name.spelling(), scalar_type(&v.ty)) else {
            p.signature_problem = Some(unsup(ins.opcode, &name, *l, "local of unsupported type"));
            return;
        };
        if v.is_new {
            p.signature_problem = Some(unsup(ins.opcode, &name, *l, "As New"));
            return;
        }
        locals.push(LocalDecl {
            name: n.to_owned(),
            ty,
        });
    }
    // A `Dim` marker with a scope flag other than the plain local one means
    // `Static`/`Public`/`Private` inside a procedure.
    for (l, ins) in &p.body {
        if ins.opcode == Opcode::DimMarker && ins.modifier & 0x2D != 0 {
            p.signature_problem =
                Some(unsup(ins.opcode, &name, *l, "Static or scoped declaration"));
            return;
        }
    }
    p.locals = locals;
}
