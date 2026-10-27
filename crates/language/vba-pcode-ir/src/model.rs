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
    /// For an array: the text between the parentheses (empty for a dynamic
    /// array, `0 To 30, 1 To 2` for fixed bounds).
    pub dims: Option<String>,
}

/// A type as the shell declares it: a plain scalar, the generic `Object`, or
/// a named type of the project (a class module, an `Enum`, a `Type`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TyRef {
    Scalar(vba_sema::Type),
    Object,
    /// A class of the built-in library that `vba-sema` describes
    /// (`Collection`, `ErrObject`).
    Builtin(String),
    /// An enumeration of the built-in library (`VbVarType`): a `Long`.
    LibEnum(String),
    Class(String),
    Enum(String),
    Udt(String),
    /// An array of a scalar element type (a local, a parameter or a result;
    /// never a module-level declaration or a field).
    Array(Box<TyRef>),
}

impl TyRef {
    /// The VBA spelling in a generated `As` clause (the element type for an
    /// array: the parentheses go with the name).
    pub fn spelling(&self) -> String {
        match self {
            TyRef::Array(e) => e.spelling(),
            TyRef::Scalar(t) => type_spelling(t).to_owned(),
            TyRef::Object => "Object".to_owned(),
            TyRef::Builtin(n) | TyRef::LibEnum(n) => n.clone(),
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

    /// Like [`TypeEnv::resolve`], for a local, a parameter or a result: an
    /// array of a scalar element type is accepted.
    pub fn resolve_slot(&self, t: &ResolvedType, module: usize) -> Result<TyRef, String> {
        if let ResolvedType::Array(id) = t {
            return match scalar_type(&ResolvedType::Intrinsic(*id)) {
                Some(e) => Ok(TyRef::Array(Box::new(TyRef::Scalar(e)))),
                None => Err("array of an element type that is not a plain scalar".to_owned()),
            };
        }
        self.resolve(t, module)
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
                let key0 = name.to_ascii_lowercase();
                let shadowed = self.classes.contains_key(&key0)
                    || self.public.contains_key(&key0)
                    || self.own[module].contains_key(&key0);
                if !shadowed {
                    use vba_sema::TypeProvider;
                    let q = qualifier.and_then(vba_pcode::Name::spelling);
                    if let Some(t) = vba_sema::builtins::BuiltinLibrary.resolve_declared(q, name) {
                        match t {
                            vba_sema::Type::Long => return Ok(TyRef::LibEnum(name.to_owned())),
                            vba_sema::Type::Library(lt) => {
                                return Ok(TyRef::Builtin(lt.name.clone()));
                            }
                            _ => {}
                        }
                    }
                }
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
            ResolvedType::Array(_) => Err("array type".to_owned()),
            ResolvedType::FixedString(_) => Err("fixed-length string".to_owned()),
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
    /// `Option Base 0` or `1` when the module has the statement.
    pub option_base: Option<u8>,
    /// The module has a `DefType` statement (the default type of implicit
    /// variables is then not `Variant`).
    pub has_deftype: bool,
    pub decls: Vec<ModuleDecl>,
    pub types: Vec<TypeDef>,
    /// `Implements` statements: line, the stored interface type and, once
    /// resolved, the class it names.
    pub implements: Vec<ImplementsDecl>,
    pub procs: Vec<ProcModel>,
    /// The module name is used as a value (`ClassName.Member`) in the
    /// project, which a class module allows only when it has a default
    /// instance (`VB_PredeclaredId`; attribute lines have no cache line).
    pub predeclared: bool,
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

/// The text of a string literal: ASCII as it is, other bytes decoded with the
/// code page of the project (the encodings `encoding_rs` knows). `None` when
/// the bytes cannot be decoded without loss.
pub(crate) fn decode_text(bytes: &[u8], code_page: Option<u16>) -> Option<String> {
    if bytes.is_ascii() {
        return Some(String::from_utf8_lossy(bytes).into_owned());
    }
    let enc = match code_page? {
        932 => encoding_rs::SHIFT_JIS,
        936 => encoding_rs::GBK,
        949 => encoding_rs::EUC_KR,
        950 => encoding_rs::BIG5,
        10000 => encoding_rs::MACINTOSH,
        n @ (874 | 1250..=1258) => encoding_rs::Encoding::for_label(format!("windows-{n}").as_bytes())?,
        _ => return None,
    };
    let (text, _, bad) = enc.decode(bytes);
    (!bad).then(|| text.into_owned())
}

/// Instructions that leave nothing on the expression stack and carry no
/// meaning in front of a declaration.
fn is_layout_marker(op: Opcode) -> bool {
    matches!(
        op,
        Opcode::StmtBegin
            | Opcode::StmtBeginImplicit
            | Opcode::CommentQuote
            | Opcode::CommentRemKeyword
            | Opcode::LineContinuation
    )
}

/// The text between the parentheses of an array declaration from the
/// instructions in front of its `VAR_DECL`: empty for a dynamic array, else
/// `lower To upper` per dimension (just `upper` where the lower bound is the
/// `Option Base` one, announced by a marker). `None` when a bound is not an
/// expression of literals and names.
fn array_dims(bounds: &[Listed], option_base: Option<u8>) -> Option<String> {
    if bounds.is_empty() {
        return Some(String::new());
    }
    // Trees and markers in stack order.
    enum Item {
        Marker,
        Tree(String),
    }
    let mut items = Vec::new();
    let mut seg: Vec<Listed> = Vec::new();
    let flush = |seg: &mut Vec<Listed>, items: &mut Vec<Item>| -> Option<()> {
        for t in expr_texts(seg)? {
            items.push(Item::Tree(t));
        }
        seg.clear();
        Some(())
    };
    for ins in bounds {
        if ins.opcode == Opcode::OptionBaseMarker {
            flush(&mut seg, &mut items)?;
            items.push(Item::Marker);
        } else {
            seg.push(ins.clone());
        }
    }
    flush(&mut seg, &mut items)?;
    let mut out = Vec::new();
    let mut it = items.into_iter();
    while let Some(first) = it.next() {
        match first {
            Item::Marker => {
                // The lower bound is the `Option Base` one; without a
                // statement it is 0, which the stub gets by default.
                let Some(Item::Tree(u)) = it.next() else {
                    return None;
                };
                let _ = option_base;
                out.push(u);
            }
            Item::Tree(l) => {
                let Some(Item::Tree(u)) = it.next() else {
                    return None;
                };
                out.push(format!("{l} To {u}"));
            }
        }
    }
    Some(out.join(", "))
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
        option_base: None,
        has_deftype: false,
        decls: Vec::new(),
        types: Vec::new(),
        implements: Vec::new(),
        procs: Vec::new(),
        predeclared: false,
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
                Opcode::DeftypeRange => model.has_deftype = true,
                Opcode::OptionStmt => {
                    model.explicit |= ins.modifier == 4;
                    if ins.modifier <= 1 {
                        model.option_base = Some(ins.modifier);
                    }
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

/// The VBA text of a constant's value: one expression of literals, names
/// and operators.
fn const_text(expr: &[Listed]) -> Option<String> {
    let mut t = expr_texts(expr)?;
    if t.len() == 1 { t.pop() } else { None }
}

/// The VBA text of the expressions in a postfix instruction sequence, one
/// per tree left on the stack. Integer literals keep their type (`&` marks
/// the 32-bit forms); every operator application is parenthesised.
fn expr_texts(expr: &[Listed]) -> Option<Vec<String>> {
    let mut st: Vec<String> = Vec::new();
    for ins in expr {
        let first = ins.operands.first();
        match (ins.opcode, first) {
            (Opcode::ConstInt16Decimal, Some(Operand::Int { value, .. })) => {
                st.push(value.to_string());
            }
            (Opcode::ConstInt32Decimal, Some(Operand::Int { value, .. })) => {
                st.push(format!("{value}&"));
            }
            (Opcode::ConstInt16Hex, Some(Operand::Int { value, .. })) if *value >= 0 => {
                st.push(format!("&H{value:X}"));
            }
            (Opcode::ConstInt32Hex, Some(Operand::Int { value, .. })) if *value >= 0 => {
                st.push(format!("&H{value:X}&"));
            }
            (Opcode::ConstFloat64, Some(Operand::Float64(v))) if v.is_finite() => {
                st.push(format!("{v:?}"));
            }
            (Opcode::ConstText, Some(Operand::Text(t))) if t.is_ascii() => {
                let body = String::from_utf8_lossy(t).replace('"', "\"\"");
                st.push(format!("\"{body}\""));
            }
            (Opcode::ConstNullObject, _) => st.push("Nothing".to_owned()),
            (Opcode::ConstKeywordValue, _) => match ins.modifier {
                0 => st.push("False".to_owned()),
                1 => st.push("True".to_owned()),
                _ => return None,
            },
            (
                Opcode::ReadSimple,
                Some(Operand::Name {
                    name,
                    suffix: None,
                    bracketed: false,
                }),
            ) if !matches!(name, Name::Me) => st.push(name.spelling()?.to_owned()),
            (
                Opcode::ReadDot,
                Some(Operand::Name {
                    name,
                    suffix: None,
                    bracketed: false,
                }),
            ) => {
                let l = st.pop()?;
                if !l
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
                {
                    return None;
                }
                st.push(format!("{l}.{}", name.spelling()?));
            }
            (Opcode::NumNegate, _) => {
                let v = st.pop()?;
                if v.starts_with('"') || v == "True" || v == "False" {
                    return None;
                }
                st.push(format!("-{v}"));
            }
            (Opcode::BoolNegate, _) => {
                let v = st.pop()?;
                st.push(format!("(Not {v})"));
            }
            (Opcode::ExprParentheses, _) => {
                let v = st.pop()?;
                st.push(format!("({v})"));
            }
            (op, _) => {
                let sym = match op {
                    Opcode::NumAdd => "+",
                    Opcode::NumSubtract => "-",
                    Opcode::NumMultiply => "*",
                    Opcode::NumDivide => "/",
                    Opcode::NumIntDivide => "\\",
                    Opcode::NumRemainder => "Mod",
                    Opcode::TextJoin => "&",
                    Opcode::BoolConjunction => "And",
                    Opcode::BoolDisjunction => "Or",
                    _ => return None,
                };
                let r = st.pop()?;
                let l = st.pop()?;
                st.push(format!("({l} {sym} {r})"));
            }
        }
    }
    Some(st)
}

/// Renders the default-value expressions of a header line as VBA text, one
/// per argument that declares a default. The expressions are stored in
/// reverse argument order (observed: the last defaulted argument first).
fn default_values(
    info: &ProcedureInfo,
    exprs: Option<Vec<Listed>>,
) -> Result<Vec<Option<String>>, &'static str> {
    let wanted = info.args.iter().filter(|a| a.has_default).count();
    let list = exprs.unwrap_or_default();
    let mut texts = expr_texts(&list).ok_or("default value expression is not supported")?;
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

/// Resolves the fields of the user types of module `mi`; a type with a field
/// that does not resolve gets a problem. Returns whether one was found.
/// Repeated, with the environment rebuilt, until nothing changes.
pub(crate) fn resolve_types(m: &mut ModuleModel, mi: usize, env: &TypeEnv) -> bool {
    let mut changed = false;
    for t in &mut m.types {
        if t.problem.is_some() {
            continue;
        }
        if t.is_enum {
            if t.members.is_empty() {
                t.problem = Some("empty enum".into());
                changed = true;
            }
            continue;
        }
        let mut fields = Vec::new();
        for (n, raw) in &t.raw_fields {
            match env.resolve(raw, mi) {
                Ok(ty) => fields.push((n.clone(), ty)),
                Err(e) => {
                    t.problem = Some(format!("field of {e}"));
                    changed = true;
                    break;
                }
            }
        }
        if t.problem.is_none() && fields.is_empty() {
            t.problem = Some("empty type".into());
            changed = true;
        }
        t.fields = fields;
    }
    changed
}

/// Resolves the types of the declarations and procedures of module `mi`
/// against the type names of the whole project (after `resolve_types`).
pub(crate) fn resolve_module(m: &mut ModuleModel, mi: usize, env: &TypeEnv) {
    for i in &mut m.implements {
        i.class = match env.resolve(&i.raw, mi) {
            Ok(TyRef::Class(n)) => Ok(n),
            Ok(_) => Err("interface that is not a class module of the project".to_owned()),
            Err(e) => Err(e),
        };
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
    let option_base = m.option_base;
    for p in &mut m.procs {
        finish_signature(p, mi, env, option_base);
    }
}

/// Checks the signature and collects the declared locals.
fn finish_signature(p: &mut ProcModel, mi: usize, env: &TypeEnv, option_base: Option<u8>) {
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
    let n_args = info.args.len();
    for (k, a) in info.args.iter().enumerate() {
        if a.param_array {
            // Only the last parameter, an array of `Variant`.
            let ok = k + 1 == n_args
                && !a.optional
                && !a.has_default
                && matches!(
                    env.resolve_slot(&a.ty, mi),
                    Ok(TyRef::Array(e)) if *e == TyRef::Scalar(vba_sema::Type::Variant)
                );
            if !ok {
                p.signature_problem = Some(fail("ParamArray parameter"));
                return;
            }
        }
        if a.has_default && !a.optional {
            p.signature_problem = Some(fail("default value on a required parameter"));
            return;
        }
        if local_spelling(&a.name).is_none() {
            p.signature_problem = Some(fail("parameter whose name does not resolve"));
            return;
        }
        match env.resolve_slot(&a.ty, mi) {
            Ok(t) => arg_tys.push(t),
            Err(e) => {
                p.signature_problem = Some(fail(&format!("parameter of {e}")));
                return;
            }
        }
    }
    if returns_value(info) {
        match env.resolve_slot(&info.return_type, mi) {
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
    // The instructions since the last `Dim` marker: the bounds of an array
    // declaration precede its `VAR_DECL`.
    let mut pending: Vec<Listed> = Vec::new();
    for (l, ins) in &p.body {
        if matches!(ins.opcode, Opcode::DimMarker | Opcode::DimMarkerImplicit) {
            pending.clear();
            continue;
        }
        if ins.opcode != Opcode::VarDecl {
            if !is_layout_marker(ins.opcode) {
                pending.push(ins.clone());
            }
            continue;
        }
        let bounds = std::mem::take(&mut pending);
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
        let ty = match env.resolve_slot(&v.ty, mi) {
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
        let dims = if matches!(ty, TyRef::Array(_)) {
            match array_dims(&bounds, option_base) {
                Some(d) => Some(d),
                None => {
                    p.signature_problem = Some(unsup(
                        ins.opcode,
                        &name,
                        *l,
                        "array bounds that are not constant expressions",
                    ));
                    return;
                }
            }
        } else if !bounds.is_empty() {
            // Bound-like expressions in front of a scalar declaration:
            // `String * n` and the like are not understood.
            p.signature_problem = Some(unsup(ins.opcode, &name, *l, "unexpected declaration form"));
            return;
        } else {
            None
        };
        locals.push(LocalDecl {
            name: n,
            ty,
            dims,
        });
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
