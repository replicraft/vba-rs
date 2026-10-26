//! Extraction of procedures from decoded module caches.

use vba_pcode::{
    Disassembler, Listed, ModuleCache, Name, Opcode, Operand, ProcKind, ProcedureInfo,
    ProjectCache, ResolvedType,
};

use vba_project::ModuleKind;

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

/// Scope word of a module-level declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeclScope {
    Public,
    Private,
}

/// A module-level variable or constant (a `VAR_DECL` outside every
/// procedure and user-defined type).
#[derive(Debug, Clone)]
pub(crate) struct ModuleDecl {
    pub line: usize,
    pub name: String,
    pub scope: DeclScope,
    pub is_const: bool,
    /// Declared scalar type; `None` for a constant without an `As` clause
    /// (the type then follows from the value).
    pub ty: Option<vba_sema::Type>,
    /// VBA text of the value of a constant.
    pub value: Option<String>,
    /// Why the declaration cannot be generated (it is then left out of the
    /// shell, and names that refer to it do not resolve).
    pub problem: Option<&'static str>,
}

/// One decoded module.
#[derive(Debug, Clone)]
pub(crate) struct ModuleModel {
    pub name: String,
    pub kind: ModuleKind,
    pub compare: CompareStored,
    /// `Option Explicit`: undeclared names are errors, so (in document-like
    /// modules) members of the host.
    pub explicit: bool,
    pub decls: Vec<ModuleDecl>,
    pub procs: Vec<ProcModel>,
}

impl ModuleModel {
    pub fn class_like(&self) -> bool {
        self.kind != ModuleKind::Standard
    }
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

/// Prefix of the names given to identifiers whose spelling the built-in
/// table does not confirm. Only the identity of such a name matters for a
/// parameter or a local (it shadows whatever the index stood for), so it
/// gets a placeholder spelling that cannot collide with a source name; it
/// never resolves to anything but that parameter or local.
pub(crate) const PLACEHOLDER: &str = "zzb_";

/// The spelling of a name for a parameter or local: the confirmed one, or a
/// placeholder for an unconfirmed built-in index.
pub(crate) fn local_spelling(n: &Name) -> Option<String> {
    match n {
        Name::UnknownBuiltin(i) => Some(format!("{PLACEHOLDER}{i}")),
        Name::OutOfRange(i) => Some(format!("{PLACEHOLDER}o{i}")),
        other => other.spelling().map(str::to_owned),
    }
}

/// Whether the procedure yields a value (`Function`, `Property Get`). The
/// header's function bit is also set for `Property Let`, so the record kind
/// decides for properties.
pub(crate) fn returns_value(info: &ProcedureInfo) -> bool {
    match info.kind {
        ProcKind::SubOrFunction => info.is_function,
        ProcKind::PropertyGet => true,
        _ => false,
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
    kind: ModuleKind,
    cache: &ModuleCache<'_>,
    pc: &ProjectCache,
) -> ModuleModel {
    let dis = Disassembler::new(cache, &pc.idents);
    let mut model = ModuleModel {
        name: name.to_owned(),
        kind,
        compare: CompareStored::Binary,
        explicit: false,
        decls: Vec::new(),
        procs: Vec::new(),
    };
    let mut cur: Option<ProcModel> = None;
    let mut decls = DeclScan::default();
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
                    model.explicit |= ins.modifier == 4;
                    model.compare = match ins.modifier {
                        2 => CompareStored::Text,
                        3 => CompareStored::Binary,
                        7 => CompareStored::Database,
                        _ => model.compare,
                    };
                }
                Opcode::ProcHeader | Opcode::ProcHeaderAlt => {
                    let mut info = match ins.operands.first() {
                        Some(Operand::Procedure(p)) => (**p).clone(),
                        _ => continue,
                    };
                    // Observed: in class-like modules the argument chain of
                    // a `Function` and of a `Property Get` ends with one
                    // more record, the result (its name is a reference
                    // outside the identifier table, its type the return
                    // type, and the record counts in the argument-count
                    // byte). It is not a parameter.
                    if kind != ModuleKind::Standard
                        && !info.is_declare
                        && returns_value(&info)
                        && matches!(info.args.last().map(|a| &a.name), Some(Name::OutOfRange(_)))
                    {
                        info.args.pop();
                    }
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
                _ => decls.feed(line, &ins, &mut model.decls),
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
    // Declarations that share a line stand or fall together: the line is
    // all the source side of a comparison can offer.
    let bad_lines: Vec<usize> = model
        .decls
        .iter()
        .filter(|d| d.problem.is_some())
        .map(|d| d.line)
        .collect();
    for d in &mut model.decls {
        if d.problem.is_none() && bad_lines.contains(&d.line) {
            d.problem = Some("shares a line with a declaration that is not supported");
        }
    }
    model
}

/// Collects the module-level declarations while the instructions outside
/// procedures stream by.
#[derive(Default)]
struct DeclScan {
    in_type: bool,
    scope_mod: u8,
    line: usize,
    expr: Vec<Listed>,
}

impl DeclScan {
    fn feed(&mut self, line: usize, ins: &Listed, out: &mut Vec<ModuleDecl>) {
        if line != self.line {
            self.line = line;
            self.scope_mod = 0;
            self.expr.clear();
        }
        match ins.opcode {
            Opcode::TypeDecl => self.in_type = true,
            Opcode::TypeDeclEnd => self.in_type = false,
            Opcode::DimMarker | Opcode::DimMarkerImplicit => {
                self.scope_mod = ins.modifier;
                self.expr.clear();
            }
            Opcode::VarDecl => {
                if !self.in_type {
                    out.push(self.declaration(line, ins));
                }
                self.expr.clear();
            }
            Opcode::CommentQuote
            | Opcode::CommentRemKeyword
            | Opcode::LineContinuation
            | Opcode::StmtBegin
            | Opcode::StmtBeginImplicit
            | Opcode::OptionStmt => {}
            _ => self.expr.push(ins.clone()),
        }
    }

    fn declaration(&self, line: usize, ins: &Listed) -> ModuleDecl {
        let mut d = ModuleDecl {
            line,
            name: String::new(),
            scope: if self.scope_mod & 0x0C != 0 {
                DeclScope::Public
            } else {
                DeclScope::Private
            },
            is_const: ins.modifier & 3 == 2,
            ty: None,
            value: None,
            problem: None,
        };
        let Some(Operand::Variable(v)) = ins.operands.first() else {
            d.problem = Some("unresolved variable");
            return d;
        };
        let Some(n) = v.name.spelling() else {
            d.problem = Some("name does not resolve");
            return d;
        };
        d.name = n.to_owned();
        if ins.modifier & 0x20 != 0 {
            d.problem = Some("WithEvents");
        } else if !matches!(ins.modifier & 3, 1 | 2) || self.scope_mod & 0x20 != 0 {
            d.problem = Some("declaration other than a plain variable or constant");
        } else if v.is_new {
            d.problem = Some("As New");
        } else if d.is_const && v.ty == ResolvedType::None {
            // The type follows from the value.
        } else {
            match scalar_type(&v.ty) {
                Some(t) => d.ty = Some(t),
                None => d.problem = Some("type is not a plain scalar"),
            }
        }
        if d.is_const && d.problem.is_none() {
            match const_text(&self.expr) {
                Some(t) => d.value = Some(t),
                None => d.problem = Some("constant value is not a literal"),
            }
        }
        d
    }
}

/// The VBA text of a constant's value: one literal, optionally negated.
/// Integer literals keep their type (`&` marks the 32-bit forms).
fn const_text(expr: &[Listed]) -> Option<String> {
    let (lit, neg) = match expr {
        [l] => (l, false),
        [l, n] if n.opcode == Opcode::NumNegate => (l, true),
        _ => return None,
    };
    let t = match (lit.opcode, lit.operands.first()) {
        (Opcode::ConstInt16Decimal, Some(Operand::Int { value, .. })) => value.to_string(),
        (Opcode::ConstInt32Decimal, Some(Operand::Int { value, .. })) => format!("{value}&"),
        (Opcode::ConstInt16Hex, Some(Operand::Int { value, .. })) if *value >= 0 => {
            format!("&H{value:X}")
        }
        (Opcode::ConstInt32Hex, Some(Operand::Int { value, .. })) if *value >= 0 => {
            format!("&H{value:X}&")
        }
        (Opcode::ConstFloat64, Some(Operand::Float64(v))) if v.is_finite() => format!("{v:?}"),
        (Opcode::ConstText, Some(Operand::Text(t))) if t.is_ascii() => {
            let body = String::from_utf8_lossy(t).replace('"', "\"\"");
            format!("\"{body}\"")
        }
        (Opcode::ConstKeywordValue, _) => match lit.modifier {
            0 => "False".to_owned(),
            1 => "True".to_owned(),
            _ => return None,
        },
        _ => return None,
    };
    if neg {
        if t.starts_with('"') || t == "True" || t == "False" {
            return None;
        }
        Some(format!("-{t}"))
    } else {
        Some(t)
    }
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
    if !matches!(
        info.kind,
        ProcKind::SubOrFunction | ProcKind::PropertyGet | ProcKind::PropertyLet
    ) {
        p.signature_problem = Some(fail("Property Set or procedure of unknown kind"));
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
        if local_spelling(&a.name).is_none() {
            p.signature_problem = Some(fail("parameter whose name does not resolve"));
            return;
        }
        if scalar_type(&a.ty).is_none() {
            p.signature_problem = Some(fail("parameter of a type that is not a plain scalar"));
            return;
        }
    }
    if returns_value(info) && scalar_type(&info.return_type).is_none() {
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
        let (Some(n), Some(ty)) = (local_spelling(&v.name), scalar_type(&v.ty)) else {
            p.signature_problem = Some(unsup(ins.opcode, &name, *l, "local of unsupported type"));
            return;
        };
        if v.is_new {
            p.signature_problem = Some(unsup(ins.opcode, &name, *l, "As New"));
            return;
        }
        locals.push(LocalDecl { name: n, ty });
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
