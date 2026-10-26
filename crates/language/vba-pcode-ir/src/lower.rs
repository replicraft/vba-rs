//! Lowering of the instruction stream of one procedure.
//!
//! The instructions of a procedure are replayed on an expression stack
//! (stack semantics as verified in `vba-pcode`); statements consume the
//! stack and emit IR in the order a source lowering would, mirroring the
//! conversions that `vba-sema` and `vba-ir` apply. Anything outside the
//! conservative subset stops lowering with [`Unsupported`].

use std::collections::HashMap;

use vba_ast::{BinaryOp as AstBin, CompareMode as AstCompare};
use vba_ir::{
    BinOp, Block, BlockId, CallArg, Callee, ErrMode, Function, Inst, InstKind, InvokeKind, LateArg,
    Member, Operand as IrOperand, OutItem, Point, RefPlace, Reg, ResumeKind, Slot, SlotRole,
    StmtId, StmtInfo, StoreMode, Terminator, UnOp,
};
use vba_pcode::{Evidence, Listed, Opcode, Operand};
use vba_sema::bound::Constant;
use vba_sema::builtins::BuiltinLibrary;
use vba_sema::{GlobalId, LocalId, LocalKind, ModuleId, ProcId, ProcInfo, Program, Symbol, Type};
use vba_sema::{LibGlobal, LibMember, LibMemberKind, LibraryType, TypeProvider};
use vba_source::Span;
use vba_value::ops::{ArithOp, CompareOp, CompareOptions, LogicOp, Overflow, StringNumberRule};
use vba_value::{VarType, Variant, VbaString};

use crate::error::Unsupported;
use crate::model::CompareStored;

/// Name index of the standard-module procedures that can be called.
#[derive(Debug, Default)]
pub(crate) struct ProcIndex {
    /// Lower-case name to `(module index, proc index, id)`.
    pub by_name: HashMap<String, Vec<(usize, usize, ProcId)>>,
    /// Lower-case names of the procedures and module-level declarations of
    /// the standard modules, lowered or not: visible from everywhere, so
    /// never host members.
    pub known_std: std::collections::HashSet<String>,
    /// `[module index]`: lower-case names of all procedures and module-level
    /// declarations of the module, lowered or not.
    pub members: Vec<std::collections::HashSet<String>>,
}

/// An expression tree built from the stack; IR is emitted when a statement
/// consumes it.
#[derive(Debug, Clone)]
enum Ex {
    Const(Variant, Type),
    Local {
        id: LocalId,
        ty: Type,
        /// A plain variable read (eligible for `ByRef` passing).
        bare: bool,
    },
    Unary {
        op: UnOp,
        ty: Type,
        e: Box<Ex>,
    },
    Binary {
        op: AstBin,
        ty: Type,
        l: Box<Ex>,
        r: Box<Ex>,
    },
    Convert {
        e: Box<Ex>,
        to: Type,
    },
    Call {
        callee: Callee,
        ty: Type,
        args: Vec<ArgEx>,
        /// The callee is a member of the current class instance (`Me`).
        on_me: bool,
    },
    /// A module-level variable (a static variable of a standard module, a
    /// field of the instance in a class-like one).
    Global {
        id: GlobalId,
        ty: Type,
        /// Read of a plain variable (eligible for `ByRef` passing).
        bare: bool,
    },
    /// `Me`.
    Me(ModuleId),
    /// An omitted positional argument; only valid inside an argument list.
    Missing,
    /// A member of the host base of a document or designer module (a
    /// control, a base-class member), or a member of such a value: bound
    /// late, as the source binding does.
    Late {
        recv: Box<Ex>,
        name: String,
        args: Vec<LateEx>,
    },
    Paren(Box<Ex>),
    /// A global object of the built-in library (`Err`).
    LibObj {
        name: String,
        lt: LibraryType,
    },
    /// A property read on a library object (`Err.Number`).
    Member {
        recv: Box<Ex>,
        name: String,
        ty: Type,
        lt: LibraryType,
        dispid: Option<i32>,
    },
}

/// A variable that can be passed by reference.
#[derive(Debug, Clone, Copy)]
enum RefTarget {
    Local(LocalId),
    Global(GlobalId),
}

#[derive(Debug, Clone)]
enum ArgEx {
    Val(Ex),
    Ref(RefTarget),
    Missing,
}

/// An argument of a late-bound call: variables travel by reference,
/// everything else by value, nothing is converted.
#[derive(Debug, Clone)]
enum LateEx {
    Val(Ex),
    Ref(RefTarget),
    Missing,
}

/// What a name is looked up for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Read, or call as a statement: `Sub`, `Function`, `Property Get`.
    Read,
    /// Assignment: `Property Let`.
    Let,
}

impl Ex {
    fn ty(&self) -> Type {
        match self {
            Ex::Const(_, t) | Ex::Local { ty: t, .. } => t.clone(),
            Ex::Unary { ty, .. }
            | Ex::Binary { ty, .. }
            | Ex::Call { ty, .. }
            | Ex::Global { ty, .. } => ty.clone(),
            Ex::Me(m) => Type::Class(*m),
            Ex::Late { .. } | Ex::Missing => Type::Variant,
            Ex::Convert { to, .. } => to.clone(),
            Ex::Paren(e) => e.ty(),
            Ex::LibObj { lt, .. } => Type::Library(lt.clone()),
            Ex::Member { ty, .. } => ty.clone(),
        }
    }
}

#[derive(Debug)]
enum Frame {
    If {
        end: BlockId,
        else_b: BlockId,
        has_else: bool,
    },
    For {
        head: BlockId,
        body: BlockId,
        next_b: BlockId,
        exit: BlockId,
        counter: LocalId,
        counter_ty: Type,
        step: IrOperand,
    },
    Loop {
        head: BlockId,
        body: BlockId,
        exit: BlockId,
    },
    Select {
        stmt: StmtId,
        /// The first case body was reached (its start is the resume point).
        first_body: bool,
        subject: IrOperand,
        subject_ty: Type,
        end: BlockId,
        /// Where the comparisons of the next case are emitted.
        chain: BlockId,
        /// Body block of the case being built or in progress.
        body: Option<BlockId>,
        /// Block after the current case's last item.
        miss: Option<BlockId>,
        /// The current case body is open (a `Case` or `End Select` closes it).
        in_body: bool,
        in_else: bool,
    },
}

struct BlockBuf {
    insts: Vec<Inst>,
    term: Option<Terminator>,
    term_stmt: StmtId,
}

/// Context needed to lower one procedure.
pub(crate) struct ProcCtx<'a> {
    pub program: &'a Program,
    pub proc_id: ProcId,
    pub index: &'a ProcIndex,
    pub module: usize,
    /// `[module index]`: whether the module is class-like.
    pub class_like: &'a [bool],
    /// The module's members that the host provides (document and designer
    /// modules): unresolved names are late bound on `Me`.
    pub host_backed: bool,
    /// Names the embedding application provides as members of the host
    /// base (lower case): a bare read of one of them is a late-bound read.
    pub host_members: &'a std::collections::HashSet<String>,
    pub compare: CompareStored,
    pub name: &'a str,
}

/// The result of lowering one procedure.
pub(crate) struct Lowered {
    pub function: Function,
    pub callees: Vec<ProcId>,
}

struct Lw<'a> {
    cx: &'a ProcCtx<'a>,
    sema: &'a ProcInfo,
    regs: Vec<Type>,
    blocks: Vec<BlockBuf>,
    stmts: Vec<StmtInfo>,
    cur: BlockId,
    cur_stmt: StmtId,
    stack: Vec<Ex>,
    frames: Vec<Frame>,
    callees: Vec<ProcId>,
    /// Stack depth at `FOR_VAR_EXPR_OPEN`, while between the markers.
    for_depth: Option<usize>,
    /// The counter captured at `FOR_VAR_EXPR_CLOSE`.
    for_var: Option<Ex>,
    /// Pending `Debug.Print` list.
    print: Option<(Vec<Ex>, Vec<OutItem>)>,
    line: usize,
    stmt_line: usize,
    /// The procedure contains `On Error`: statements need resume points.
    has_on_error: bool,
    /// Blocks of the labels (lower-case name), created on first mention.
    labels: HashMap<String, BlockId>,
    /// Labels that were defined by a `LABEL_NAMED`.
    defined: Vec<String>,
    /// Local constants (lower-case name): folded where they are read.
    local_consts: HashMap<String, (Variant, Type)>,
}

type R<T> = Result<T, Unsupported>;

const SPAN: Span = Span { start: 0, end: 0 };

fn is_layout_only(op: Opcode) -> bool {
    matches!(
        op,
        Opcode::CommentQuote
            | Opcode::CommentRemKeyword
            | Opcode::LineContinuation
            | Opcode::StmtBegin
            | Opcode::StmtBeginImplicit
    )
}

fn var_type_to_type(v: VarType) -> Type {
    match v {
        VarType::Integer => Type::Integer,
        VarType::Long => Type::Long,
        VarType::Single => Type::Single,
        VarType::Double => Type::Double,
        VarType::Currency => Type::Currency,
        VarType::Date => Type::Date,
        VarType::String => Type::String,
        VarType::Boolean => Type::Boolean,
        VarType::Decimal => Type::Decimal,
        VarType::Byte => Type::Byte,
        VarType::LongLong => Type::LongLong,
        _ => Type::Variant,
    }
}

fn overflow_for(l: &Type, r: &Type) -> Overflow {
    if l.is_variant() || r.is_variant() {
        Overflow::Promote
    } else {
        Overflow::Error
    }
}

fn compare_op(op: AstBin) -> CompareOp {
    match op {
        AstBin::Eq => CompareOp::Eq,
        AstBin::Ne => CompareOp::Ne,
        AstBin::Lt => CompareOp::Lt,
        AstBin::Gt => CompareOp::Gt,
        AstBin::Le => CompareOp::Le,
        _ => CompareOp::Ge,
    }
}

/// Lowers the body of a procedure.
pub(crate) fn lower_body(
    cx: &ProcCtx<'_>,
    body: &[(usize, Listed)],
    header_line: usize,
    kind_template: &Function,
) -> R<Lowered> {
    let sema = cx.program.proc(cx.proc_id);
    let mut lw = Lw {
        cx,
        sema,
        regs: Vec::new(),
        blocks: vec![BlockBuf {
            insts: Vec::new(),
            term: None,
            term_stmt: StmtId(0),
        }],
        stmts: Vec::new(),
        cur: BlockId(0),
        cur_stmt: StmtId(0),
        stack: Vec::new(),
        frames: Vec::new(),
        callees: Vec::new(),
        for_depth: None,
        for_var: None,
        print: None,
        line: header_line,
        stmt_line: usize::MAX,
        has_on_error: body.iter().any(|(_, i)| i.opcode == Opcode::ErrHandlerSet),
        labels: HashMap::new(),
        defined: Vec::new(),
        local_consts: HashMap::new(),
    };
    lw.begin_stmt();
    let mut last_op = Opcode::ProcHeader;
    let mut i = 0;
    while i < body.len() {
        let (line, ins) = &body[i];
        lw.line = *line;
        last_op = ins.opcode;
        if *line != lw.stmt_line {
            lw.begin_stmt();
            lw.stmt_line = *line;
        }
        let consumed = lw.step(body, i)?;
        i += consumed;
    }
    if !lw.frames.is_empty() {
        return Err(lw.no(last_op, "unterminated block structure"));
    }
    if !lw.stack.is_empty() {
        return Err(lw.no(last_op, "values left on the expression stack"));
    }
    if let Some(missing) = lw.labels.keys().find(|k| !lw.defined.contains(k)) {
        let reason = format!("label `{missing}` is not defined in the procedure");
        return Err(lw.no(last_op, reason));
    }
    lw.terminate(Terminator::Return);
    Ok(lw.finish(kind_template))
}

impl Lw<'_> {
    fn no(&self, op: Opcode, reason: impl Into<String>) -> Unsupported {
        Unsupported {
            opcode: op,
            procedure: self.cx.name.to_owned(),
            line: self.line,
            reason: reason.into(),
        }
    }

    // ---- building ---------------------------------------------------------

    fn reg(&mut self, ty: Type) -> Reg {
        self.regs.push(ty);
        Reg(self.regs.len() as u32 - 1)
    }

    fn new_block(&mut self) -> BlockId {
        self.blocks.push(BlockBuf {
            insts: Vec::new(),
            term: None,
            term_stmt: StmtId(0),
        });
        BlockId(self.blocks.len() as u32 - 1)
    }

    fn switch_to(&mut self, b: BlockId) {
        self.cur = b;
    }

    fn begin_stmt(&mut self) {
        self.stmts.push(StmtInfo {
            span: SPAN,
            line: None,
            resume_next: None,
        });
        self.cur_stmt = StmtId(self.stmts.len() as u32 - 1);
    }

    fn emit(&mut self, kind: InstKind) {
        let inst = Inst {
            span: SPAN,
            stmt: self.cur_stmt,
            kind,
        };
        self.blocks[self.cur.0 as usize].insts.push(inst);
    }

    /// The block of a label, created on first use.
    fn label_block(&mut self, name: &str) -> BlockId {
        let key = name.to_ascii_lowercase();
        if let Some(b) = self.labels.get(&key) {
            return *b;
        }
        let b = self.new_block();
        self.labels.insert(key, b);
        b
    }

    fn is_terminated(&self) -> bool {
        self.blocks[self.cur.0 as usize].term.is_some()
    }

    fn terminate(&mut self, t: Terminator) {
        if !self.is_terminated() {
            let stmt = self.cur_stmt;
            let b = &mut self.blocks[self.cur.0 as usize];
            b.term = Some(t);
            b.term_stmt = stmt;
        }
    }

    fn here(&self) -> Point {
        Point {
            block: self.cur,
            index: self.blocks[self.cur.0 as usize].insts.len() as u32,
        }
    }

    /// Ends a simple statement: `Resume Next` continues right here.
    fn close_stmt(&mut self) {
        let p = self.here();
        let s = &mut self.stmts[self.cur_stmt.0 as usize];
        if s.resume_next.is_none() {
            s.resume_next = Some(p);
        }
    }

    /// `Resume Next` of the current statement continues at the start of `b`.
    fn resume_at(&mut self, b: BlockId) {
        self.stmts[self.cur_stmt.0 as usize].resume_next = Some(Point { block: b, index: 0 });
    }

    fn jump_to(&mut self, b: BlockId) {
        self.terminate(Terminator::Jump(b));
        self.switch_to(b);
    }

    fn terminate_and_continue(&mut self, t: Terminator) {
        self.terminate(t);
        let n = self.new_block();
        self.switch_to(n);
    }

    fn finish(mut self, template: &Function) -> Lowered {
        let blocks = std::mem::take(&mut self.blocks);
        let blocks: Vec<Block> = blocks
            .into_iter()
            .map(|b| Block {
                insts: b.insts,
                term: b.term.unwrap_or(Terminator::Unreachable),
                term_stmt: b.term_stmt,
                term_span: SPAN,
            })
            .collect();
        let slots: Vec<Slot> = template
            .slots
            .iter()
            .map(|s| Slot {
                name: s.name.clone(),
                ty: s.ty.clone(),
                role: match s.role {
                    SlotRole::Param(i) => SlotRole::Param(i),
                    SlotRole::Result => SlotRole::Result,
                    SlotRole::Local => SlotRole::Local,
                    SlotRole::Temp => SlotRole::Temp,
                },
            })
            .collect();
        Lowered {
            function: Function {
                kind: template.kind,
                name: template.name.clone(),
                slots,
                regs: self.regs,
                blocks,
                stmts: self.stmts,
            },
            callees: self.callees,
        }
    }

    // ---- types and conversions --------------------------------------------

    fn compare_ast(&self) -> AstCompare {
        match self.cx.compare {
            CompareStored::Binary => AstCompare::Binary,
            CompareStored::Text => AstCompare::Text,
            CompareStored::Database => AstCompare::Database,
        }
    }

    fn compare_opts(&self, l: &Type, r: &Type) -> CompareOptions {
        let mode = match self.compare_ast() {
            AstCompare::Binary => vba_value::CompareMode::Binary,
            AstCompare::Text | AstCompare::Database => vba_value::CompareMode::Text,
        };
        let numeric = |t: &Type| t.is_numeric() || matches!(t, Type::Boolean | Type::Date);
        let coerce = (l.is_string() && numeric(r)) || (numeric(l) && r.is_string());
        CompareOptions {
            mode,
            string_number: if coerce {
                StringNumberRule::Coerce
            } else {
                StringNumberRule::VariantOrdering
            },
        }
    }

    /// Converts `e` to `to` as an assignment would: constants are converted
    /// at lowering time when exact, other values get a `Convert`.
    fn coerce(&self, e: Ex, to: &Type) -> Ex {
        let from = e.ty();
        if from == *to {
            return e;
        }
        if let Ex::Const(v, _) = &e {
            if *to == Type::Variant {
                return Ex::Const(v.clone(), Type::Variant);
            }
            if let Some(vt) = to.var_type()
                && !matches!(to, Type::Udt(_) | Type::Array { .. } | Type::FixedString(_))
                && !to.is_object()
                && let Ok(c) = vba_value::convert::convert_to(v, vt)
            {
                return Ex::Const(c, to.clone());
            }
        }
        Ex::Convert {
            e: Box::new(e),
            to: to.clone(),
        }
    }

    fn coerce_condition(&self, e: Ex) -> Ex {
        match e.ty() {
            Type::Boolean | Type::Variant => e,
            _ => self.coerce(e, &Type::Boolean),
        }
    }

    // ---- name resolution -------------------------------------------------

    fn local_by_name(&self, name: &str) -> Option<(LocalId, Type)> {
        self.sema.locals.iter().enumerate().find_map(|(i, l)| {
            (matches!(l.kind, LocalKind::Param(_) | LocalKind::Var)
                && l.name.to_string().eq_ignore_ascii_case(name))
            .then(|| (LocalId(i as u32), l.ty.clone()))
        })
    }

    fn is_result_name(&self, name: &str) -> bool {
        self.sema.has_result && self.sema.name.to_string().eq_ignore_ascii_case(name)
    }

    /// A procedure of the project by name: the current module's own first,
    /// then the unique one of the standard modules. Members of other
    /// class-like modules are not reachable by a bare name.
    fn proc_target(&mut self, name: &str, role: Role) -> Option<ProcId> {
        use vba_ast::ProcKind as K;
        let list = self.cx.index.by_name.get(&name.to_ascii_lowercase())?;
        let fits = |id: ProcId| {
            let k = self.cx.program.proc(id).kind;
            match role {
                Role::Read => matches!(k, K::Sub | K::Function | K::PropertyGet),
                Role::Let => k == K::PropertyLet,
            }
        };
        let same: Vec<_> = list
            .iter()
            .filter(|(m, _, id)| *m == self.cx.module && fits(*id))
            .collect();
        let pick = if same.len() == 1 {
            same[0].2
        } else if same.is_empty() {
            let others: Vec<_> = list
                .iter()
                .filter(|(m, _, id)| *m != self.cx.module && !self.cx.class_like[*m] && fits(*id))
                .collect();
            if others.len() == 1 {
                others[0].2
            } else {
                return None;
            }
        } else {
            return None;
        };
        if !self.callees.contains(&pick) {
            self.callees.push(pick);
        }
        Some(pick)
    }

    /// A module-scope variable or constant visible to the procedure: the
    /// module's own, or a public one of a standard module.
    fn module_symbol(&self, name: &str) -> Option<Symbol> {
        let key = vba_ast::Name::new(name);
        let own = self.cx.program.module(self.sema.module);
        if let Some((sym, _)) = own.symbols.get(&key) {
            return Some(*sym);
        }
        let mut found = None;
        for m in &self.cx.program.modules {
            if m.id == own.id || m.is_class_like() {
                continue;
            }
            if let Some((sym, _)) = m.symbols.get(&key)
                && matches!(sym, Symbol::Var(_) | Symbol::Const(_))
            {
                if found.is_some() {
                    return None;
                }
                found = Some(*sym);
            }
        }
        found
    }

    /// Whether an unresolved name is a member of the host base. A call, a
    /// statement and the receiver of a member access always are; a bare
    /// value read only when the embedding application lists the name
    /// (otherwise the source binding reads an implicit variable or reports
    /// an undeclared one).
    fn is_host_name(&self, name: &str, bare_read: bool) -> bool {
        self.cx.host_backed
            && !name.starts_with(crate::model::PLACEHOLDER)
            && (!bare_read || self.cx.host_members.contains(&name.to_ascii_lowercase()))
            && self.module_symbol(name).is_none()
            && !self.cx.index.known_std.contains(&name.to_ascii_lowercase())
            && !self.cx.index.members[self.cx.module].contains(&name.to_ascii_lowercase())
            && vba_stdlib::lookup(name).is_none()
            && BuiltinLibrary.global(name).is_none()
    }

    fn me(&self) -> Ex {
        Ex::Me(self.sema.module)
    }

    fn global_ex(&self, g: GlobalId) -> Ex {
        let info = self.cx.program.global(g);
        Ex::Global {
            id: g,
            ty: info.ty.clone(),
            bare: true,
        }
    }

    fn name_of(&self, ins: &Listed) -> R<(String, Option<char>)> {
        match ins.operands.first() {
            Some(Operand::Name {
                name,
                suffix,
                bracketed: _,
            }) => match crate::model::local_spelling(name) {
                Some(s) if !matches!(name, vba_pcode::Name::Me) => Ok((s, *suffix)),
                _ => Err(self.no(ins.opcode, "name does not resolve")),
            },
            _ => Err(self.no(ins.opcode, "missing name operand")),
        }
    }

    // ---- stack helpers -----------------------------------------------------

    fn pop(&mut self, ins: &Listed) -> R<Ex> {
        match self.stack.pop() {
            Some(Ex::Missing) => Err(self.no(ins.opcode, "omitted argument outside a call")),
            Some(e) => Ok(e),
            None => Err(self.no(ins.opcode, "expression stack underflow")),
        }
    }

    fn pop_n(&mut self, ins: &Listed, n: usize) -> R<Vec<Ex>> {
        if self.stack.len() < n {
            return Err(self.no(ins.opcode, "expression stack underflow"));
        }
        let at = self.stack.len() - n;
        Ok(self.stack.split_off(at))
    }

    // ---- calls ------------------------------------------------------------

    fn project_call(&mut self, ins: &Listed, id: ProcId, args: Vec<Ex>) -> R<Ex> {
        let callee = self.cx.program.proc(id);
        let given = args.len();
        if given > callee.params.len()
            || callee.params[given..]
                .iter()
                .any(|p| !p.optional || p.param_array)
        {
            return Err(self.no(ins.opcode, "argument count differs from the parameter list"));
        }
        let mut out = Vec::new();
        for (p, a) in callee.params.iter().zip(args) {
            if matches!(a, Ex::Missing) {
                if !p.optional {
                    return Err(self.no(ins.opcode, "omitted argument for a required parameter"));
                }
                out.push(ArgEx::Missing);
                continue;
            }
            let forced = matches!(a, Ex::Paren(_));
            if p.by_val || forced {
                out.push(ArgEx::Val(self.coerce(a, &p.ty)));
                continue;
            }
            let target = match &a {
                Ex::Local {
                    id: lid,
                    ty,
                    bare: true,
                } => Some((RefTarget::Local(*lid), ty)),
                Ex::Global {
                    id: gid,
                    ty,
                    bare: true,
                } => Some((RefTarget::Global(*gid), ty)),
                _ => None,
            };
            if let Some((t, ty)) = target {
                if *ty == p.ty {
                    out.push(ArgEx::Ref(t));
                    continue;
                }
                if p.ty != Type::Variant {
                    return Err(self.no(ins.opcode, "ByRef argument of a different type"));
                }
            }
            out.push(ArgEx::Val(self.coerce(a, &p.ty)));
        }
        out.resize(callee.params.len(), ArgEx::Missing);
        let on_me = self.cx.program.module(callee.module).is_class_like();
        Ok(Ex::Call {
            callee: Callee::Proc(id),
            ty: if callee.has_result {
                callee.ret.clone()
            } else {
                Type::Variant
            },
            args: out,
            on_me,
        })
    }

    fn intrinsic_call(
        &mut self,
        ins: &Listed,
        name: &str,
        dollar: bool,
        mut args: Vec<Ex>,
    ) -> R<Ex> {
        let lookup = if dollar {
            format!("{name}$")
        } else {
            name.to_owned()
        };
        let Some(r) = vba_stdlib::lookup(&lookup) else {
            if !dollar && let Some(LibGlobal::Function(m)) = BuiltinLibrary.global(name) {
                return self.runtime_call(ins, &m, args);
            }
            return Err(self.no(
                ins.opcode,
                format!("`{name}` is not a known library function"),
            ));
        };
        let info = r.info;
        if info.params.iter().any(|p| p.param_array) {
            return Err(self.no(ins.opcode, "ParamArray function"));
        }
        let is_instr = matches!(info.name, "InStr" | "InStrB");
        let mut lead_missing = false;
        if is_instr {
            match args.len() {
                2 => lead_missing = true,
                // `start, string1, string2`: the form the source binding
                // gives three arguments.
                3 | 4 => {}
                _ => {
                    return Err(self.no(ins.opcode, "InStr with an unusual argument count"));
                }
            }
        }
        let total = args.len() + usize::from(lead_missing);
        if total > info.params.len() || args.len() < info.min_args() - usize::from(lead_missing) {
            return Err(self.no(ins.opcode, "argument count does not fit the signature"));
        }
        let mut out = Vec::new();
        if lead_missing {
            out.push(ArgEx::Missing);
        }
        let skip = usize::from(lead_missing);
        for (k, a) in args.drain(..).enumerate() {
            let p = &info.params[k + skip];
            if matches!(a, Ex::Missing) {
                if !p.optional {
                    return Err(self.no(ins.opcode, "omitted argument for a required parameter"));
                }
                out.push(ArgEx::Missing);
                continue;
            }
            let pty = var_type_to_type(p.ty);
            if pty.is_object() {
                return Err(self.no(ins.opcode, "object parameter"));
            }
            out.push(ArgEx::Val(self.coerce(a, &pty)));
        }
        while out.len() < info.params.len() {
            let p = &info.params[out.len()];
            if !p.optional {
                return Err(self.no(ins.opcode, "missing required argument"));
            }
            out.push(ArgEx::Missing);
        }
        let ty = if dollar && info.dollar {
            Type::String
        } else {
            var_type_to_type(info.returns)
        };
        Ok(Ex::Call {
            callee: Callee::Intrinsic {
                name: info.name.to_owned(),
                dollar: r.dollar,
            },
            ty,
            args: out,
            on_me: false,
        })
    }

    /// A call of a function that the run time provides (`MsgBox`, `Beep`,
    /// ...): arguments are converted to the parameter types and omitted
    /// optional ones are passed as missing, as the source binding does.
    fn runtime_call(&mut self, ins: &Listed, m: &LibMember, args: Vec<Ex>) -> R<Ex> {
        if m.params.iter().any(|p| p.param_array || p.ty.is_object()) || m.ty.is_object() {
            return Err(self.no(ins.opcode, "object or ParamArray run-time function"));
        }
        if args.len() > m.params.len() {
            return Err(self.no(ins.opcode, "too many arguments"));
        }
        let mut out = Vec::new();
        for (p, a) in m.params.iter().zip(args) {
            if matches!(a, Ex::Missing) {
                if !p.optional {
                    return Err(self.no(ins.opcode, "omitted argument for a required parameter"));
                }
                out.push(ArgEx::Missing);
                continue;
            }
            out.push(ArgEx::Val(self.coerce(a, &p.ty)));
        }
        while out.len() < m.params.len() {
            if !m.params[out.len()].optional {
                return Err(self.no(ins.opcode, "missing required argument"));
            }
            out.push(ArgEx::Missing);
        }
        Ok(Ex::Call {
            callee: Callee::Intrinsic {
                name: m.name.clone(),
                dollar: false,
            },
            ty: if m.is_sub {
                Type::Variant
            } else {
                m.ty.clone()
            },
            args: out,
            on_me: false,
        })
    }

    // ---- emission ---------------------------------------------------------

    fn emit_ex(&mut self, e: &Ex) -> IrOperand {
        match e {
            Ex::Const(v, _) => IrOperand::Const(Constant::Value(v.clone())),
            Ex::Local { id, ty, .. } => {
                let dst = self.reg(ty.clone());
                self.emit(InstKind::LoadLocal { dst, slot: *id });
                IrOperand::Reg(dst)
            }
            Ex::Paren(inner) => self.emit_ex(inner),
            Ex::LibObj { name, lt } => {
                let dst = self.reg(Type::Library(lt.clone()));
                self.emit(InstKind::LoadLibGlobal {
                    dst,
                    name: name.clone(),
                });
                IrOperand::Reg(dst)
            }
            Ex::Member {
                recv,
                name,
                ty,
                lt,
                dispid,
            } => {
                let r = self.emit_ex(recv);
                let dst = self.reg(ty.clone());
                self.emit(InstKind::Invoke {
                    dst: Some(dst),
                    receiver: r,
                    member: Member::Named(name.clone()),
                    kind: InvokeKind::Get,
                    args: Vec::new(),
                    static_receiver: Some(lt.clone()),
                    dispid: *dispid,
                });
                IrOperand::Reg(dst)
            }
            Ex::Unary { op, ty, e: inner } => {
                let it = inner.ty();
                let o = self.emit_ex(inner);
                let dst = self.reg(ty.clone());
                self.emit(InstKind::Unary {
                    dst,
                    op: *op,
                    ty: ty.clone(),
                    overflow: if it.is_variant() {
                        Overflow::Promote
                    } else {
                        Overflow::Error
                    },
                    src: o,
                });
                IrOperand::Reg(dst)
            }
            Ex::Binary { op, ty, l, r } => {
                let (lt, rt) = (l.ty(), r.ty());
                let lo = self.emit_ex(l);
                let ro = self.emit_ex(r);
                let dst = self.reg(ty.clone());
                let bop = match op {
                    AstBin::Add => BinOp::Arith(ArithOp::Add),
                    AstBin::Sub => BinOp::Arith(ArithOp::Sub),
                    AstBin::Mul => BinOp::Arith(ArithOp::Mul),
                    AstBin::Div => BinOp::Arith(ArithOp::Div),
                    AstBin::IntDiv => BinOp::Arith(ArithOp::IntDiv),
                    AstBin::Mod => BinOp::Arith(ArithOp::Mod),
                    AstBin::Pow => BinOp::Arith(ArithOp::Pow),
                    AstBin::Concat => BinOp::Concat,
                    AstBin::Eq | AstBin::Ne | AstBin::Lt | AstBin::Gt | AstBin::Le | AstBin::Ge => {
                        BinOp::Compare(compare_op(*op), self.compare_opts(&lt, &rt))
                    }
                    AstBin::Like => BinOp::Like(self.compare_ast()),
                    AstBin::Is => BinOp::Is,
                    AstBin::And => BinOp::Logic(LogicOp::And),
                    AstBin::Or => BinOp::Logic(LogicOp::Or),
                    AstBin::Xor => BinOp::Logic(LogicOp::Xor),
                    AstBin::Eqv => BinOp::Logic(LogicOp::Eqv),
                    AstBin::Imp => BinOp::Logic(LogicOp::Imp),
                };
                self.emit(InstKind::Binary {
                    dst,
                    op: bop,
                    ty: ty.clone(),
                    overflow: overflow_for(&lt, &rt),
                    lhs: lo,
                    rhs: ro,
                    guard: None,
                });
                IrOperand::Reg(dst)
            }
            Ex::Convert { e: inner, to } => {
                let o = self.emit_ex(inner);
                let dst = self.reg(to.clone());
                self.emit(InstKind::Convert {
                    dst,
                    src: o,
                    to: to.clone(),
                });
                IrOperand::Reg(dst)
            }
            Ex::Call {
                callee,
                ty,
                args,
                on_me,
            } => {
                let dst = self.emit_call(callee, args, *on_me, Some(ty));
                IrOperand::Reg(dst.unwrap_or(Reg(0)))
            }
            Ex::Global { id, ty, .. } => {
                let dst = self.reg(ty.clone());
                self.emit_global_load(*id, dst);
                IrOperand::Reg(dst)
            }
            Ex::Missing => IrOperand::Const(Constant::Value(Variant::Empty)),
            Ex::Me(m) => {
                let dst = self.reg(Type::Class(*m));
                self.emit(InstKind::LoadMe { dst });
                IrOperand::Reg(dst)
            }
            Ex::Late { recv, name, args } => {
                let dst = self.emit_late(recv, name, args, InvokeKind::Get, None, true);
                IrOperand::Reg(dst.unwrap_or(Reg(0)))
            }
        }
    }

    fn local_ty(&self, l: LocalId) -> Type {
        self.sema.locals[l.index()].ty.clone()
    }

    fn emit_global_load(&mut self, g: GlobalId, dst: Reg) {
        if self.global_in_instance(g) {
            let obj = self.emit_me();
            self.emit(InstKind::LoadField {
                dst,
                obj,
                global: g,
            });
        } else {
            self.emit(InstKind::LoadGlobal { dst, global: g });
        }
    }

    fn emit_global_store(&mut self, g: GlobalId, src: IrOperand) {
        if self.global_in_instance(g) {
            let obj = self.emit_me();
            self.emit(InstKind::StoreField {
                obj,
                global: g,
                src,
                mode: StoreMode::Let,
            });
        } else {
            self.emit(InstKind::StoreGlobal {
                global: g,
                src,
                mode: StoreMode::Let,
            });
        }
    }

    /// Whether the variable lives in the class instance.
    fn global_in_instance(&self, g: GlobalId) -> bool {
        self.cx
            .program
            .module(self.cx.program.global(g).module)
            .is_class_like()
    }

    fn emit_me(&mut self) -> IrOperand {
        let dst = self.reg(Type::Class(self.sema.module));
        self.emit(InstKind::LoadMe { dst });
        IrOperand::Reg(dst)
    }

    fn emit_ref(&mut self, t: RefTarget) -> Reg {
        match t {
            RefTarget::Local(l) => {
                let r = self.reg(self.local_ty(l));
                self.emit(InstKind::MakeRef {
                    dst: r,
                    place: RefPlace::Local(l),
                });
                r
            }
            RefTarget::Global(g) => {
                let ty = self.cx.program.global(g).ty.clone();
                let place = if self.global_in_instance(g) {
                    RefPlace::Field {
                        obj: self.emit_me(),
                        global: g,
                    }
                } else {
                    RefPlace::Global(g)
                };
                let r = self.reg(ty);
                self.emit(InstKind::MakeRef { dst: r, place });
                r
            }
        }
    }

    /// Emits a call of a project procedure (receiver first, as the source
    /// lowering does), a library function or a run-time function. The
    /// result register, when wanted, is allocated after the arguments.
    fn emit_call(
        &mut self,
        callee: &Callee,
        args: &[ArgEx],
        on_me: bool,
        result: Option<&Type>,
    ) -> Option<Reg> {
        let receiver = on_me.then(|| self.emit_me());
        let mut out = Vec::new();
        for a in args {
            out.push(match a {
                ArgEx::Val(v) => CallArg::Val(self.emit_ex(v)),
                ArgEx::Missing => CallArg::Missing,
                ArgEx::Ref(t) => CallArg::Ref(self.emit_ref(*t)),
            });
        }
        let dst = result.map(|t| self.reg(t.clone()));
        self.emit(InstKind::Call {
            dst,
            callee: callee.clone(),
            receiver,
            args: out,
        });
        dst
    }

    /// Emits a late-bound member access. For `Put`, `value` is the
    /// assigned value (passed last). The result register, when wanted, is
    /// allocated after the receiver and the arguments.
    fn emit_late(
        &mut self,
        recv: &Ex,
        name: &str,
        args: &[LateEx],
        kind: InvokeKind,
        value: Option<IrOperand>,
        want: bool,
    ) -> Option<Reg> {
        let receiver = self.emit_ex(recv);
        let mut out = Vec::new();
        for a in args {
            out.push(LateArg {
                name: None,
                arg: match a {
                    LateEx::Val(v) => CallArg::Val(self.emit_ex(v)),
                    LateEx::Ref(t) => CallArg::Ref(self.emit_ref(*t)),
                    LateEx::Missing => CallArg::Missing,
                },
            });
        }
        if let Some(v) = value {
            out.push(LateArg {
                name: None,
                arg: CallArg::Val(v),
            });
        }
        let dst = want.then(|| self.reg(Type::Variant));
        self.emit(InstKind::Invoke {
            dst,
            receiver,
            member: Member::Named(name.to_owned()),
            kind,
            args: out,
            static_receiver: None,
            dispid: None,
        });
        dst
    }

    /// Emits a statement-level call: the result, if any, is discarded.
    fn emit_call_stmt(&mut self, e: &Ex) {
        match e {
            Ex::Call {
                callee,
                args,
                on_me,
                ..
            } => {
                self.emit_call(callee, args, *on_me, None);
            }
            Ex::Late { recv, name, args } => {
                self.emit_late(recv, name, args, InvokeKind::Get, None, false);
            }
            _ => {}
        }
    }

    fn cmp(
        &mut self,
        op: CompareOp,
        l: &IrOperand,
        lt: &Type,
        r: IrOperand,
        rt: &Type,
    ) -> IrOperand {
        let ty = if lt.is_variant() || rt.is_variant() {
            Type::Variant
        } else {
            Type::Boolean
        };
        let dst = self.reg(ty.clone());
        let opts = self.compare_opts(lt, rt);
        self.emit(InstKind::Binary {
            dst,
            op: BinOp::Compare(op, opts),
            ty,
            overflow: overflow_for(lt, rt),
            lhs: l.clone(),
            rhs: r,
            guard: None,
        });
        IrOperand::Reg(dst)
    }

    // ---- the instruction step ------------------------------------------------

    /// Processes the instruction at `i`; returns how many were consumed.
    fn step(&mut self, body: &[(usize, Listed)], i: usize) -> R<usize> {
        let ins = &body[i].1;
        let op = ins.opcode;
        if is_layout_only(op) {
            // A statement marker starts a new statement of its own (needed
            // for `Resume Next` after the statement).
            if self.has_on_error && matches!(op, Opcode::StmtBegin | Opcode::StmtBeginImplicit) {
                self.begin_stmt();
            }
            return Ok(1);
        }
        if op.evidence() != Evidence::Verified && !self.accepted_without_verification(ins) {
            return Err(self.no(op, format!("opcode is not verified ({:?})", op.evidence())));
        }
        match op {
            // ---- literals ----
            Opcode::ConstInt16Decimal => {
                let v = self.int_operand(ins)?;
                let v = i16::try_from(v).map_err(|_| self.no(op, "literal out of range"))?;
                self.stack
                    .push(Ex::Const(Variant::Integer(v), Type::Integer));
            }
            Opcode::ConstInt32Decimal => {
                let v = self.int_operand(ins)?;
                let v = i32::try_from(v).map_err(|_| self.no(op, "literal out of range"))?;
                self.stack.push(Ex::Const(Variant::Long(v), Type::Long));
            }
            Opcode::ConstInt32Hex => {
                // The stored value is unsigned; whether a 32-bit hex literal
                // below 0x10000 is an Integer or a Long is not settled.
                // The 32-bit form is a `Long` literal: a hexadecimal literal
                // with up to four digits and no `&` suffix is an `Integer`
                // and has the 16-bit form, so values below 0x10000 here
                // were written with the suffix.
                let v = self.int_operand(ins)?;
                if !(0..=0xFFFF_FFFF).contains(&v) {
                    return Err(self.no(op, "hex literal out of range"));
                }
                self.stack
                    .push(Ex::Const(Variant::Long(v as u32 as i32), Type::Long));
            }
            Opcode::ConstInt16Hex => {
                // An `Integer` literal; the digits are the 16 bits.
                let v = self.int_operand(ins)?;
                if !(0..=0xFFFF).contains(&v) {
                    return Err(self.no(op, "hex literal out of range"));
                }
                self.stack
                    .push(Ex::Const(Variant::Integer(v as u16 as i16), Type::Integer));
            }
            Opcode::ConvertToType => {
                // The modifier is the target's `VarType`; only the two
                // targets that occur in the observed material are lowered.
                let name = match ins.modifier {
                    3 => "CLng",
                    8 => "CStr",
                    _ => return Err(self.no(op, "conversion target not observed")),
                };
                let a = self.pop(ins)?;
                let c = self.intrinsic_call(ins, name, false, vec![a])?;
                self.stack.push(c);
            }
            Opcode::ConstFloat64 => match ins.operands.first() {
                Some(Operand::Float64(v)) => {
                    self.stack
                        .push(Ex::Const(Variant::Double(*v), Type::Double));
                }
                _ => return Err(self.no(op, "bad literal")),
            },
            Opcode::ConstDate => match ins.operands.first() {
                Some(Operand::Date(v)) => {
                    let d = vba_value::Date::from_serial(*v)
                        .map_err(|_| self.no(op, "date literal out of range"))?;
                    self.stack.push(Ex::Const(Variant::Date(d), Type::Date));
                }
                _ => return Err(self.no(op, "bad literal")),
            },
            Opcode::ConstText => match ins.operands.first() {
                Some(Operand::Text(t)) if t.is_ascii() => {
                    let s = String::from_utf8_lossy(t).into_owned();
                    self.stack
                        .push(Ex::Const(Variant::String(VbaString::from(s)), Type::String));
                }
                _ => return Err(self.no(op, "string literal outside ASCII")),
            },
            Opcode::ConstKeywordValue => match ins.modifier {
                0 => self
                    .stack
                    .push(Ex::Const(Variant::Boolean(false), Type::Boolean)),
                1 => self
                    .stack
                    .push(Ex::Const(Variant::Boolean(true), Type::Boolean)),
                2 => self.stack.push(Ex::Const(Variant::Null, Type::Variant)),
                3 => self.stack.push(Ex::Const(Variant::Empty, Type::Variant)),
                _ => return Err(self.no(op, "unknown keyword value")),
            },
            // ---- operators ----
            Opcode::NumAdd => self.binary(ins, AstBin::Add)?,
            Opcode::NumSubtract => self.binary(ins, AstBin::Sub)?,
            Opcode::NumMultiply => self.binary(ins, AstBin::Mul)?,
            Opcode::NumDivide => self.binary(ins, AstBin::Div)?,
            Opcode::NumIntDivide => self.binary(ins, AstBin::IntDiv)?,
            Opcode::NumRemainder => self.binary(ins, AstBin::Mod)?,
            Opcode::TextJoin => self.binary(ins, AstBin::Concat)?,
            Opcode::CompareEqual => self.binary(ins, AstBin::Eq)?,
            Opcode::CompareNotEqual => self.binary(ins, AstBin::Ne)?,
            Opcode::CompareLess => self.binary(ins, AstBin::Lt)?,
            Opcode::CompareGreater => self.binary(ins, AstBin::Gt)?,
            Opcode::CompareLessEqual => self.binary(ins, AstBin::Le)?,
            Opcode::CompareGreaterEqual => self.binary(ins, AstBin::Ge)?,
            Opcode::ComparePattern => self.binary(ins, AstBin::Like)?,
            Opcode::BoolConjunction => self.binary(ins, AstBin::And)?,
            Opcode::BoolDisjunction => self.binary(ins, AstBin::Or)?,
            Opcode::BoolNegate => {
                let e = self.pop(ins)?;
                let ty = vba_sema::not_result(&e.ty());
                self.stack.push(Ex::Unary {
                    op: UnOp::Not,
                    ty,
                    e: Box::new(e),
                });
            }
            Opcode::NumNegate => {
                let e = self.pop(ins)?;
                let ty = vba_sema::negate_result(&e.ty());
                self.stack.push(Ex::Unary {
                    op: UnOp::Neg,
                    ty,
                    e: Box::new(e),
                });
            }
            Opcode::ExprParentheses => {
                let e = self.pop(ins)?;
                self.stack.push(Ex::Paren(Box::new(e)));
            }
            Opcode::IntrinsicAbsoluteValue => self.unary_intrinsic(ins, "Abs")?,
            Opcode::IntrinsicFloor => self.unary_intrinsic(ins, "Int")?,
            Opcode::IntrinsicSign => self.unary_intrinsic(ins, "Sgn")?,
            Opcode::IntrinsicLengthChars => self.unary_intrinsic(ins, "Len")?,
            Opcode::IntrinsicSearchA | Opcode::IntrinsicSearchB | Opcode::IntrinsicSearchC => {
                let n = match op {
                    Opcode::IntrinsicSearchA => 2,
                    Opcode::IntrinsicSearchB => 3,
                    _ => 4,
                };
                let args = self.pop_n(ins, n)?;
                let c = self.intrinsic_call(ins, "InStr", false, args)?;
                self.stack.push(c);
            }
            // ---- names ----
            Opcode::ReadSimple => {
                if matches!(
                    ins.operands.first(),
                    Some(Operand::Name {
                        name: vba_pcode::Name::Me,
                        ..
                    })
                ) {
                    if !self.cx.class_like[self.cx.module] {
                        return Err(self.no(op, "Me in a standard module"));
                    }
                    let me = self.me();
                    self.stack.push(me);
                    return Ok(1);
                }
                let (name, _) = self.name_of(ins)?;
                // A member access consumes the receiver right after it is
                // pushed.
                let receiver = body.get(i + 1).is_some_and(|(_, n)| {
                    matches!(
                        n.opcode,
                        Opcode::ReadDot
                            | Opcode::ReadDotParams
                            | Opcode::InvokeDot
                            | Opcode::WriteDot
                    )
                });
                let e = self.read_name(ins, &name, Vec::new(), receiver)?;
                self.stack.push(e);
            }
            Opcode::ReadSimpleParams => {
                let (name, suffix) = self.name_of(ins)?;
                let argc = match ins.operands.get(1) {
                    Some(Operand::Count(c)) => usize::from(*c),
                    _ => return Err(self.no(op, "missing argument count")),
                };
                let args = self.pop_n(ins, argc)?;
                let e = self.read_name_call(ins, &name, suffix, args)?;
                self.stack.push(e);
            }
            Opcode::WriteSimple => {
                self.assign(ins)?;
                self.close_stmt();
            }
            Opcode::ErrHandlerSet => {
                let mode = if ins.modifier == 1 {
                    ErrMode::ResumeNext
                } else {
                    let (name, _) = self.name_of(ins)?;
                    ErrMode::Handler(self.label_block(&name))
                };
                self.emit(InstKind::SetErrorMode { mode });
                self.close_stmt();
            }
            Opcode::ErrResume => {
                let (name, _) = self.name_of(ins)?;
                let b = self.label_block(&name);
                // No resume point: the code after a jump is unreachable, and
                // error 20 (`Resume` without an error) propagates.
                self.terminate_and_continue(Terminator::Resume(ResumeKind::Block(b)));
            }
            Opcode::LabelNamed => {
                let (name, _) = self.name_of(ins)?;
                let b = self.label_block(&name);
                let key = name.to_ascii_lowercase();
                if self.defined.contains(&key) {
                    return Err(self.no(op, "label defined twice"));
                }
                self.defined.push(key);
                self.jump_to(b);
                self.close_stmt();
            }
            Opcode::ReadDot => {
                let (name, _) = self.name_of(ins)?;
                let recv = self.pop(ins)?;
                let e = self.dot_member(ins, recv, &name, Vec::new())?;
                self.stack.push(e);
            }
            Opcode::ReadDotParams => {
                let (name, _) = self.name_of(ins)?;
                let argc = match ins.operands.get(1) {
                    Some(Operand::Count(c)) => usize::from(*c),
                    _ => return Err(self.no(op, "missing argument count")),
                };
                let recv = self.pop(ins)?;
                let args = self.pop_n(ins, argc)?;
                let e = self.dot_member(ins, recv, &name, args)?;
                self.stack.push(e);
            }
            Opcode::InvokeDot => {
                let (name, _) = self.name_of(ins)?;
                let argc = match ins.operands.get(1) {
                    Some(Operand::Count(c)) => usize::from(*c),
                    _ => return Err(self.no(op, "missing argument count")),
                };
                let recv = self.pop(ins)?;
                let args = self.pop_n(ins, argc)?;
                let call = self.dot_member(ins, recv, &name, args)?;
                self.expect_empty(ins)?;
                self.emit_call_stmt(&call);
                self.close_stmt();
            }
            Opcode::WriteDot => {
                self.dot_assign(ins)?;
                self.close_stmt();
            }

            // ---- statements ----
            Opcode::HaltEnd => {
                self.expect_empty(ins)?;
                self.terminate_and_continue(Terminator::End);
            }
            Opcode::ArgMissing => self.stack.push(Ex::Missing),
            Opcode::BranchGoto => {
                let (name, _) = self.name_of(ins)?;
                let b = self.label_block(&name);
                self.terminate_and_continue(Terminator::Jump(b));
            }
            Opcode::VarDecl if ins.modifier & 0x23 == 2 => self.local_const(ins)?,
            Opcode::DimMarker | Opcode::VarDecl | Opcode::OptionStmt => {}
            Opcode::InvokeSimple => {
                let (name, suffix) = self.name_of(ins)?;
                let argc = match ins.operands.get(1) {
                    Some(Operand::Count(c)) => usize::from(*c),
                    _ => return Err(self.no(op, "missing argument count")),
                };
                let args = self.pop_n(ins, argc)?;
                let call = self.statement_call(ins, &name, suffix, args)?;
                self.expect_empty(ins)?;
                self.emit_call_stmt(&call);
                self.close_stmt();
            }
            Opcode::IfBlockOpen => {
                let c = self.pop(ins)?;
                self.expect_empty(ins)?;
                let c = self.coerce_condition(c);
                let co = self.emit_ex(&c);
                let then_b = self.new_block();
                let else_b = self.new_block();
                let end = self.new_block();
                self.resume_at(then_b);
                self.terminate(Terminator::Branch {
                    cond: co,
                    then_block: then_b,
                    else_block: else_b,
                });
                self.switch_to(then_b);
                self.frames.push(Frame::If {
                    end,
                    else_b,
                    has_else: false,
                });
            }
            Opcode::IfSingleLine => {
                let c = self.pop(ins)?;
                self.expect_empty(ins)?;
                let c = self.coerce_condition(c);
                let co = self.emit_ex(&c);
                let then_b = self.new_block();
                let else_b = self.new_block();
                let end = self.new_block();
                self.resume_at(then_b);
                self.terminate(Terminator::Branch {
                    cond: co,
                    then_block: then_b,
                    else_block: else_b,
                });
                self.switch_to(then_b);
                self.frames.push(Frame::If {
                    end,
                    else_b,
                    has_else: false,
                });
            }
            Opcode::BlockElseifBranch => {
                let c = self.pop(ins)?;
                self.expect_empty(ins)?;
                let (end, prev_else) = match self.frames.last() {
                    Some(Frame::If {
                        end,
                        else_b,
                        has_else: false,
                    }) => (*end, *else_b),
                    _ => return Err(self.no(op, "ElseIf outside a block If")),
                };
                self.terminate(Terminator::Jump(end));
                self.switch_to(prev_else);
                self.begin_stmt();
                let c = self.coerce_condition(c);
                let co = self.emit_ex(&c);
                let then_b = self.new_block();
                let else_b = self.new_block();
                self.resume_at(then_b);
                self.terminate(Terminator::Branch {
                    cond: co,
                    then_block: then_b,
                    else_block: else_b,
                });
                self.switch_to(then_b);
                if let Some(Frame::If { else_b: e, .. }) = self.frames.last_mut() {
                    *e = else_b;
                }
            }
            Opcode::BlockElseBranch => {
                let (end, prev_else) = match self.frames.last() {
                    Some(Frame::If {
                        end,
                        else_b,
                        has_else: false,
                    }) => (*end, *else_b),
                    _ => return Err(self.no(op, "Else outside a block If")),
                };
                self.terminate(Terminator::Jump(end));
                self.switch_to(prev_else);
                if let Some(Frame::If { has_else, .. }) = self.frames.last_mut() {
                    *has_else = true;
                }
            }
            Opcode::BlockIfClose | Opcode::EndifSingleLine => match self.frames.pop() {
                Some(Frame::If {
                    end,
                    else_b,
                    has_else,
                }) => {
                    self.terminate(Terminator::Jump(end));
                    if !has_else {
                        self.switch_to(else_b);
                        self.terminate(Terminator::Jump(end));
                    }
                    self.switch_to(end);
                }
                _ => return Err(self.no(op, "End If without a block If")),
            },
            Opcode::LeaveFunction | Opcode::LeaveSub => {
                self.terminate_and_continue(Terminator::Return);
            }
            Opcode::LeaveFor => {
                let exit = self.frames.iter().rev().find_map(|f| match f {
                    Frame::For { exit, .. } => Some(*exit),
                    _ => None,
                });
                let Some(exit) = exit else {
                    return Err(self.no(op, "Exit For outside a For loop"));
                };
                self.terminate_and_continue(Terminator::Jump(exit));
            }
            Opcode::ForVarExprOpen => {
                self.for_depth = Some(self.stack.len());
                self.for_var = None;
            }
            Opcode::ForVarExprClose => {
                let Some(d) = self.for_depth.take() else {
                    return Err(self.no(op, "unbalanced For variable markers"));
                };
                if self.stack.len() != d + 1 {
                    return Err(self.no(op, "For variable expression is not one value"));
                }
                self.for_var = self.stack.pop();
            }
            Opcode::ForCounted | Opcode::ForCountedStep => self.for_open(ins)?,
            Opcode::ForAdvance | Opcode::ForAdvanceNamed => self.for_next(ins)?,
            Opcode::DoOpen => {
                let head = self.new_block();
                let body_b = self.new_block();
                let exit = self.new_block();
                self.resume_at(body_b);
                self.jump_to(head);
                self.terminate(Terminator::Jump(body_b));
                self.switch_to(body_b);
                self.frames.push(Frame::Loop {
                    head,
                    body: body_b,
                    exit,
                });
            }
            Opcode::DoOpenWhile | Opcode::DoOpenUntil | Opcode::WhileOpen => {
                let c = self.pop(ins)?;
                self.expect_empty(ins)?;
                let until = op == Opcode::DoOpenUntil;
                let head = self.new_block();
                let body_b = self.new_block();
                let exit = self.new_block();
                self.resume_at(body_b);
                self.jump_to(head);
                let c = self.coerce_condition(c);
                let co = self.emit_ex(&c);
                let (t, e) = if until {
                    (exit, body_b)
                } else {
                    (body_b, exit)
                };
                self.terminate(Terminator::Branch {
                    cond: co,
                    then_block: t,
                    else_block: e,
                });
                self.switch_to(body_b);
                self.frames.push(Frame::Loop {
                    head,
                    body: body_b,
                    exit,
                });
            }
            Opcode::DoClose | Opcode::WhileClose => match self.frames.pop() {
                Some(Frame::Loop { head, exit, .. }) => {
                    self.terminate(Terminator::Jump(head));
                    self.switch_to(exit);
                }
                _ => return Err(self.no(op, "Loop without Do")),
            },
            Opcode::DoCloseWhile | Opcode::DoCloseUntil => {
                let c = self.pop(ins)?;
                self.expect_empty(ins)?;
                match self.frames.pop() {
                    Some(Frame::Loop { head, body, exit }) => {
                        self.begin_stmt();
                        self.resume_at(body);
                        let c = self.coerce_condition(c);
                        let co = self.emit_ex(&c);
                        let (t, e) = if op == Opcode::DoCloseUntil {
                            (exit, head)
                        } else {
                            (head, exit)
                        };
                        self.terminate(Terminator::Branch {
                            cond: co,
                            then_block: t,
                            else_block: e,
                        });
                        self.switch_to(exit);
                    }
                    _ => return Err(self.no(op, "Loop without Do")),
                }
            }
            Opcode::SwitchBegin => {
                let s = self.pop(ins)?;
                self.expect_empty(ins)?;
                let ty = s.ty();
                let so = self.emit_ex(&s);
                let end = self.new_block();
                let chain = self.cur;
                let stmt = self.cur_stmt;
                self.frames.push(Frame::Select {
                    stmt,
                    first_body: false,
                    subject: so,
                    subject_ty: ty,
                    end,
                    chain,
                    body: None,
                    miss: None,
                    in_body: false,
                    in_else: false,
                });
            }
            Opcode::CaseValue
            | Opcode::CaseRange
            | Opcode::CaseIsEqual
            | Opcode::CaseIsNotEqual => {
                return self.case_item(body, i);
            }
            Opcode::CaseListEnd => {
                let (body_b, miss) = match self.frames.last_mut() {
                    Some(Frame::Select { body, miss, .. }) => (body.take(), miss.take()),
                    _ => return Err(self.no(op, "Case list outside Select Case")),
                };
                let (Some(body_b), Some(miss)) = (body_b, miss) else {
                    return Err(self.no(op, "empty Case list"));
                };
                self.switch_to(body_b);
                if let Some(Frame::Select { chain, in_body, .. }) = self.frames.last_mut() {
                    *chain = miss;
                    *in_body = true;
                }
            }
            Opcode::CaseDefault => {
                let (end, chain, in_body) = match self.frames.last() {
                    Some(Frame::Select {
                        end,
                        chain,
                        in_body,
                        in_else: false,
                        ..
                    }) => (*end, *chain, *in_body),
                    _ => return Err(self.no(op, "Case Else outside Select Case")),
                };
                if in_body {
                    self.terminate(Terminator::Jump(end));
                }
                self.switch_to(chain);
                if let Some(Frame::Select {
                    in_body, in_else, ..
                }) = self.frames.last_mut()
                {
                    *in_body = false;
                    *in_else = true;
                }
            }
            Opcode::SwitchEnd => match self.frames.pop() {
                Some(Frame::Select {
                    end,
                    chain,
                    in_body,
                    in_else,
                    ..
                }) => {
                    self.terminate(Terminator::Jump(end));
                    if in_body && !in_else {
                        // No `Case Else`: the unmatched chain falls out.
                        self.switch_to(chain);
                        self.terminate(Terminator::Jump(end));
                    }
                    self.switch_to(end);
                }
                _ => return Err(self.no(op, "End Select without Select Case")),
            },
            Opcode::DebugPrefix => {
                if self.print.is_some() {
                    return Err(self.no(op, "nested Debug.Print"));
                }
                self.print = Some((Vec::new(), Vec::new()));
            }
            Opcode::OutTargetObject => {
                if self.print.is_none() {
                    return Err(self.no(op, "output target outside Debug.Print"));
                }
            }
            Opcode::OutItemThenComma | Opcode::OutItemThenSemicolon | Opcode::OutItemLast => {
                let e = self.pop(ins)?;
                self.expect_empty(ins)?;
                let Some((vals, seps)) = self.print.as_mut() else {
                    return Err(self.no(op, "output item outside Debug.Print"));
                };
                vals.push(e);
                match op {
                    Opcode::OutItemThenComma => seps.push(OutItem::Comma),
                    Opcode::OutItemThenSemicolon => seps.push(OutItem::Semicolon),
                    _ => {
                        let (vals, seps) = self.print.take().unwrap_or_default();
                        let mut items = Vec::new();
                        let mut seps = seps.into_iter();
                        for v in &vals {
                            let o = self.emit_ex(v);
                            items.push(OutItem::Value(o));
                            if let Some(s) = seps.next() {
                                items.push(s);
                            }
                        }
                        self.emit(InstKind::DebugPrint { items });
                        self.close_stmt();
                    }
                }
            }
            _ => {
                return Err(self.no(op, "verified opcode outside the lowering subset"));
            }
        }
        Ok(1)
    }

    /// Opcodes that are lowered although the opcode table marks them
    /// inferred: the form is pinned by the differential run against the
    /// source-derived program (`On Error Resume Next`, modifier 1).
    fn accepted_without_verification(&self, ins: &Listed) -> bool {
        match ins.opcode {
            // `On Error Resume Next` (modifier 1) and `On Error GoTo label`
            // (modifier 0 with a name).
            Opcode::ErrHandlerSet => ins.modifier == 1 || ins.modifier == 0,
            // `Resume label`.
            Opcode::ErrResume => ins.modifier == 0,
            Opcode::LabelNamed => true,
            // An omitted positional argument.
            Opcode::ArgMissing => true,
            // Hexadecimal `Integer` literals and the `CLng`/`CStr` family.
            Opcode::ConstInt16Hex | Opcode::ConstDate => true,
            Opcode::ConvertToType => matches!(ins.modifier, 3 | 8),
            // Property reads on the `Err` object, and the member forms with
            // `Me` or a late-bound value as the receiver (on top of the
            // stack).
            Opcode::ReadDot => matches!(
                self.stack.last(),
                Some(Ex::LibObj { .. } | Ex::Me(_) | Ex::Late { .. })
            ),
            Opcode::ReadDotParams | Opcode::InvokeDot | Opcode::WriteDot => {
                matches!(self.stack.last(), Some(Ex::Me(_) | Ex::Late { .. }))
            }
            _ => false,
        }
    }

    fn expect_empty(&self, ins: &Listed) -> R<()> {
        if self.stack.is_empty() {
            Ok(())
        } else {
            Err(self.no(ins.opcode, "values left on the expression stack"))
        }
    }

    fn int_operand(&self, ins: &Listed) -> R<i64> {
        match ins.operands.first() {
            Some(Operand::Int { value, .. }) => Ok(*value),
            _ => Err(self.no(ins.opcode, "bad literal")),
        }
    }

    fn binary(&mut self, ins: &Listed, op: AstBin) -> R<()> {
        let r = self.pop(ins)?;
        let l = self.pop(ins)?;
        let (lt, rt) = (l.ty(), r.ty());
        if lt.is_object() || rt.is_object() {
            return Err(self.no(ins.opcode, "object operand"));
        }
        let ty = vba_sema::binary_result(op, &lt, &rt);
        if ty.is_error() {
            return Err(self.no(ins.opcode, "operand types do not combine"));
        }
        self.stack.push(Ex::Binary {
            op,
            ty,
            l: Box::new(l),
            r: Box::new(r),
        });
        Ok(())
    }

    fn unary_intrinsic(&mut self, ins: &Listed, name: &str) -> R<()> {
        let a = self.pop(ins)?;
        let c = self.intrinsic_call(ins, name, false, vec![a])?;
        self.stack.push(c);
        Ok(())
    }

    /// The arguments of a late-bound call: variables travel by reference.
    fn late_args(args: Vec<Ex>) -> Vec<LateEx> {
        args.into_iter()
            .map(|a| match &a {
                Ex::Local { id, bare: true, .. } => LateEx::Ref(RefTarget::Local(*id)),
                Ex::Global { id, bare: true, .. } => LateEx::Ref(RefTarget::Global(*id)),
                Ex::Missing => LateEx::Missing,
                _ => LateEx::Val(a),
            })
            .collect()
    }

    /// A member of the host base, called on `Me`.
    fn host_member(&self, name: &str, args: Vec<Ex>) -> Ex {
        Ex::Late {
            recv: Box::new(self.me()),
            name: name.to_owned(),
            args: Self::late_args(args),
        }
    }

    /// A module-level constant as a value.
    fn const_ex(&self, ins: &Listed, c: vba_sema::ConstId) -> R<Ex> {
        let info = &self.cx.program.consts[c.index()];
        match &info.value {
            Constant::Value(v) => Ok(Ex::Const(v.clone(), info.ty.clone())),
            _ => Err(self.no(ins.opcode, "constant that is not a plain value")),
        }
    }

    /// `READ_SIMPLE` / zero-argument names.
    fn read_name(&mut self, ins: &Listed, name: &str, args: Vec<Ex>, receiver: bool) -> R<Ex> {
        if self.is_result_name(name) {
            return Err(self.no(ins.opcode, "reads the function's own name"));
        }
        if let Some((id, ty)) = self.local_by_name(name) {
            return Ok(Ex::Local { id, ty, bare: true });
        }
        if let Some((v, ty)) = self.local_consts.get(&name.to_ascii_lowercase()) {
            return Ok(Ex::Const(v.clone(), ty.clone()));
        }
        match self.module_symbol(name) {
            Some(Symbol::Var(g)) if args.is_empty() => return Ok(self.global_ex(g)),
            Some(Symbol::Const(c)) if args.is_empty() => return self.const_ex(ins, c),
            Some(Symbol::Var(_) | Symbol::Const(_)) => {
                return Err(self.no(ins.opcode, "indexing of a variable"));
            }
            _ => {}
        }
        if let Some(pid) = self.proc_target(name, Role::Read) {
            let callee = self.cx.program.proc(pid);
            if callee.has_result && callee.params.is_empty() {
                return self.project_call(ins, pid, args);
            }
        }
        if args.is_empty() {
            match BuiltinLibrary.global(name) {
                Some(LibGlobal::Constant(v, ty)) => return Ok(Ex::Const(v, ty)),
                Some(LibGlobal::Object(Type::Library(lt))) if name.eq_ignore_ascii_case("err") => {
                    return Ok(Ex::LibObj {
                        name: "Err".to_owned(),
                        lt,
                    });
                }
                _ => {}
            }
        }
        if let Some(r) = vba_stdlib::lookup(name)
            && r.info.min_args() == 0
            && !r.info.params.iter().any(|p| p.param_array)
        {
            return self.intrinsic_call(ins, name, false, args);
        }
        if self.is_host_name(name, !receiver && args.is_empty()) {
            return Ok(self.host_member(name, args));
        }
        Err(self.no(
            ins.opcode,
            format!("name `{name}` is not a local, a procedure or a library function"),
        ))
    }

    fn read_name_call(
        &mut self,
        ins: &Listed,
        name: &str,
        suffix: Option<char>,
        args: Vec<Ex>,
    ) -> R<Ex> {
        if self.local_by_name(name).is_some()
            || self.local_consts.contains_key(&name.to_ascii_lowercase())
        {
            return Err(self.no(ins.opcode, "indexing of a variable"));
        }
        if self.is_result_name(name) && suffix.is_none() {
            // `Name(args)` inside the function `Name` is a recursive call.
            let pid = self.cx.proc_id;
            if !self.callees.contains(&pid) {
                self.callees.push(pid);
            }
            return self.project_call(ins, pid, args);
        }
        if matches!(
            self.module_symbol(name),
            Some(Symbol::Var(_) | Symbol::Const(_))
        ) {
            return Err(self.no(ins.opcode, "indexing of a variable"));
        }
        if suffix.is_none()
            && let Some(pid) = self.proc_target(name, Role::Read)
        {
            let callee = self.cx.program.proc(pid);
            if !callee.has_result {
                return Err(self.no(ins.opcode, "a Sub used as a value"));
            }
            return self.project_call(ins, pid, args);
        }
        let dollar = match suffix {
            None => false,
            Some('$') => true,
            Some(_) => return Err(self.no(ins.opcode, "type-suffixed name")),
        };
        if suffix.is_none() && self.is_host_name(name, false) {
            return Ok(self.host_member(name, args));
        }
        self.intrinsic_call(ins, name, dollar, args)
    }

    fn statement_call(
        &mut self,
        ins: &Listed,
        name: &str,
        suffix: Option<char>,
        args: Vec<Ex>,
    ) -> R<Ex> {
        if self.local_by_name(name).is_some()
            || self.is_result_name(name)
            || self.local_consts.contains_key(&name.to_ascii_lowercase())
        {
            return Err(self.no(ins.opcode, "call of a variable"));
        }
        if matches!(
            self.module_symbol(name),
            Some(Symbol::Var(_) | Symbol::Const(_))
        ) {
            return Err(self.no(ins.opcode, "call of a variable"));
        }
        if suffix.is_none()
            && let Some(pid) = self.proc_target(name, Role::Read)
        {
            return self.project_call(ins, pid, args);
        }
        if suffix.is_none() && self.is_host_name(name, false) {
            return Ok(self.host_member(name, args));
        }
        let dollar = suffix == Some('$');
        self.intrinsic_call(ins, name, dollar, args)
    }

    /// `receiver.name(args)` for the receivers the lowering knows: the
    /// `Err` object (properties only), `Me` (members of the module, then
    /// members of the host base) and late-bound values.
    fn dot_member(&mut self, ins: &Listed, recv: Ex, name: &str, args: Vec<Ex>) -> R<Ex> {
        match recv {
            Ex::Me(_) => self.me_member(ins, name, args),
            Ex::Late { .. } => Ok(Ex::Late {
                recv: Box::new(recv),
                name: name.to_owned(),
                args: Self::late_args(args),
            }),
            Ex::LibObj { ref lt, .. } if args.is_empty() => {
                let lt = lt.clone();
                let Some(m) = BuiltinLibrary.member(&lt, name) else {
                    return Err(self.no(ins.opcode, format!("`{name}` is not a known member")));
                };
                if !matches!(m.kind, LibMemberKind::Property | LibMemberKind::PropertyGet)
                    || !m.params.is_empty()
                    || m.ty.is_object()
                {
                    return Err(self.no(ins.opcode, "member is not a plain property"));
                }
                Ok(Ex::Member {
                    recv: Box::new(recv),
                    name: m.name.clone(),
                    ty: m.ty.clone(),
                    lt,
                    dispid: m.dispid,
                })
            }
            _ => Err(self.no(
                ins.opcode,
                "member access outside the Err object, Me and host members",
            )),
        }
    }

    /// A member of the current module reached through `Me`.
    fn me_member(&mut self, ins: &Listed, name: &str, args: Vec<Ex>) -> R<Ex> {
        let key = vba_ast::Name::new(name);
        let own = self.cx.program.module(self.sema.module);
        match own.symbols.get(&key).map(|(s, _)| *s) {
            Some(Symbol::Var(g)) if args.is_empty() => Ok(self.global_ex(g)),
            Some(Symbol::Proc(id)) => self.me_call(ins, id, args),
            Some(Symbol::Property(group)) => match group.get {
                Some(id) => self.me_call(ins, id, args),
                None => Err(self.no(ins.opcode, "property without a Get")),
            },
            Some(_) => Err(self.no(ins.opcode, "member of the module that is not callable")),
            None if self.cx.host_backed && !self.declared_in_module(name) => {
                Ok(self.host_member(name, args))
            }
            None => Err(self.no(ins.opcode, "member of the module that is not available")),
        }
    }

    /// Whether the cache declares `name` in the current module (a procedure
    /// or a variable, even one that is not part of the program).
    fn declared_in_module(&self, name: &str) -> bool {
        self.cx.index.members[self.cx.module].contains(&name.to_ascii_lowercase())
    }

    fn me_call(&mut self, ins: &Listed, id: ProcId, args: Vec<Ex>) -> R<Ex> {
        if !self.callees.contains(&id) {
            self.callees.push(id);
        }
        self.project_call(ins, id, args)
    }

    fn assign(&mut self, ins: &Listed) -> R<()> {
        let (name, _) = self.name_of(ins)?;
        let v = self.pop(ins)?;
        self.expect_empty(ins)?;
        self.store_name(ins, &name, v)
    }

    /// Stores `v` into the variable or `Property Let` called `name`.
    fn store_name(&mut self, ins: &Listed, name: &str, v: Ex) -> R<()> {
        enum Target {
            Local(LocalId, Type),
            Global(GlobalId, Type),
            Property(ProcId),
        }
        if self.local_consts.contains_key(&name.to_ascii_lowercase()) {
            return Err(self.no(ins.opcode, "assignment to a constant"));
        }
        let target = if self.is_result_name(name) {
            match self.sema.result_local {
                Some(l) => Target::Local(l, self.sema.ret.clone()),
                None => return Err(self.no(ins.opcode, "assignment to a Sub's name")),
            }
        } else if let Some((l, t)) = self.local_by_name(name) {
            Target::Local(l, t)
        } else {
            match self.module_symbol(name) {
                Some(Symbol::Var(g)) => Target::Global(g, self.cx.program.global(g).ty.clone()),
                Some(Symbol::Property(_)) => match self.proc_target(name, Role::Let) {
                    Some(p) => Target::Property(p),
                    None => return Err(self.no(ins.opcode, "property without a Let")),
                },
                _ => {
                    return Err(self.no(ins.opcode, "assignment to a name that is not a variable"));
                }
            }
        };
        let (ty, store) = match target {
            Target::Property(pid) => return self.let_property(ins, pid, v),
            Target::Local(l, t) => (t, Ok(l)),
            Target::Global(g, t) => (t, Err(g)),
        };
        if v.ty().is_object() || ty.is_object() {
            return Err(self.no(ins.opcode, "object assignment"));
        }
        let v = self.coerce(v, &ty);
        let o = self.emit_ex(&v);
        match store {
            Ok(slot) => self.emit(InstKind::StoreLocal {
                slot,
                src: o,
                mode: StoreMode::Let,
            }),
            Err(g) => self.emit_global_store(g, o),
        }
        Ok(())
    }

    /// Stores `v` into a module-level variable.
    fn store_global(&mut self, ins: &Listed, g: GlobalId, v: Ex) -> R<()> {
        let ty = self.cx.program.global(g).ty.clone();
        if v.ty().is_object() || ty.is_object() {
            return Err(self.no(ins.opcode, "object assignment"));
        }
        let v = self.coerce(v, &ty);
        let o = self.emit_ex(&v);
        self.emit_global_store(g, o);
        Ok(())
    }

    /// `Property Let` call with the value as its last argument (by value).
    fn let_property(&mut self, ins: &Listed, pid: ProcId, v: Ex) -> R<()> {
        if v.ty().is_object() {
            return Err(self.no(ins.opcode, "object assignment"));
        }
        let call = self.project_call(ins, pid, vec![Ex::Paren(Box::new(v))])?;
        self.emit_call_stmt(&call);
        Ok(())
    }

    /// `Const name [As T] = value` inside the procedure: the value must fold
    /// to a literal.
    fn local_const(&mut self, ins: &Listed) -> R<()> {
        let Some(Operand::Variable(v)) = ins.operands.first() else {
            return Err(self.no(ins.opcode, "unresolved constant"));
        };
        let Some(name) = crate::model::local_spelling(&v.name) else {
            return Err(self.no(ins.opcode, "name does not resolve"));
        };
        let value = self.pop(ins)?;
        self.expect_empty(ins)?;
        let value = self.fold(value);
        let value = match crate::model::scalar_type(&v.ty) {
            Some(t) if crate::model::declares_type(&v.ty) => self.coerce(value, &t),
            _ => value,
        };
        let Ex::Const(c, ty) = value else {
            return Err(self.no(ins.opcode, "constant value is not a literal"));
        };
        self.local_consts.insert(name.to_ascii_lowercase(), (c, ty));
        Ok(())
    }

    /// Folds a negated numeric literal.
    fn fold(&self, e: Ex) -> Ex {
        if let Ex::Unary {
            op: UnOp::Neg,
            e: inner,
            ..
        } = &e
            && let Ex::Const(v, t) = &**inner
            && let Ok(n) = vba_value::ops::negate(v, Overflow::Error)
        {
            return Ex::Const(n, t.clone());
        }
        e
    }

    /// `receiver.name = v`.
    fn dot_assign(&mut self, ins: &Listed) -> R<()> {
        let name = self.name_of(ins)?.0;
        let recv = self.pop(ins)?;
        let v = self.pop(ins)?;
        self.expect_empty(ins)?;
        match recv {
            Ex::Me(_) => {
                let key = vba_ast::Name::new(&name);
                let own = self.cx.program.module(self.sema.module);
                match own.symbols.get(&key).map(|(s, _)| *s) {
                    Some(Symbol::Var(g)) => self.store_global(ins, g, v),
                    Some(Symbol::Property(g)) => match g.let_ {
                        Some(p) => {
                            if !self.callees.contains(&p) {
                                self.callees.push(p);
                            }
                            self.let_property(ins, p, v)
                        }
                        None => Err(self.no(ins.opcode, "property without a Let")),
                    },
                    Some(_) => Err(self.no(ins.opcode, "assignment to a module member")),
                    None if self.cx.host_backed && !self.declared_in_module(&name) => {
                        self.late_put(ins, Ex::Me(self.sema.module), &name, v)
                    }
                    None => Err(self.no(ins.opcode, "member of the module that is not available")),
                }
            }
            Ex::Late { .. } => self.late_put(ins, recv, &name, v),
            _ => Err(self.no(ins.opcode, "assignment to a member of this receiver")),
        }
    }

    fn late_put(&mut self, ins: &Listed, recv: Ex, name: &str, v: Ex) -> R<()> {
        if v.ty().is_object() {
            return Err(self.no(ins.opcode, "object assignment"));
        }
        let v = self.coerce(v, &Type::Variant);
        // The value is evaluated before the receiver: the compiled code
        // pushes it first, and the source lowering evaluates the right-hand
        // side of an assignment before the target.
        let vo = self.emit_ex(&v);
        let receiver = self.emit_ex(&recv);
        self.emit(InstKind::Invoke {
            dst: None,
            receiver,
            member: Member::Named(name.to_owned()),
            kind: InvokeKind::Put,
            args: vec![LateArg {
                name: None,
                arg: CallArg::Val(vo),
            }],
            static_receiver: None,
            dispid: None,
        });
        Ok(())
    }

    // ---- For ---------------------------------------------------------------

    fn for_open(&mut self, ins: &Listed) -> R<()> {
        let op = ins.opcode;
        let n = if op == Opcode::ForCountedStep { 3 } else { 2 };
        let mut vals = self.pop_n(ins, n)?;
        self.expect_empty(ins)?;
        let Some(counter) = self.for_var.take() else {
            return Err(self.no(op, "For without a counter variable"));
        };
        let Ex::Local {
            id: counter_id,
            ty: counter_ty,
            ..
        } = counter
        else {
            return Err(self.no(op, "For counter is not a local variable"));
        };
        if counter_ty.is_object() {
            return Err(self.no(op, "object counter"));
        }
        let step_ex = if n == 3 { vals.pop() } else { None };
        let end_ex = vals.pop();
        let start_ex = vals.pop();
        let (Some(start_ex), Some(end_ex)) = (start_ex, end_ex) else {
            return Err(self.no(op, "malformed For header"));
        };
        let start = self.coerce(start_ex, &counter_ty);
        let end = self.coerce(end_ex, &counter_ty);
        let step = match step_ex {
            Some(s) => self.coerce(s, &counter_ty),
            None => self.default_step(&counter_ty),
        };
        let s = self.emit_ex(&start);
        let e = self.emit_ex(&end);
        let st = self.emit_ex(&step);
        self.emit(InstKind::StoreLocal {
            slot: counter_id,
            src: s,
            mode: StoreMode::Let,
        });
        let head = self.new_block();
        let body_b = self.new_block();
        let next_b = self.new_block();
        let exit = self.new_block();
        self.resume_at(body_b);
        self.jump_to(head);
        let cur = self.reg(counter_ty.clone());
        self.emit(InstKind::LoadLocal {
            dst: cur,
            slot: counter_id,
        });
        let c = self.reg(Type::Boolean);
        self.emit(InstKind::ForTest {
            dst: c,
            counter: IrOperand::Reg(cur),
            end: e,
            step: st.clone(),
        });
        self.terminate(Terminator::Branch {
            cond: IrOperand::Reg(c),
            then_block: body_b,
            else_block: exit,
        });
        self.switch_to(body_b);
        self.frames.push(Frame::For {
            head,
            body: body_b,
            next_b,
            exit,
            counter: counter_id,
            counter_ty,
            step: st,
        });
        Ok(())
    }

    fn default_step(&self, counter_ty: &Type) -> Ex {
        // The default step is the Integer constant 1, converted like the
        // other header values.
        self.coerce(Ex::Const(Variant::Integer(1), Type::Integer), counter_ty)
    }

    fn for_next(&mut self, ins: &Listed) -> R<()> {
        let op = ins.opcode;
        // `Next` carries FOR_VAR_EXPR markers (and the variable for the
        // named form); they were captured into `for_var`.
        self.for_var = None;
        self.for_depth = None;
        self.expect_empty(ins)?;
        match self.frames.pop() {
            Some(Frame::For {
                head,
                body,
                next_b,
                exit,
                counter,
                counter_ty,
                step,
            }) => {
                self.jump_to(next_b);
                self.begin_stmt();
                self.resume_at(body);
                let cur = self.reg(counter_ty.clone());
                self.emit(InstKind::LoadLocal {
                    dst: cur,
                    slot: counter,
                });
                let nv = self.reg(counter_ty.clone());
                self.emit(InstKind::Binary {
                    dst: nv,
                    op: BinOp::Arith(ArithOp::Add),
                    ty: counter_ty.clone(),
                    overflow: if counter_ty.is_variant() {
                        Overflow::Promote
                    } else {
                        Overflow::Error
                    },
                    lhs: IrOperand::Reg(cur),
                    rhs: step,
                    guard: None,
                });
                self.emit(InstKind::StoreLocal {
                    slot: counter,
                    src: IrOperand::Reg(nv),
                    mode: StoreMode::Let,
                });
                self.terminate(Terminator::Jump(head));
                self.switch_to(exit);
                Ok(())
            }
            _ => Err(self.no(op, "Next without For")),
        }
    }

    // ---- Select Case --------------------------------------------------------

    fn case_item(&mut self, body: &[(usize, Listed)], i: usize) -> R<usize> {
        // The comparisons belong to the statement of the `Select Case`, as in
        // the source lowering: the run time releases the objects held by
        // temporaries when the statement changes, and the subject may be one.
        let saved = self.cur_stmt;
        if let Some(Frame::Select { stmt, .. }) = self.frames.last() {
            self.cur_stmt = *stmt;
        }
        let r = self.case_item_in_stmt(body, i);
        self.cur_stmt = saved;
        r
    }

    fn case_item_in_stmt(&mut self, body: &[(usize, Listed)], i: usize) -> R<usize> {
        let ins = &body[i].1;
        let op = ins.opcode;
        let is_range = op == Opcode::CaseRange;
        let items = if is_range { 2 } else { 1 };
        let mut vals = self.pop_n(ins, items)?;
        // Open a new case when none is being built.
        let need_open = matches!(self.frames.last(), Some(Frame::Select { body: None, .. }));
        if !matches!(self.frames.last(), Some(Frame::Select { .. })) {
            return Err(self.no(op, "Case outside Select Case"));
        }
        if need_open {
            let (end, chain, in_body, in_else) = match self.frames.last() {
                Some(Frame::Select {
                    end,
                    chain,
                    in_body,
                    in_else,
                    ..
                }) => (*end, *chain, *in_body, *in_else),
                _ => return Err(self.no(op, "Case outside Select Case")),
            };
            if in_else {
                return Err(self.no(op, "Case after Case Else"));
            }
            if in_body {
                self.terminate(Terminator::Jump(end));
            }
            self.switch_to(chain);
            let body_b = self.new_block();
            let miss = self.new_block();
            let mut resume = None;
            if let Some(Frame::Select {
                stmt,
                first_body,
                body,
                miss: m,
                in_body,
                ..
            }) = self.frames.last_mut()
            {
                *body = Some(body_b);
                *m = Some(miss);
                *in_body = false;
                if !*first_body {
                    *first_body = true;
                    resume = Some(*stmt);
                }
            }
            if let Some(st) = resume {
                self.stmts[st.0 as usize].resume_next = Some(Point {
                    block: body_b,
                    index: 0,
                });
            }
        }
        let (subject, subject_ty, body_b, miss) = match self.frames.last() {
            Some(Frame::Select {
                subject,
                subject_ty,
                body: Some(b),
                miss: Some(m),
                ..
            }) => (subject.clone(), subject_ty.clone(), *b, *m),
            _ => return Err(self.no(op, "Case outside Select Case")),
        };
        // The item is the last of its list when CASE_LIST_END follows.
        let last = body
            .get(i + 1)
            .is_some_and(|(_, n)| n.opcode == Opcode::CaseListEnd);
        let next = if last { miss } else { self.new_block() };
        if is_range {
            let hi = vals.pop();
            let lo = vals.pop();
            let (Some(lo), Some(hi)) = (lo, hi) else {
                return Err(self.no(op, "malformed Case range"));
            };
            let (lot, hit) = (lo.ty(), hi.ty());
            let lo_o = self.emit_ex(&lo);
            let c1 = self.cmp(CompareOp::Ge, &subject, &subject_ty, lo_o, &lot);
            let second = self.new_block();
            self.terminate(Terminator::Branch {
                cond: c1,
                then_block: second,
                else_block: next,
            });
            self.switch_to(second);
            let hi_o = self.emit_ex(&hi);
            let c2 = self.cmp(CompareOp::Le, &subject, &subject_ty, hi_o, &hit);
            self.terminate(Terminator::Branch {
                cond: c2,
                then_block: body_b,
                else_block: next,
            });
        } else {
            let Some(v) = vals.pop() else {
                return Err(self.no(op, "malformed Case value"));
            };
            let vt = v.ty();
            let vo = self.emit_ex(&v);
            let cop = if op == Opcode::CaseIsNotEqual {
                CompareOp::Ne
            } else {
                CompareOp::Eq
            };
            let c = self.cmp(cop, &subject, &subject_ty, vo, &vt);
            self.terminate(Terminator::Branch {
                cond: c,
                then_block: body_b,
                else_block: next,
            });
        }
        self.switch_to(next);
        Ok(1)
    }
}
