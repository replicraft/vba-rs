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
    pub ty: TyRef,
}

/// A type as the shell declares it: a plain scalar, the generic `Object`, or
/// a named type of the project (a class module, an `Enum`, a `Type`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TyRef {
    Scalar(vba_sema::Type),
    Object,
    Class(String),
    Enum(String),
    Udt(String),
}

impl TyRef {
    /// The VBA spelling in a generated `As` clause.
    pub fn spelling(&self) -> String {
        match self {
            TyRef::Scalar(t) => type_spelling(t).to_owned(),
            TyRef::Object => "Object".to_owned(),
            TyRef::Class(n) | TyRef::Enum(n) | TyRef::Udt(n) => n.clone(),
        }
    }
}

/// What a type name of the project stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TyKind {
    Enum,
    Udt,
}

/// A `Type` or `Enum` declared at module level.
#[derive(Debug, Clone)]
pub(crate) struct TypeDef {
    pub name: String,
    pub public: bool,
    pub is_enum: bool,
    pub start_line: usize,
    pub end_line: usize,
    /// Enum members, with the VBA text of an explicit value.
    pub members: Vec<(String, Option<String>)>,
    /// Fields of a `Type`, as stored.
    pub raw_fields: Vec<(String, ResolvedType)>,
    /// Fields of a `Type` once their types resolved.
    pub fields: Vec<(String, TyRef)>,
    /// Why the definition cannot be generated.
    pub problem: Option<String>,
}

/// The type names of the project, for resolving `Named` types.
#[derive(Debug, Default)]
pub(crate) struct TypeEnv {
    /// `[module]`: lower-case name to kind, for the types declared in the
    /// module and for the module itself when it is class-like (kept apart in
    /// `classes`).
    pub own: Vec<std::collections::HashMap<String, (TyKind, String)>>,
    /// Public types of all modules.
    pub public: std::collections::HashMap<String, (TyKind, String)>,
    /// Class-like modules by lower-case name.
    pub classes: std::collections::HashMap<String, String>,
    /// Lower-case name of the project itself: a type qualified with it is a
    /// type of the project.
    pub project: Option<String>,
}

impl TypeEnv {
    pub fn build(models: &[ModuleModel], project_name: Option<&str>) -> Self {
        let mut env = TypeEnv {
            project: project_name.map(str::to_ascii_lowercase),
            ..TypeEnv::default()
        };
        for m in models {
            let mut own = std::collections::HashMap::new();
            for t in &m.types {
                if t.problem.is_some() {
                    continue;
                }
                let k = if t.is_enum { TyKind::Enum } else { TyKind::Udt };
                let key = t.name.to_ascii_lowercase();
                own.insert(key.clone(), (k, t.name.clone()));
                if t.public {
                    env.public.entry(key).or_insert((k, t.name.clone()));
                }
            }
            env.own.push(own);
            if m.class_like() {
                env.classes
                    .insert(m.name.to_ascii_lowercase(), m.name.clone());
            }
        }
        env
    }

    /// Resolves a stored type of module `module`.
    pub fn resolve(&self, t: &ResolvedType, module: usize) -> Result<TyRef, String> {
        if let Some(s) = scalar_type(t) {
            return Ok(TyRef::Scalar(s));
        }
        match t {
            ResolvedType::Intrinsic(id) => {
                if id.flags == 0 && id.intrinsic.name() == Some("Object") {
                    Ok(TyRef::Object)
                } else {
                    Err("array, pointer or unlisted intrinsic type".to_owned())
                }
            }
            ResolvedType::Named(_) | ResolvedType::Qualified(_) => {
                let (n, qualifier) = match t {
                    ResolvedType::Qualified(q) => (&q.name, Some(&q.qualifier)),
                    ResolvedType::Named(n) => (n, None),
                    _ => unreachable!(),
                };
                let Some(name) = n.spelling() else {
                    return Err("type name does not resolve".to_owned());
                };
                if let Some(q) = qualifier {
                    let own = q.spelling().is_some_and(|q| {
                        self.project
                            .as_deref()
                            .is_some_and(|p| p.eq_ignore_ascii_case(q))
                    });
                    if !own {
                        return Err(format!("library-qualified type `{name}`"));
                    }
                }
                let key = name.to_ascii_lowercase();
                if let Some((k, canon)) =
                    self.own[module].get(&key).or_else(|| self.public.get(&key))
                {
                    return Ok(match k {
                        TyKind::Enum => TyRef::Enum(canon.clone()),
                        TyKind::Udt => TyRef::Udt(canon.clone()),
                    });
                }
                if let Some(canon) = self.classes.get(&key) {
                    return Ok(TyRef::Class(canon.clone()));
                }
                Err(format!("library or unresolved type `{name}`"))
            }
            ResolvedType::Unknown(_) => Err("unresolved type reference".to_owned()),
            ResolvedType::None => Ok(TyRef::Scalar(vba_sema::Type::Variant)),
        }
    }
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
    /// Types of the parameters and of the result, once resolved.
    pub arg_tys: Vec<TyRef>,
    pub ret_ty: Option<TyRef>,
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
    pub ty: Option<TyRef>,
    /// The stored type, resolved once the type names of the project are known.
    pub raw_ty: ResolvedType,
    /// VBA text of the value of a constant.
    pub value: Option<String>,
    /// Why the declaration cannot be generated (it is then left out of the
    /// shell, and names that refer to it do not resolve).
    pub problem: Option<String>,
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
    pub types: Vec<TypeDef>,
    /// `Implements` statements: line, the stored interface type and, once
    /// resolved, the class it names.
    pub implements: Vec<ImplementsDecl>,
    pub procs: Vec<ProcModel>,
}

/// An `Implements` statement of a class-like module.
#[derive(Debug, Clone)]
pub(crate) struct ImplementsDecl {
    pub line: usize,
    pub raw: ResolvedType,
    pub class: Result<String, String>,
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

/// Whether the type field of a constant comes from an `As` clause. A
/// constant without one has no type, or (observed) an intrinsic type with
/// flag 0x40 that is the type of its value.
pub(crate) fn declares_type(t: &ResolvedType) -> bool {
    match t {
        ResolvedType::None => false,
        ResolvedType::Intrinsic(id) => id.flags != 0x40,
        _ => true,
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
        types: Vec::new(),
        implements: Vec::new(),
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
                Opcode::ImplementsStmt => {
                    if let Some(Operand::Interface(t)) = ins.operands.get(1) {
                        model.implements.push(ImplementsDecl {
                            line,
                            raw: t.clone(),
                            class: Err("not resolved".into()),
                        });
                    }
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
                        arg_tys: Vec::new(),
                        ret_ty: None,
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
                        model.procs.push(p);
                    } else {
                        cur = Some(p);
                    }
                }
                _ => decls.feed(line, &ins, &mut model.decls, &mut model.types),
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
            d.problem = Some("shares a line with a declaration that is not supported".into());
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
    /// Line on which the open `Type` or `Enum` started.
    type_start: usize,
}

impl DeclScan {
    fn feed(
        &mut self,
        line: usize,
        ins: &Listed,
        out: &mut Vec<ModuleDecl>,
        types: &mut Vec<TypeDef>,
    ) {
        if line != self.line {
            self.line = line;
            self.scope_mod = 0;
            self.expr.clear();
        }
        match ins.opcode {
            Opcode::TypeDecl => {
                self.in_type = true;
                self.type_start = line;
                let (name, private) = match ins.operands.first() {
                    Some(Operand::UserType { name, private }) => {
                        (name.spelling().map(str::to_owned), *private)
                    }
                    _ => (None, true),
                };
                types.push(TypeDef {
                    name: name.clone().unwrap_or_default(),
                    public: !private,
                    is_enum: ins.modifier & 2 != 0,
                    members: Vec::new(),
                    raw_fields: Vec::new(),
                    fields: Vec::new(),
                    problem: name.is_none().then(|| "type name does not resolve".into()),
                    start_line: line,
                    end_line: line,
                });
            }
            Opcode::TypeDeclEnd | Opcode::EnumDeclEnd => {
                self.in_type = false;
                if let Some(t) = types.last_mut() {
                    t.end_line = line;
                    if t.is_enum != (ins.opcode == Opcode::EnumDeclEnd) {
                        t.problem = Some("Type and Enum markers do not match".into());
                    }
                }
            }
            Opcode::DimMarker | Opcode::DimMarkerImplicit => {
                self.scope_mod = ins.modifier;
                self.expr.clear();
            }
            Opcode::VarDecl => {
                if self.in_type {
                    if let Some(t) = types.last_mut() {
                        match ins.operands.first() {
                            Some(Operand::Variable(v)) if !t.is_enum => match v.name.spelling() {
                                Some(n) if ins.modifier & 0x23 == 1 && !v.is_new => {
                                    t.raw_fields.push((n.to_owned(), v.ty.clone()));
                                }
                                _ => t.problem = Some("field other than a plain one".into()),
                            },
                            _ => t.problem = Some("unresolved field".into()),
                        }
                    }
                } else {
                    out.push(self.declaration(line, ins));
                }
                self.expr.clear();
            }
            Opcode::InvokeSimple if self.in_type => {
                // An enum member without a value.
                if let (Some(t), Some(Operand::Name { name, .. })) =
                    (types.last_mut(), ins.operands.first())
                {
                    match name.spelling() {
                        Some(n) if t.is_enum => t.members.push((n.to_owned(), None)),
                        _ => t.problem = Some("enum member does not resolve".into()),
                    }
                }
                self.expr.clear();
            }
            Opcode::WriteSimple if self.in_type => {
                // An enum member with a value: the expression, then the name.
                if let (Some(t), Some(Operand::Name { name, .. })) =
                    (types.last_mut(), ins.operands.first())
                {
                    match (name.spelling(), const_text(&self.expr)) {
                        (Some(n), Some(v)) if t.is_enum => {
                            t.members.push((n.to_owned(), Some(v)));
                        }
                        _ => t.problem = Some("enum member value is not a literal".into()),
                    }
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
            raw_ty: ResolvedType::None,
            value: None,
            problem: None,
        };
        let Some(Operand::Variable(v)) = ins.operands.first() else {
            d.problem = Some("unresolved variable".into());
            return d;
        };
        let Some(n) = v.name.spelling() else {
            d.problem = Some("name does not resolve".into());
            return d;
        };
        d.name = n.to_owned();
        if ins.modifier & 0x20 != 0 {
            d.problem = Some("WithEvents".into());
        } else if !matches!(ins.modifier & 3, 1 | 2) || self.scope_mod & 0x20 != 0 {
            d.problem = Some("declaration other than a plain variable or constant".into());
        } else if v.is_new {
            d.problem = Some("As New".into());
        } else {
            // The type is resolved once the type names of the project are
            // known (`resolve_declarations`).
            d.raw_ty = v.ty.clone();
        }
        if d.is_const && d.problem.is_none() {
            match const_text(&self.expr) {
                Some(t) => d.value = Some(t),
                None => d.problem = Some("constant value is not a literal".into()),
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

/// Resolves the types of the declarations, types and procedures of module
/// `mi` against the type names of the whole project.
pub(crate) fn resolve_module(m: &mut ModuleModel, mi: usize, env: &TypeEnv) {
    for i in &mut m.implements {
        i.class = match env.resolve(&i.raw, mi) {
            Ok(TyRef::Class(n)) => Ok(n),
            Ok(_) => Err("interface that is not a class module of the project".to_owned()),
            Err(e) => Err(e),
        };
    }
    for t in &mut m.types {
        if t.problem.is_some() {
            continue;
        }
        if t.is_enum {
            if t.members.is_empty() {
                t.problem = Some("empty enum".into());
            }
            continue;
        }
        let mut fields = Vec::new();
        for (n, raw) in &t.raw_fields {
            match env.resolve(raw, mi) {
                Ok(ty) => fields.push((n.clone(), ty)),
                Err(e) => {
                    t.problem = Some(format!("field of {e}"));
                    break;
                }
            }
        }
        if t.problem.is_none() && fields.is_empty() {
            t.problem = Some("empty type".into());
        }
        t.fields = fields;
    }
    // A type that refers to a type with a problem has one itself.
    for _ in 0..4 {
        let bad: Vec<String> = m
            .types
            .iter()
            .filter(|t| t.problem.is_some())
            .map(|t| t.name.to_ascii_lowercase())
            .collect();
        for t in &mut m.types {
            if t.problem.is_none()
                && t.fields
                    .iter()
                    .any(|(_, ty)| matches!(ty, TyRef::Udt(n) | TyRef::Enum(n) if bad.contains(&n.to_ascii_lowercase())))
            {
                t.problem = Some("field of a type that is not supported".into());
            }
        }
    }
    for d in &mut m.decls {
        if d.problem.is_some() {
            continue;
        }
        if d.is_const && !declares_type(&d.raw_ty) {
            continue;
        }
        match env.resolve(&d.raw_ty, mi) {
            Ok(TyRef::Scalar(t)) => d.ty = Some(TyRef::Scalar(t)),
            Ok(t) if !d.is_const => d.ty = Some(t),
            Ok(_) => d.problem = Some("constant of a type that is not a plain scalar".into()),
            Err(e) => d.problem = Some(e),
        }
    }
    for p in &mut m.procs {
        finish_signature(p, mi, env);
    }
}

/// Checks the signature and collects the declared locals.
fn finish_signature(p: &mut ProcModel, mi: usize, env: &TypeEnv) {
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
        ProcKind::SubOrFunction
            | ProcKind::PropertyGet
            | ProcKind::PropertyLet
            | ProcKind::PropertySet
    ) {
        p.signature_problem = Some(fail("procedure of unknown kind"));
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
    let mut arg_tys = Vec::new();
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
        match env.resolve(&a.ty, mi) {
            Ok(t) => arg_tys.push(t),
            Err(e) => {
                p.signature_problem = Some(fail(&format!("parameter of {e}")));
                return;
            }
        }
    }
    if returns_value(info) {
        match env.resolve(&info.return_type, mi) {
            Ok(t) => p.ret_ty = Some(t),
            Err(e) => {
                p.signature_problem = Some(fail(&format!("return of {e}")));
                return;
            }
        }
    }
    p.arg_tys = arg_tys;
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
        if m & 0x20 == 0 && m & 0x03 == 2 && declares_type(&v.ty) {
            // A local constant is not a slot: the lowering folds its value.
            if scalar_type(&v.ty).is_none() {
                p.signature_problem = Some(unsup(
                    ins.opcode,
                    &name,
                    *l,
                    "local constant of unsupported type",
                ));
                return;
            }
            continue;
        }
        if m & 0x20 == 0 && m & 0x03 == 2 {
            continue;
        }
        if m & 0x20 != 0 || m & 0x03 != 1 {
            p.signature_problem = Some(unsup(
                ins.opcode,
                &name,
                *l,
                "declaration other than a plain Dim",
            ));
            return;
        }
        let Some(n) = local_spelling(&v.name) else {
            p.signature_problem = Some(unsup(ins.opcode, &name, *l, "local name does not resolve"));
            return;
        };
        let ty = match env.resolve(&v.ty, mi) {
            Ok(t) => t,
            Err(e) => {
                p.signature_problem = Some(unsup(ins.opcode, &name, *l, format!("local of {e}")));
                return;
            }
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
        if ins.opcode == Opcode::DimMarker && ins.modifier & 0x2C != 0 {
            p.signature_problem =
                Some(unsup(ins.opcode, &name, *l, "Static or scoped declaration"));
            return;
        }
    }
    p.locals = locals;
}
