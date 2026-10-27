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
use vba_sema::bound::NewTarget;
use vba_sema::builtins::BuiltinLibrary;
use vba_sema::builtins::collection_type;
use vba_sema::{
    EnumId, GlobalId, LocalId, LocalKind, ModuleId, ProcId, ProcInfo, Program, Symbol, Type, UdtId,
};
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
    /// Lower-case names of the modules of the project: a name that is the
    /// name of a module is that module (its default instance), never a
    /// member of the host base.
    pub module_names: std::collections::HashSet<String>,
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
        /// The object a method is invoked on (`Me`, or a value).
        recv: Option<Box<Ex>>,
    },
    /// A field of a class instance reached through a value.
    Field {
        recv: Box<Ex>,
        global: GlobalId,
        ty: Type,
    },
    /// A field of a user-defined type value.
    UdtField {
        base: Box<Ex>,
        udt: UdtId,
        index: u32,
        ty: Type,
    },
    /// `New T`.
    New {
        target: NewTarget,
        ty: Type,
    },
    /// A value that was already emitted (an assigned value that is
    /// evaluated before the target).
    Emitted(IrOperand, Type),
    /// A named argument (`name:=value`); only valid inside an argument list.
    Named(String, Box<Ex>),
    /// The predeclared default instance of a class module.
    DefaultInstance(ModuleId),
    /// A name that qualifies another (a module, an enum, the project); only
    /// valid as the receiver of a member form.
    Ns(Ns),
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
    /// A member of a built-in library object, early bound (`Err.Number`,
    /// `Collection.Add`).
    LibCall {
        recv: Box<Ex>,
        name: String,
        args: Vec<ArgEx>,
        ty: Type,
        lt: LibraryType,
        dispid: Option<i32>,
    },
    /// `TypeOf e Is T`.
    TypeOfIs {
        e: Box<Ex>,
        ty: Type,
    },
}

/// The sub-libraries of the `VBA` library: `Strings.Left` names a function
/// of the library like `Left` does.
fn is_sublibrary(name: &str) -> bool {
    [
        "Strings",
        "Conversion",
        "Information",
        "Interaction",
        "DateTime",
        "FileSystem",
        "Math",
        "Constants",
        "Financial",
    ]
    .iter()
    .any(|s| s.eq_ignore_ascii_case(name))
}

/// The class an object-table operand names.
enum ObjClass {
    Module(ModuleId),
    Builtin(LibraryType),
}

/// What a qualifying name stands for.
#[derive(Debug, Clone, Copy)]
enum Ns {
    Project,
    Module(ModuleId),
    Enum(EnumId),
    /// The `VBA` library and its sub-libraries (`VBA.Strings`).
    Vba,
}

/// A location that can be passed by reference or assigned to.
#[derive(Debug, Clone)]
enum Place {
    Local(LocalId),
    Global(GlobalId),
    Field {
        recv: Box<Ex>,
        global: GlobalId,
    },
    Udt {
        base: Box<Place>,
        udt: UdtId,
        index: u32,
        ty: Type,
    },
}

#[derive(Debug, Clone)]
enum ArgEx {
    Val(Ex),
    Ref(Place),
    /// A variable of another type passed to a `ByRef Variant` parameter: the
    /// callee works on a `Variant` copy in a temporary that is converted
    /// back into the variable when the call returns.
    RefCopy {
        place: Place,
        temp: LocalId,
        value: Ex,
        back: Ex,
    },
    Missing,
}

/// An argument of a late-bound call: variables travel by reference,
/// everything else by value, nothing is converted.
#[derive(Debug, Clone)]
enum LateEx {
    /// A named argument.
    Named(String, Box<LateEx>),
    Val(Ex),
    Ref(Place),
    Missing,
}

/// What a name is looked up for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Read, or call as a statement: `Sub`, `Function`, `Property Get`.
    Read,
    /// Assignment: `Property Let`.
    Let,
    /// Reference assignment: `Property Set`.
    Set,
}

impl Ex {
    fn ty(&self) -> Type {
        match self {
            Ex::Const(_, t) | Ex::Local { ty: t, .. } => t.clone(),
            Ex::Unary { ty, .. }
            | Ex::Binary { ty, .. }
            | Ex::Call { ty, .. }
            | Ex::Field { ty, .. }
            | Ex::UdtField { ty, .. }
            | Ex::New { ty, .. }
            | Ex::Emitted(_, ty)
            | Ex::Global { ty, .. } => ty.clone(),
            Ex::Me(m) | Ex::DefaultInstance(m) => Type::Class(*m),
            Ex::Named(_, e) => e.ty(),
            Ex::Ns(_) => Type::Error,
            Ex::Late { .. } | Ex::Missing => Type::Variant,
            Ex::Convert { to, .. } => to.clone(),
            Ex::Paren(e) => e.ty(),
            Ex::LibObj { lt, .. } => Type::Library(lt.clone()),
            Ex::LibCall { ty, .. } => ty.clone(),
            Ex::TypeOfIs { .. } => Type::Boolean,
        }
    }
}

#[derive(Debug)]
enum Frame {
    /// A `With` block: the temporary that holds the object.
    With { temp: LocalId },
    /// A `For Each` loop.
    ForEach { head: BlockId, exit: BlockId },
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
    /// The name of the project, for types qualified with it.
    pub project: Option<&'a str>,
}

/// The result of lowering one procedure.
pub(crate) struct Lowered {
    pub function: Function,
    pub callees: Vec<ProcId>,
    /// Types of the temporaries added after the procedure's own locals.
    pub temps: Vec<Type>,
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
    /// A `Set` keyword marker was seen and not yet consumed by the
    /// reference assignment that follows it.
    set_marker: bool,
    /// Temporaries added to the function (the sema program has none for
    /// the lowered body): slots after the template's.
    extra_slots: Vec<Slot>,
    n_base: usize,
    /// The objects of the open `With` blocks.
    with_stack: Vec<Ex>,
}

type R<T> = Result<T, Unsupported>;

/// The location behind a value that denotes a variable, a field or a
/// user-type field.
fn place_of(e: &Ex) -> Option<Place> {
    match e {
        Ex::Local { id, .. } => Some(Place::Local(*id)),
        Ex::Global { id, .. } => Some(Place::Global(*id)),
        Ex::Field { recv, global, .. } => Some(Place::Field {
            recv: recv.clone(),
            global: *global,
        }),
        Ex::UdtField {
            base,
            udt,
            index,
            ty,
        } => Some(Place::Udt {
            base: Box::new(place_of(base)?),
            udt: *udt,
            index: *index,
            ty: ty.clone(),
        }),
        _ => None,
    }
}

const SPAN: Span = Span { start: 0, end: 0 };

fn is_layout_only(op: Opcode) -> bool {
    matches!(
        op,
        Opcode::CommentQuote
            | Opcode::CommentRemKeyword
            | Opcode::LineContinuation
            | Opcode::StmtBegin
            | Opcode::StmtBeginImplicit
            // The lines of the branch that conditional compilation did not
            // take are kept as text.
            | Opcode::TextUnparsed
    )
}

/// The ordinary member form that a `With` member form stands for.
fn with_dot_equivalent(op: Opcode) -> Option<Opcode> {
    Some(match op {
        Opcode::ReadWithDot => Opcode::ReadDot,
        Opcode::ReadWithDotParams => Opcode::ReadDotParams,
        Opcode::WriteWithDot => Opcode::WriteDot,
        Opcode::WriteWithDotParams => Opcode::WriteDotParams,
        Opcode::WriteRefWithDot => Opcode::WriteRefDot,
        Opcode::WriteRefWithDotParams => Opcode::WriteRefDotParams,
        Opcode::InvokeWithDot => Opcode::InvokeDot,
        _ => return None,
    })
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
        set_marker: false,
        extra_slots: Vec::new(),
        n_base: kind_template.slots.len(),
        with_stack: Vec::new(),
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
        self.set_marker = false;
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
            .chain(self.extra_slots.iter().map(|s| Slot {
                name: s.name.clone(),
                ty: s.ty.clone(),
                role: SlotRole::Temp,
            }))
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
            temps: self.extra_slots.iter().map(|s| s.ty.clone()).collect(),
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

    /// Converts an argument to the parameter type: scalars as an
    /// assignment would, objects as `vba-sema` does for an argument.
    fn coerce_arg(&self, ins: &Listed, e: Ex, to: &Type) -> R<Ex> {
        if to.is_object() || e.ty().is_object() {
            return self.coerce_object(ins, e, to, true);
        }
        Ok(self.coerce(e, to))
    }

    /// Converts an object value to the type of the location or parameter
    /// that receives it (`Set` or an argument). A `Variant` source is only
    /// accepted for an argument (the `Set` statement checks it at run time
    /// through a call that is not lowered).
    fn coerce_object(&self, ins: &Listed, e: Ex, to: &Type, arg: bool) -> R<Ex> {
        let from = e.ty();
        if from == *to {
            return Ok(e);
        }
        if to.is_variant() {
            // An object into a `Variant` stays an object reference.
            return if from.is_object() {
                Ok(e)
            } else {
                Err(self.no(ins.opcode, "non-object value into an object assignment"))
            };
        }
        if !to.is_object() {
            return Err(self.no(ins.opcode, "object value where a scalar is expected"));
        }
        if from.is_variant() {
            return if arg {
                Ok(Ex::Convert {
                    e: Box::new(e),
                    to: to.clone(),
                })
            } else {
                Err(self.no(ins.opcode, "Variant source of a Set statement"))
            };
        }
        if !from.is_object() {
            return Err(self.no(ins.opcode, "non-object value into an object assignment"));
        }
        let program = self.cx.program;
        match (&from, to) {
            (Type::Nothing, _) | (_, Type::Object) => Ok(e),
            (Type::Class(a), Type::Class(_) | Type::Library(_)) => {
                if program
                    .module(*a)
                    .implements
                    .iter()
                    .any(|i| i.interface == *to)
                {
                    Ok(e)
                } else if let Type::Class(b) = to
                    && program
                        .module(*b)
                        .implements
                        .iter()
                        .any(|i| i.interface == Type::Class(*a))
                {
                    Ok(Ex::Convert {
                        e: Box::new(e),
                        to: to.clone(),
                    })
                } else {
                    Err(self.no(ins.opcode, "object types that do not convert"))
                }
            }
            (Type::Object, Type::Class(_) | Type::Library(_)) => Ok(Ex::Convert {
                e: Box::new(e),
                to: to.clone(),
            }),
            _ => Err(self.no(ins.opcode, "object types that do not convert")),
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
                Role::Set => k == K::PropertySet,
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

    /// An enum type by name: the module's own, then a public one of any
    /// module (enums are project-wide names).
    fn enum_by_name(&self, name: &str) -> Option<EnumId> {
        let key = vba_ast::Name::new(name);
        let program = self.cx.program;
        if let Some((Symbol::Enum(e), _)) = program.module(self.sema.module).symbols.get(&key) {
            return Some(*e);
        }
        let mut found = None;
        for m in &program.modules {
            if let Some((Symbol::Enum(e), _)) = m.symbols.get(&key)
                && program.enums[e.index()].visibility != vba_ast::Visibility::Private
            {
                if found.is_some() {
                    return None;
                }
                found = Some(*e);
            }
        }
        found
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
            if m.id == own.id {
                continue;
            }
            if m.is_class_like() {
                // Members of the public enums of a class module are global.
                if let Some((sym @ Symbol::EnumMember(e, _), _)) = m.symbols.get(&key)
                    && self.cx.program.enums[e.index()].visibility != vba_ast::Visibility::Private
                {
                    if found.is_some() {
                        return None;
                    }
                    found = Some(*sym);
                }
                continue;
            }
            if let Some((sym, _)) = m.symbols.get(&key)
                && matches!(
                    sym,
                    Symbol::Var(_) | Symbol::Const(_) | Symbol::EnumMember(..)
                )
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
            && !self
                .cx
                .index
                .module_names
                .contains(&name.to_ascii_lowercase())
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
            Some(Ex::Ns(_)) => Err(self.no(ins.opcode, "a qualifying name used as a value")),
            Some(Ex::Named(..)) => Err(self.no(ins.opcode, "named argument outside a call")),
            Some(e) => Ok(e),
            None => Err(self.no(ins.opcode, "expression stack underflow")),
        }
    }

    /// Pops the receiver of a member form: a value, or a qualifying name.
    fn pop_receiver(&mut self, ins: &Listed) -> R<Ex> {
        match self.stack.pop() {
            Some(e @ Ex::Ns(_)) => Ok(e),
            Some(e) => {
                self.stack.push(e);
                self.pop(ins)
            }
            None => Err(self.no(ins.opcode, "expression stack underflow")),
        }
    }

    fn pop_n(&mut self, ins: &Listed, n: usize) -> R<Vec<Ex>> {
        if self.stack.len() < n {
            return Err(self.no(ins.opcode, "expression stack underflow"));
        }
        let at = self.stack.len() - n;
        let v = self.stack.split_off(at);
        if v.iter().any(|e| matches!(e, Ex::Ns(_))) {
            return Err(self.no(ins.opcode, "a qualifying name used as a value"));
        }
        Ok(v)
    }

    /// Puts the arguments of a call in parameter order when some are named:
    /// positional ones first, then each named one at the place of the
    /// parameter it names; the others are omitted. Without named arguments
    /// the list is returned unchanged.
    fn arrange_args(&self, ins: &Listed, params: &[&str], args: Vec<Ex>) -> R<Vec<Ex>> {
        if !args.iter().any(|a| matches!(a, Ex::Named(..))) {
            return Ok(args);
        }
        let mut slots: Vec<Option<Ex>> = (0..params.len()).map(|_| None).collect();
        let mut pos = 0usize;
        let mut named_seen = false;
        for a in args {
            match a {
                Ex::Named(n, v) => {
                    named_seen = true;
                    let Some(i) = params.iter().position(|p| p.eq_ignore_ascii_case(&n)) else {
                        return Err(self.no(ins.opcode, "named argument that names no parameter"));
                    };
                    if slots[i].is_some() {
                        return Err(self.no(ins.opcode, "argument given twice"));
                    }
                    slots[i] = Some(*v);
                }
                other => {
                    if named_seen || pos >= slots.len() {
                        return Err(self.no(ins.opcode, "positional argument after a named one"));
                    }
                    slots[pos] = Some(other);
                    pos += 1;
                }
            }
        }
        // Trailing omitted parameters are dropped; the others are `Missing`.
        while matches!(slots.last(), Some(None)) {
            slots.pop();
        }
        Ok(slots
            .into_iter()
            .map(|s| s.unwrap_or(Ex::Missing))
            .collect())
    }

    // ---- calls ------------------------------------------------------------

    fn project_call(&mut self, ins: &Listed, id: ProcId, args: Vec<Ex>) -> R<Ex> {
        self.project_call_on(ins, None, id, args)
    }

    /// A call of a project procedure; `recv` is the object it is invoked on
    /// (a member of a class), otherwise `Me` for a member of the current
    /// class.
    fn project_call_on(
        &mut self,
        ins: &Listed,
        recv: Option<Ex>,
        id: ProcId,
        args: Vec<Ex>,
    ) -> R<Ex> {
        let callee = self.cx.program.proc(id);
        let names: Vec<String> = callee.params.iter().map(|p| p.name.to_string()).collect();
        let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let args = self.arrange_args(ins, &name_refs, args)?;
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
                out.push(ArgEx::Val(self.coerce_arg(ins, a, &p.ty)?));
                continue;
            }
            let target = match &a {
                Ex::Local {
                    id: lid,
                    ty,
                    bare: true,
                } => Some((Place::Local(*lid), ty.clone())),
                Ex::Global {
                    id: gid,
                    ty,
                    bare: true,
                } => Some((Place::Global(*gid), ty.clone())),
                Ex::Field { .. } | Ex::UdtField { .. } => place_of(&a).map(|pl| (pl, a.ty())),
                _ => None,
            };
            if let Some((t, ty)) = target {
                let same = ty == p.ty
                    || matches!(
                        (&ty, &p.ty),
                        (Type::Enum(_), Type::Long) | (Type::Long, Type::Enum(_))
                    );
                if same {
                    out.push(ArgEx::Ref(t));
                    continue;
                }
                if p.ty != Type::Variant
                    || matches!(ty, Type::Udt(_) | Type::Array { .. } | Type::FixedString(_))
                {
                    return Err(self.no(ins.opcode, "ByRef argument of a different type"));
                }
                // MS-VBAL 5.3.1.11: the callee works on a `Variant` copy.
                let value = self.coerce_arg(ins, a.clone(), &Type::Variant)?;
                let temp = self.new_temp(Type::Variant);
                let read = Ex::Local {
                    id: temp,
                    ty: Type::Variant,
                    bare: false,
                };
                let back = if ty.is_object() {
                    self.coerce_object(ins, read, &ty, true)?
                } else {
                    self.coerce(read, &ty)
                };
                out.push(ArgEx::RefCopy {
                    place: t,
                    temp,
                    value,
                    back,
                });
                continue;
            }
            out.push(ArgEx::Val(self.coerce_arg(ins, a, &p.ty)?));
        }
        out.resize(callee.params.len(), ArgEx::Missing);
        let recv = match recv {
            Some(r) => Some(Box::new(r)),
            None => self
                .cx
                .program
                .module(callee.module)
                .is_class_like()
                .then(|| Box::new(self.me())),
        };
        Ok(Ex::Call {
            callee: Callee::Proc(id),
            ty: if callee.has_result {
                callee.ret.clone()
            } else {
                Type::Variant
            },
            args: out,
            recv,
        })
    }

    fn intrinsic_call(&mut self, ins: &Listed, name: &str, dollar: bool, args: Vec<Ex>) -> R<Ex> {
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
        if args.iter().any(|a| matches!(a, Ex::Named(..)))
            && matches!(info.name, "InStr" | "InStrB")
        {
            return Err(self.no(ins.opcode, "named argument of InStr"));
        }
        let names: Vec<&str> = info.params.iter().map(|p| p.name).collect();
        let mut args = self.arrange_args(ins, &names, args)?;
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
            recv: None,
        })
    }

    /// A call of a function that the run time provides (`MsgBox`, `Beep`,
    /// ...): arguments are converted to the parameter types and omitted
    /// optional ones are passed as missing, as the source binding does.
    fn runtime_call(&mut self, ins: &Listed, m: &LibMember, args: Vec<Ex>) -> R<Ex> {
        if m.params.iter().any(|p| p.param_array || p.ty.is_object()) || m.ty.is_object() {
            return Err(self.no(ins.opcode, "object or ParamArray run-time function"));
        }
        let names: Vec<&str> = m.params.iter().map(|p| p.name.as_str()).collect();
        let args = self.arrange_args(ins, &names, args)?;
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
            out.push(ArgEx::Val(self.coerce_arg(ins, a, &p.ty)?));
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
            recv: None,
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
            Ex::LibCall { .. } => {
                let dst = self.emit_lib_call(e, true);
                IrOperand::Reg(dst.unwrap_or(Reg(0)))
            }
            Ex::TypeOfIs { e: inner, ty } => {
                let o = self.emit_ex(inner);
                let dst = self.reg(Type::Boolean);
                self.emit(InstKind::TypeOfIs {
                    dst,
                    src: o,
                    ty: ty.clone(),
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
                recv,
            } => {
                let dst = self.emit_call(callee, args, recv.as_deref(), Some(ty));
                IrOperand::Reg(dst.unwrap_or(Reg(0)))
            }
            Ex::Field { recv, global, ty } => {
                let obj = self.emit_ex(recv);
                let dst = self.reg(ty.clone());
                self.emit(InstKind::LoadField {
                    dst,
                    obj,
                    global: *global,
                });
                IrOperand::Reg(dst)
            }
            Ex::UdtField {
                base,
                udt,
                index,
                ty,
            } => {
                let b = self.emit_ex(base);
                let dst = self.reg(ty.clone());
                self.emit(InstKind::UdtField {
                    dst,
                    base: b,
                    udt: *udt,
                    index: *index,
                });
                IrOperand::Reg(dst)
            }
            Ex::Emitted(o, _) => o.clone(),
            Ex::Named(_, e) => self.emit_ex(e),
            Ex::DefaultInstance(m) => {
                let dst = self.reg(Type::Class(*m));
                self.emit(InstKind::LoadDefaultInstance { dst, module: *m });
                IrOperand::Reg(dst)
            }
            Ex::Ns(_) => IrOperand::Const(Constant::Value(Variant::Empty)),
            Ex::New { target, ty } => {
                let dst = self.reg(ty.clone());
                self.emit(InstKind::New {
                    dst,
                    target: target.clone(),
                });
                IrOperand::Reg(dst)
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
        match self.sema.locals.get(l.index()) {
            Some(li) => li.ty.clone(),
            None => self
                .extra_slots
                .get(l.index() - self.n_base)
                .map_or(Type::Variant, |s| s.ty.clone()),
        }
    }

    /// A new temporary slot.
    fn new_temp(&mut self, ty: Type) -> LocalId {
        let id = LocalId((self.n_base + self.extra_slots.len()) as u32);
        self.extra_slots.push(Slot {
            name: vba_ast::Name::new(""),
            ty,
            role: SlotRole::Temp,
        });
        id
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

    fn emit_ref(&mut self, t: &Place) -> Reg {
        match t {
            Place::Local(l) => {
                let r = self.reg(self.local_ty(*l));
                self.emit(InstKind::MakeRef {
                    dst: r,
                    place: RefPlace::Local(*l),
                });
                r
            }
            Place::Global(g) => {
                let g = *g;
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
            Place::Field { recv, global } => {
                let obj = self.emit_ex(recv);
                let ty = self.cx.program.global(*global).ty.clone();
                let r = self.reg(ty);
                self.emit(InstKind::MakeRef {
                    dst: r,
                    place: RefPlace::Field {
                        obj,
                        global: *global,
                    },
                });
                r
            }
            Place::Udt {
                base,
                udt,
                index,
                ty,
            } => {
                let b = self.emit_ref(base);
                let r = self.reg(ty.clone());
                self.emit(InstKind::MakeRef {
                    dst: r,
                    place: RefPlace::UdtField {
                        base: b,
                        udt: *udt,
                        index: *index,
                    },
                });
                r
            }
        }
    }

    /// Stores `v` into a location.
    fn store_place(&mut self, p: &Place, v: IrOperand, mode: StoreMode) {
        match p {
            Place::Local(l) => self.emit(InstKind::StoreLocal {
                slot: *l,
                src: v,
                mode,
            }),
            Place::Global(g) => {
                if self.global_in_instance(*g) {
                    let obj = self.emit_me();
                    self.emit(InstKind::StoreField {
                        obj,
                        global: *g,
                        src: v,
                        mode,
                    });
                } else {
                    self.emit(InstKind::StoreGlobal {
                        global: *g,
                        src: v,
                        mode,
                    });
                }
            }
            Place::Field { recv, global } => {
                let obj = self.emit_ex(recv);
                self.emit(InstKind::StoreField {
                    obj,
                    global: *global,
                    src: v,
                    mode,
                });
            }
            Place::Udt { .. } => {
                let r = self.emit_ref(p);
                self.emit(InstKind::StoreRef { r, src: v, mode });
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
        recv: Option<&Ex>,
        result: Option<&Type>,
    ) -> Option<Reg> {
        let receiver = recv.map(|r| self.emit_ex(r));
        let mut out = Vec::new();
        let mut writebacks: Vec<(Place, LocalId, Ex)> = Vec::new();
        for a in args {
            out.push(match a {
                ArgEx::Val(v) => CallArg::Val(self.emit_ex(v)),
                ArgEx::Missing => CallArg::Missing,
                ArgEx::Ref(t) => CallArg::Ref(self.emit_ref(t)),
                ArgEx::RefCopy {
                    place,
                    temp,
                    value,
                    back,
                } => {
                    let v = self.emit_ex(value);
                    self.emit(InstKind::StoreLocal {
                        slot: *temp,
                        src: v,
                        mode: StoreMode::Let,
                    });
                    let r = self.reg(Type::Variant);
                    self.emit(InstKind::MakeRef {
                        dst: r,
                        place: RefPlace::Local(*temp),
                    });
                    writebacks.push((place.clone(), *temp, back.clone()));
                    CallArg::Ref(r)
                }
            });
        }
        let dst = result.map(|t| self.reg(t.clone()));
        self.emit(InstKind::Call {
            dst,
            callee: callee.clone(),
            receiver,
            args: out,
        });
        // After a normal return: assign the variables from their copies and
        // release what the copies hold.
        for (place, temp, back) in writebacks {
            let v = self.emit_ex(&back);
            self.store_place(&place, v, StoreMode::Let);
            self.emit(InstKind::StoreLocal {
                slot: temp,
                src: IrOperand::Const(Constant::Value(Variant::Empty)),
                mode: StoreMode::Let,
            });
        }
        dst
    }

    /// One argument of a late-bound call.
    fn late_arg(&mut self, a: &LateEx) -> LateArg {
        match a {
            LateEx::Named(n, inner) => {
                let mut l = self.late_arg(inner);
                l.name = Some(n.clone());
                l
            }
            LateEx::Val(v) => LateArg {
                name: None,
                arg: CallArg::Val(self.emit_ex(v)),
            },
            LateEx::Ref(t) => LateArg {
                name: None,
                arg: CallArg::Ref(self.emit_ref(t)),
            },
            LateEx::Missing => LateArg {
                name: None,
                arg: CallArg::Missing,
            },
        }
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
            out.push(self.late_arg(a));
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

    /// Emits a member of a built-in library object; the result register,
    /// when wanted, is allocated after the receiver and the arguments.
    fn emit_lib_call(&mut self, e: &Ex, want: bool) -> Option<Reg> {
        let Ex::LibCall {
            recv,
            name,
            args,
            ty,
            lt,
            dispid,
            ..
        } = e
        else {
            return None;
        };
        let receiver = self.emit_ex(recv);
        let mut out = Vec::new();
        for a in args {
            out.push(LateArg {
                name: None,
                arg: match a {
                    ArgEx::Val(v) => CallArg::Val(self.emit_ex(v)),
                    ArgEx::Missing | ArgEx::RefCopy { .. } => CallArg::Missing,
                    ArgEx::Ref(t) => CallArg::Ref(self.emit_ref(t)),
                },
            });
        }
        let dst = want.then(|| self.reg(ty.clone()));
        self.emit(InstKind::Invoke {
            dst,
            receiver,
            member: Member::Named(name.clone()),
            kind: InvokeKind::Get,
            args: out,
            static_receiver: Some(lt.clone()),
            dispid: *dispid,
        });
        dst
    }

    /// Emits a statement-level call: the result, if any, is discarded.
    fn emit_call_stmt(&mut self, e: &Ex) {
        match e {
            Ex::LibCall { .. } => {
                self.emit_lib_call(e, false);
            }
            Ex::Call {
                callee, args, recv, ..
            } => {
                self.emit_call(callee, args, recv.as_deref(), None);
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
        // The member forms of a `With` block are the ordinary member forms
        // with the object of the block as receiver.
        let mapped;
        let ins = match with_dot_equivalent(body[i].1.opcode) {
            Some(plain) => {
                let Some(w) = self.with_stack.last() else {
                    return Err(self.no(body[i].1.opcode, "member form outside a With block"));
                };
                let w = w.clone();
                self.stack.push(w);
                mapped = Listed {
                    opcode: plain,
                    ..body[i].1.clone()
                };
                &mapped
            }
            None => &body[i].1,
        };
        let op = ins.opcode;
        // A conditional-compilation directive (`#If cond Then`, `#Else`,
        // `#End If`, `#Const`) has no effect on the code that was compiled:
        // the compiler took one branch. The directive is the marker, its
        // condition or value, and the instruction that closes it.
        if op == Opcode::PpMarker {
            let mut k = 1;
            while let Some((_, n)) = body.get(i + k) {
                k += 1;
                if matches!(
                    n.opcode,
                    Opcode::PpIf
                        | Opcode::PpElseif
                        | Opcode::PpElse
                        | Opcode::PpEndif
                        | Opcode::PpDefine
                ) {
                    return Ok(k);
                }
            }
            return Err(self.no(op, "conditional-compilation directive without its end"));
        }
        if is_layout_only(op) {
            // A statement marker starts a new statement of its own (needed
            // for `Resume Next` after the statement).
            if self.has_on_error && matches!(op, Opcode::StmtBegin | Opcode::StmtBeginImplicit) {
                self.begin_stmt();
            }
            return Ok(1);
        }
        if op.evidence() != Evidence::Verified && !self.accepted_without_verification(ins) {
            let shape = self
                .stack
                .last()
                .map_or_else(String::new, |e| format!(", receiver of type {:?}", e.ty()));
            return Err(self.no(
                op,
                format!("opcode is not verified ({:?}{shape})", op.evidence()),
            ));
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
                            | Opcode::WriteDotParams
                            | Opcode::WriteRefDot
                            | Opcode::WriteRefDotParams
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
                self.assign(ins, false)?;
                self.close_stmt();
            }
            Opcode::SetKeywordMarker => {
                self.set_marker = true;
            }
            Opcode::WriteRefSimple => {
                self.assign(ins, true)?;
                self.close_stmt();
            }
            Opcode::WriteRefDot => {
                self.dot_assign(ins, true, 0)?;
                self.close_stmt();
            }
            Opcode::ConstNullObject => {
                self.stack.push(Ex::Const(
                    Variant::Object(vba_value::ObjectRef::Nothing),
                    Type::Nothing,
                ));
            }
            Opcode::NewObject => {
                let e = self.new_object(ins)?;
                self.stack.push(e);
            }
            Opcode::TypeTest => {
                let ty = match self.object_class(ins)? {
                    ObjClass::Module(m) => Type::Class(m),
                    ObjClass::Builtin(lt) => Type::Library(lt),
                };
                let e = self.pop(ins)?;
                if !(e.ty().is_object() || e.ty().is_variant()) {
                    return Err(self.no(op, "TypeOf needs an object"));
                }
                self.stack.push(Ex::TypeOfIs { e: Box::new(e), ty });
            }
            Opcode::DebugAssert => {
                let c = self.pop(ins)?;
                self.expect_empty(ins)?;
                let Some(LibGlobal::Object(Type::Library(lt))) = BuiltinLibrary.global("Debug")
                else {
                    return Err(self.no(op, "no Debug object"));
                };
                let recv = Ex::LibObj {
                    name: "Debug".to_owned(),
                    lt: lt.clone(),
                };
                let call = self.lib_member(ins, recv, lt, "Assert", vec![c])?;
                self.emit_call_stmt(&call);
                self.close_stmt();
            }
            Opcode::CompareObjectIdentity => {
                let r = self.pop(ins)?;
                let l = self.pop(ins)?;
                if !(l.ty().is_object() || l.ty().is_variant())
                    || !(r.ty().is_object() || r.ty().is_variant())
                {
                    return Err(self.no(ins.opcode, "Is needs object operands"));
                }
                self.stack.push(Ex::Binary {
                    op: AstBin::Is,
                    ty: Type::Boolean,
                    l: Box::new(l),
                    r: Box::new(r),
                });
            }
            Opcode::ErrHandlerSet => {
                let mode = if ins.modifier == 1 {
                    ErrMode::ResumeNext
                } else if ins.modifier == 2 {
                    // `On Error GoTo 0`: errors propagate to the caller.
                    ErrMode::Propagate
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
                // The code after the jump is reached only through
                // `On Error Resume Next`, which skips error 20 (`Resume`
                // without an error) and continues with the next statement.
                self.terminate_and_continue(Terminator::Resume(ResumeKind::Block(b)));
                self.close_stmt();
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
                let (name, suffix) = self.name_of(ins)?;
                let recv = self.pop_receiver(ins)?;
                let e = self.dot_member(ins, recv, &name, suffix, Vec::new())?;
                self.stack.push(e);
            }
            Opcode::ReadDotParams => {
                let (name, suffix) = self.name_of(ins)?;
                let argc = match ins.operands.get(1) {
                    Some(Operand::Count(c)) => usize::from(*c),
                    _ => return Err(self.no(op, "missing argument count")),
                };
                let recv = self.pop_receiver(ins)?;
                let args = self.pop_n(ins, argc)?;
                let e = self.dot_member(ins, recv, &name, suffix, args)?;
                self.stack.push(e);
            }
            Opcode::InvokeDot => {
                let (name, suffix) = self.name_of(ins)?;
                let argc = match ins.operands.get(1) {
                    Some(Operand::Count(c)) => usize::from(*c),
                    _ => return Err(self.no(op, "missing argument count")),
                };
                let recv = self.pop_receiver(ins)?;
                let args = self.pop_n(ins, argc)?;
                let call = self.dot_member(ins, recv, &name, suffix, args)?;
                self.expect_empty(ins)?;
                self.emit_call_stmt(&call);
                self.close_stmt();
            }
            Opcode::WriteDot => {
                self.dot_assign(ins, false, 0)?;
                self.close_stmt();
            }
            Opcode::WriteDotParams | Opcode::WriteRefDotParams => {
                let argc = match ins.operands.get(1) {
                    Some(Operand::Count(c)) => usize::from(*c),
                    _ => return Err(self.no(op, "missing argument count")),
                };
                self.dot_assign(ins, op == Opcode::WriteRefDotParams, argc)?;
                self.close_stmt();
            }
            Opcode::ArgNamed => {
                let (name, _) = self.name_of(ins)?;
                let v = self.pop(ins)?;
                self.stack.push(Ex::Named(name, Box::new(v)));
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
                    Frame::For { exit, .. } | Frame::ForEach { exit, .. } => Some(*exit),
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
            Opcode::ForCollection => self.for_each_open(ins)?,
            Opcode::WithExprOpen => {}
            Opcode::WithOpen => self.with_open(ins)?,
            Opcode::WithClose => self.with_close(ins)?,
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
            Opcode::ErrHandlerSet => matches!(ins.modifier, 0..=2),
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
            Opcode::ReadDot => {
                matches!(
                    self.stack.last(),
                    Some(Ex::LibObj { .. } | Ex::Me(_) | Ex::Late { .. })
                ) || self.stack.last().is_some_and(|e| self.member_receiver(e))
            }
            Opcode::ReadDotParams
            | Opcode::InvokeDot
            | Opcode::WriteDot
            | Opcode::WriteDotParams
            | Opcode::WriteRefDot
            | Opcode::WriteRefDotParams => {
                matches!(self.stack.last(), Some(Ex::Me(_) | Ex::Late { .. }))
                    || self.stack.last().is_some_and(|e| self.member_receiver(e))
            }
            // `Set` statements, `New`.
            Opcode::SetKeywordMarker
            | Opcode::WriteRefSimple
            | Opcode::NewObject
            | Opcode::ForCollection
            | Opcode::WithExprOpen
            | Opcode::WithOpen
            | Opcode::TypeTest
            | Opcode::DebugAssert
            | Opcode::ArgNamed => true,
            _ => false,
        }
    }

    /// Whether a value is a receiver the member forms are lowered for: a
    /// value of a class of the project, a user-defined type or `Object`.
    fn member_receiver(&self, e: &Ex) -> bool {
        matches!(e, Ex::Ns(_))
            || matches!(
                e.ty(),
                Type::Class(_) | Type::Udt(_) | Type::Object | Type::Variant | Type::Library(_)
            )
    }

    /// `New T`: a class of the project, or the built-in `Collection`.
    fn new_object(&mut self, ins: &Listed) -> R<Ex> {
        match self.object_class(ins)? {
            ObjClass::Module(id) => {
                let m = self.cx.program.module(id);
                for p in [m.initialize, m.terminate].into_iter().flatten() {
                    self.note_callee(p);
                }
                Ok(Ex::New {
                    target: NewTarget::Class(id),
                    ty: Type::Class(id),
                })
            }
            ObjClass::Builtin(lt) => Ok(Ex::New {
                target: NewTarget::Library(lt.clone()),
                ty: Type::Library(lt),
            }),
        }
    }

    /// The class an object-table operand names: a class module of the
    /// project, or the built-in `Collection`.
    fn object_class(&mut self, ins: &Listed) -> R<ObjClass> {
        let (name, qualifier) = match ins.operands.first() {
            Some(Operand::Object { class: Some(n), .. }) => (n.spelling(), None),
            Some(Operand::Object {
                qualified: Some(q), ..
            }) => (q.name.spelling(), q.qualifier.spelling()),
            _ => (None, None),
        };
        let Some(name) = name else {
            return Err(self.no(ins.opcode, "class of New does not resolve"));
        };
        if let Some(q) = qualifier {
            let own = self.cx.project.is_some_and(|p| p.eq_ignore_ascii_case(q));
            let vba = q.eq_ignore_ascii_case("VBA");
            if !own && !vba {
                return Err(self.no(ins.opcode, "New of a library-qualified class"));
            }
        }
        let program = self.cx.program;
        if let Some(m) = program.module_by_name(name)
            && m.is_class_like()
        {
            return Ok(ObjClass::Module(m.id));
        }
        if name.eq_ignore_ascii_case("Collection") {
            return Ok(ObjClass::Builtin(collection_type()));
        }
        Err(self.no(ins.opcode, "class that is not part of the project"))
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
                Ex::Named(n, inner) => LateEx::Named(
                    n.clone(),
                    Box::new(
                        Self::late_args(vec![(**inner).clone()])
                            .pop()
                            .unwrap_or(LateEx::Missing),
                    ),
                ),
                Ex::Local { bare: true, .. }
                | Ex::Global { bare: true, .. }
                | Ex::Field { .. }
                | Ex::UdtField { .. } => match place_of(&a) {
                    Some(p) => LateEx::Ref(p),
                    None => LateEx::Val(a),
                },
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
            Some(Symbol::EnumMember(e, i)) if args.is_empty() => {
                let v = self.cx.program.enums[e.index()].members[i as usize].1;
                return Ok(Ex::Const(Variant::Long(v), Type::Enum(e)));
            }
            Some(Symbol::Var(_) | Symbol::Const(_) | Symbol::EnumMember(..)) => {
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
        if args.is_empty()
            && let Some(LibGlobal::Function(m)) = BuiltinLibrary.global(name)
            && m.params.iter().all(|p| p.optional)
        {
            return self.runtime_call(ins, &m, args);
        }
        // A library enum qualifies its members (`VbQueryClose.vbAppWindows`),
        // ahead of the members of the host base.
        if receiver
            && args.is_empty()
            && self.module_symbol(name).is_none()
            && matches!(
                vba_sema::TypeProvider::resolve_declared(&BuiltinLibrary, None, name),
                Some(Type::Long)
            )
        {
            return Ok(Ex::Ns(Ns::Vba));
        }
        if self.is_host_name(name, !receiver && args.is_empty()) {
            return Ok(self.host_member(name, args));
        }
        if args.is_empty() {
            let program = self.cx.program;
            if let Some(m) = program.module_by_name(name) {
                let id = m.id;
                if m.is_class_like() {
                    if m.predeclared {
                        return Ok(Ex::DefaultInstance(id));
                    }
                } else if receiver {
                    return Ok(Ex::Ns(Ns::Module(id)));
                }
            }
            if receiver && (name.eq_ignore_ascii_case("VBA") || is_sublibrary(name)) {
                return Ok(Ex::Ns(Ns::Vba));
            }
            if receiver {
                if let Some(e) = self.enum_by_name(name) {
                    return Ok(Ex::Ns(Ns::Enum(e)));
                }
                if self
                    .cx
                    .project
                    .is_some_and(|p| p.eq_ignore_ascii_case(name))
                {
                    return Ok(Ex::Ns(Ns::Project));
                }
            }
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
    fn dot_member(
        &mut self,
        ins: &Listed,
        recv: Ex,
        name: &str,
        suffix: Option<char>,
        args: Vec<Ex>,
    ) -> R<Ex> {
        match recv {
            Ex::Me(_) => self.me_member(ins, name, args),
            Ex::Late { .. } => Ok(Ex::Late {
                recv: Box::new(recv),
                name: name.to_owned(),
                args: Self::late_args(args),
            }),
            Ex::Ns(ns) => self.ns_member(ins, ns, name, suffix, args),
            other => match other.ty() {
                Type::Class(m) => self.class_member(ins, other, m, name, args),
                Type::Udt(u) => self.udt_member(ins, other, u, name, args),
                Type::Library(lt) => self.lib_member(ins, other, lt, name, args),
                Type::Object | Type::Variant => Ok(Ex::Late {
                    recv: Box::new(other),
                    name: name.to_owned(),
                    args: Self::late_args(args),
                }),
                _ => Err(self.no(
                    ins.opcode,
                    "member access outside the Err object, Me, objects and user types",
                )),
            },
        }
    }

    /// `Qualifier.name[(args)]`: a member of a standard module, of the
    /// default instance of a class, a member of an enum, a module of the
    /// project.
    fn ns_member(
        &mut self,
        ins: &Listed,
        ns: Ns,
        name: &str,
        suffix: Option<char>,
        args: Vec<Ex>,
    ) -> R<Ex> {
        let program = self.cx.program;
        let key = vba_ast::Name::new(name);
        match ns {
            Ns::Vba => {
                if args.is_empty()
                    && (is_sublibrary(name)
                        || matches!(
                            vba_sema::TypeProvider::resolve_declared(&BuiltinLibrary, None, name),
                            Some(Type::Long)
                        ))
                {
                    return Ok(Ex::Ns(Ns::Vba));
                }
                self.library_name(ins, name, suffix, args)
            }
            Ns::Project => match program.module_by_name(name) {
                Some(m) if args.is_empty() => Ok(self.module_ns(m.id)),
                _ => Err(self.no(ins.opcode, "project has no such module")),
            },
            Ns::Enum(e) => {
                let info = &program.enums[e.index()];
                match info.members.iter().find(|(n, _)| *n == key) {
                    Some((_, v)) if args.is_empty() => {
                        Ok(Ex::Const(Variant::Long(*v), Type::Enum(e)))
                    }
                    _ => Err(self.no(ins.opcode, "enum has no such member")),
                }
            }
            Ns::Module(m) => {
                let mi = program.module(m);
                if mi.is_class_like() {
                    if !mi.predeclared {
                        return Err(self.no(ins.opcode, "class used without an instance"));
                    }
                    return self.class_member(ins, Ex::DefaultInstance(m), m, name, args);
                }
                let own = m == self.sema.module;
                match mi.symbols.get(&key).map(|(s, _)| *s) {
                    Some(Symbol::Proc(id)) => {
                        self.note_callee(id);
                        self.project_call_on(ins, None, id, args)
                    }
                    Some(Symbol::Property(grp)) => match grp.get {
                        Some(id) => {
                            self.note_callee(id);
                            self.project_call_on(ins, None, id, args)
                        }
                        None => Err(self.no(ins.opcode, "property without a Get")),
                    },
                    Some(Symbol::Var(g)) if args.is_empty() => {
                        let info = program.global(g);
                        let public = matches!(
                            info.visibility,
                            vba_ast::Visibility::Public
                                | vba_ast::Visibility::Global
                                | vba_ast::Visibility::Friend
                        );
                        if !public && !own {
                            return Err(self.no(ins.opcode, "private variable of another module"));
                        }
                        Ok(self.global_ex(g))
                    }
                    Some(Symbol::Const(c)) if args.is_empty() => self.const_ex(ins, c),
                    Some(Symbol::EnumMember(e, i)) if args.is_empty() => {
                        let v = program.enums[e.index()].members[i as usize].1;
                        Ok(Ex::Const(Variant::Long(v), Type::Enum(e)))
                    }
                    Some(Symbol::Enum(e)) if args.is_empty() => Ok(Ex::Ns(Ns::Enum(e))),
                    _ => Err(self.no(
                        ins.opcode,
                        format!("module `{}` has no member `{name}`", mi.name),
                    )),
                }
            }
        }
    }

    /// A name of the built-in library, as `VBA.name` or bare: a constant, a
    /// global object, a function.
    fn library_name(
        &mut self,
        ins: &Listed,
        name: &str,
        suffix: Option<char>,
        args: Vec<Ex>,
    ) -> R<Ex> {
        let dollar = match suffix {
            None => false,
            Some('$') => true,
            Some(_) => return Err(self.no(ins.opcode, "type-suffixed name")),
        };
        if args.is_empty() && !dollar {
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
        self.intrinsic_call(ins, name, dollar, args)
    }

    /// A module name as a value or qualifier: a class with a default
    /// instance is that instance, a standard module qualifies.
    fn module_ns(&self, m: ModuleId) -> Ex {
        if self.cx.program.module(m).is_class_like() {
            Ex::DefaultInstance(m)
        } else {
            Ex::Ns(Ns::Module(m))
        }
    }

    /// `receiver.name[(args)]` for a built-in library object: the member is
    /// early bound with the arguments in parameter order, converted to the
    /// declared types, as the source binding does.
    fn lib_member(
        &mut self,
        ins: &Listed,
        recv: Ex,
        lt: LibraryType,
        name: &str,
        args: Vec<Ex>,
    ) -> R<Ex> {
        let Some(m) = BuiltinLibrary.member(&lt, name) else {
            return Err(self.no(ins.opcode, format!("`{name}` is not a known member")));
        };
        if m.kind == LibMemberKind::PropertyPut {
            return Err(self.no(ins.opcode, "write-only member read"));
        }
        if m.params.iter().any(|p| p.param_array || p.by_ref) {
            return Err(self.no(ins.opcode, "ByRef or ParamArray library member"));
        }
        let names: Vec<&str> = m.params.iter().map(|p| p.name.as_str()).collect();
        let args = self.arrange_args(ins, &names, args)?;
        if args.len() > m.params.len() {
            return Err(self.no(ins.opcode, "more arguments than the member declares"));
        }
        let mut given = args.into_iter();
        let mut out = Vec::new();
        for p in &m.params {
            match given.next() {
                None | Some(Ex::Missing) => {
                    if !p.optional {
                        return Err(
                            self.no(ins.opcode, "omitted argument for a required parameter")
                        );
                    }
                    match &p.default {
                        Some(v) if !matches!(v, Variant::Empty) => {
                            let c = Ex::Const(v.clone(), var_type_to_type(v.vartype()));
                            out.push(ArgEx::Val(self.coerce_arg(ins, c, &p.ty)?));
                        }
                        _ => out.push(ArgEx::Missing),
                    }
                }
                Some(a) => out.push(ArgEx::Val(self.coerce_arg(ins, a, &p.ty)?)),
            }
        }
        Ok(Ex::LibCall {
            recv: Box::new(recv),
            name: m.name.clone(),
            args: out,
            ty: if m.is_sub {
                Type::Variant
            } else {
                m.ty.clone()
            },
            lt,
            dispid: m.dispid,
        })
    }

    /// Records a call dependency: a procedure whose callee is not lowered is
    /// not lowered either.
    fn note_callee(&mut self, id: ProcId) {
        if !self.callees.contains(&id) {
            self.callees.push(id);
        }
    }

    /// `receiver.name[(args)]` for a value of a class of the project.
    fn class_member(
        &mut self,
        ins: &Listed,
        recv: Ex,
        m: ModuleId,
        name: &str,
        args: Vec<Ex>,
    ) -> R<Ex> {
        let program = self.cx.program;
        let key = vba_ast::Name::new(name);
        match program.module(m).symbols.get(&key).map(|(s, _)| *s) {
            Some(Symbol::Var(g)) if args.is_empty() => {
                let info = program.global(g);
                if !matches!(
                    info.visibility,
                    vba_ast::Visibility::Public
                        | vba_ast::Visibility::Global
                        | vba_ast::Visibility::Friend
                ) {
                    return Err(self.no(ins.opcode, "private field of another class"));
                }
                if info.with_events || info.auto_new {
                    return Err(self.no(ins.opcode, "WithEvents or As New field"));
                }
                Ok(Ex::Field {
                    recv: Box::new(recv),
                    global: g,
                    ty: info.ty.clone(),
                })
            }
            Some(Symbol::Proc(id)) => {
                self.note_callee(id);
                self.project_call_on(ins, Some(recv), id, args)
            }
            Some(Symbol::Property(grp)) => match grp.get {
                Some(id) => {
                    self.note_callee(id);
                    self.project_call_on(ins, Some(recv), id, args)
                }
                None => Err(self.no(ins.opcode, "property without a Get")),
            },
            Some(_) => Err(self.no(ins.opcode, "member of the class that is not callable")),
            None => Err(self.no(
                ins.opcode,
                format!(
                    "member `{}.{name}` of the class is not available",
                    program.module(m).name
                ),
            )),
        }
    }

    /// `value.field` for a user-defined type.
    fn udt_member(&mut self, ins: &Listed, base: Ex, u: UdtId, name: &str, args: Vec<Ex>) -> R<Ex> {
        if !args.is_empty() {
            return Err(self.no(ins.opcode, "indexing of a user-type field"));
        }
        let info = &self.cx.program.udts[u.index()];
        let key = vba_ast::Name::new(name);
        let Some(i) = info.fields.iter().position(|f| f.name == key) else {
            return Err(self.no(ins.opcode, "no such field in the user type"));
        };
        let ty = info.fields[i].ty.clone();
        Ok(Ex::UdtField {
            base: Box::new(base),
            udt: u,
            index: i as u32,
            ty,
        })
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

    fn assign(&mut self, ins: &Listed, set: bool) -> R<()> {
        let (name, _) = self.name_of(ins)?;
        let v = self.pop(ins)?;
        self.expect_empty(ins)?;
        if set {
            self.check_set_source(ins, &v)?;
        }
        self.store_name(ins, &name, v, set)
    }

    /// The right-hand side of a `Set` must be an object reference known at
    /// lowering time (a `Variant` is checked at run time by a call that is
    /// not lowered).
    fn check_set_source(&mut self, ins: &Listed, v: &Ex) -> R<()> {
        if !self.set_marker {
            return Err(self.no(ins.opcode, "reference assignment without a Set marker"));
        }
        self.set_marker = false;
        if v.ty().is_object() {
            Ok(())
        } else {
            Err(self.no(ins.opcode, "Set with a value that is not a known object"))
        }
    }

    /// Assigns `v` to a location of type `ty`: a value assignment for
    /// scalars, a reference assignment (`Set`) for objects.
    fn assign_to_place(&mut self, ins: &Listed, place: Place, ty: Type, v: Ex, set: bool) -> R<()> {
        let v = if set {
            if !(ty.is_object() || ty.is_variant()) {
                return Err(self.no(ins.opcode, "Set on a location that is not an object"));
            }
            self.coerce_object(ins, v, &ty, false)?
        } else {
            if v.ty().is_object()
                || ty.is_object()
                || matches!(ty, Type::Udt(_) | Type::Array { .. } | Type::FixedString(_))
                || matches!(v.ty(), Type::Udt(_) | Type::Array { .. })
            {
                return Err(self.no(ins.opcode, "object assignment"));
            }
            self.coerce(v, &ty)
        };
        let o = self.emit_ex(&v);
        self.store_place(&place, o, if set { StoreMode::Set } else { StoreMode::Let });
        Ok(())
    }

    /// Stores `v` into the variable or `Property Let` called `name`.
    fn store_name(&mut self, ins: &Listed, name: &str, v: Ex, set: bool) -> R<()> {
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
                Some(Symbol::Property(_)) => {
                    match self.proc_target(name, if set { Role::Set } else { Role::Let }) {
                        Some(p) => Target::Property(p),
                        None => return Err(self.no(ins.opcode, "property without a Let or Set")),
                    }
                }
                _ => {
                    return Err(self.no(
                        ins.opcode,
                        format!("assignment to `{name}`, which is not a variable"),
                    ));
                }
            }
        };
        let (ty, place) = match target {
            Target::Property(pid) => {
                return self.property_assign(ins, None, pid, Vec::new(), v, set);
            }
            Target::Local(l, t) => (t, Place::Local(l)),
            Target::Global(g, t) => (t, Place::Global(g)),
        };
        self.assign_to_place(ins, place, ty, v, set)
    }

    /// A `Property Let` or `Property Set` call with the value as its last
    /// argument. The value is evaluated before the receiver, as the source
    /// lowering evaluates the right-hand side of an assignment first.
    fn property_assign(
        &mut self,
        ins: &Listed,
        recv: Option<Ex>,
        pid: ProcId,
        idx: Vec<Ex>,
        v: Ex,
        set: bool,
    ) -> R<()> {
        if !set && v.ty().is_object() {
            return Err(self.no(ins.opcode, "object assignment"));
        }
        self.note_callee(pid);
        let callee = self.cx.program.proc(pid);
        let Some(last) = callee.params.last() else {
            return Err(self.no(ins.opcode, "property without a value parameter"));
        };
        let pty = last.ty.clone();
        let v = if set {
            self.coerce_object(ins, v, &pty, true)?
        } else {
            self.coerce_arg(ins, v, &pty)?
        };
        let vty = v.ty();
        let o = self.emit_ex(&v);
        let mut args = idx;
        args.push(Ex::Paren(Box::new(Ex::Emitted(o, vty))));
        let call = self.project_call_on(ins, recv, pid, args)?;
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
    fn dot_assign(&mut self, ins: &Listed, set: bool, argc: usize) -> R<()> {
        let name = self.name_of(ins)?.0;
        let recv = self.pop_receiver(ins)?;
        let idx = self.pop_n(ins, argc)?;
        let v = self.pop(ins)?;
        self.expect_empty(ins)?;
        if set {
            self.check_set_source(ins, &v)?;
        }
        if !idx.is_empty() {
            return self.dot_assign_indexed(ins, recv, &name, idx, v, set);
        }
        match recv {
            Ex::Ns(_) => Err(self.no(ins.opcode, "assignment to a member of a qualifying name")),
            Ex::Me(_) => {
                let key = vba_ast::Name::new(&name);
                let own = self.cx.program.module(self.sema.module);
                match own.symbols.get(&key).map(|(s, _)| *s) {
                    Some(Symbol::Var(g)) => {
                        let ty = self.cx.program.global(g).ty.clone();
                        self.assign_to_place(ins, Place::Global(g), ty, v, set)
                    }
                    Some(Symbol::Property(g)) => match if set { g.set } else { g.let_ } {
                        Some(p) => self.property_assign(ins, None, p, Vec::new(), v, set),
                        None => Err(self.no(ins.opcode, "property without a Let or Set")),
                    },
                    Some(_) => Err(self.no(ins.opcode, "assignment to a module member")),
                    None if self.cx.host_backed && !self.declared_in_module(&name) => {
                        self.late_put(ins, Ex::Me(self.sema.module), &name, v, set)
                    }
                    None => Err(self.no(ins.opcode, "member of the module that is not available")),
                }
            }
            Ex::Late { .. } => self.late_put(ins, recv, &name, v, set),
            other => match other.ty() {
                Type::Class(m) => self.class_assign(ins, other, m, &name, v, set),
                Type::Udt(u) => {
                    let field = self.udt_member(ins, other, u, &name, Vec::new())?;
                    let ty = field.ty();
                    match place_of(&field) {
                        Some(place) => self.assign_to_place(ins, place, ty, v, set),
                        None => {
                            Err(self.no(ins.opcode, "assignment to a value that is not a variable"))
                        }
                    }
                }
                Type::Object | Type::Variant => self.late_put(ins, other, &name, v, set),
                _ => Err(self.no(ins.opcode, "assignment to a member of this receiver")),
            },
        }
    }

    /// `receiver.name(idx) = v`: a property with parameters of a class of
    /// the project, or a late-bound member.
    fn dot_assign_indexed(
        &mut self,
        ins: &Listed,
        recv: Ex,
        name: &str,
        idx: Vec<Ex>,
        v: Ex,
        set: bool,
    ) -> R<()> {
        let program = self.cx.program;
        let key = vba_ast::Name::new(name);
        let class = match &recv {
            Ex::Me(m) => Some(*m),
            Ex::Ns(_) => return Err(self.no(ins.opcode, "indexed assignment to a qualifier")),
            other => match other.ty() {
                Type::Class(m) => Some(m),
                _ => None,
            },
        };
        if let Some(m) = class {
            return match program.module(m).symbols.get(&key).map(|(s, _)| *s) {
                Some(Symbol::Property(grp)) => match if set { grp.set } else { grp.let_ } {
                    Some(p) => {
                        let recv = if matches!(recv, Ex::Me(_)) {
                            None
                        } else {
                            Some(recv)
                        };
                        self.property_assign(ins, recv, p, idx, v, set)
                    }
                    None => Err(self.no(ins.opcode, "property without a Let or Set")),
                },
                _ => Err(self.no(
                    ins.opcode,
                    "indexed assignment to a member that is not a property",
                )),
            };
        }
        match recv.ty() {
            Type::Object | Type::Variant => self.late_put_indexed(ins, recv, name, idx, v, set),
            _ => Err(self.no(
                ins.opcode,
                "indexed assignment to a member of this receiver",
            )),
        }
    }

    /// `receiver.name(idx) = v` on a late-bound receiver.
    fn late_put_indexed(
        &mut self,
        ins: &Listed,
        recv: Ex,
        name: &str,
        idx: Vec<Ex>,
        v: Ex,
        set: bool,
    ) -> R<()> {
        if set {
            if !v.ty().is_object() {
                return Err(self.no(ins.opcode, "Set with a value that is not an object"));
            }
        } else if v.ty().is_object() {
            return Err(self.no(ins.opcode, "object assignment"));
        }
        let v = if set {
            v
        } else {
            self.coerce(v, &Type::Variant)
        };
        let vo = self.emit_ex(&v);
        let receiver = self.emit_ex(&recv);
        let mut args = Vec::new();
        for a in Self::late_args(idx) {
            args.push(self.late_arg(&a));
        }
        args.push(LateArg {
            name: None,
            arg: CallArg::Val(vo),
        });
        self.emit(InstKind::Invoke {
            dst: None,
            receiver,
            member: Member::Named(name.to_owned()),
            kind: if set {
                InvokeKind::PutRef
            } else {
                InvokeKind::Put
            },
            args,
            static_receiver: None,
            dispid: None,
        });
        Ok(())
    }

    /// `receiver.name = v` / `Set receiver.name = v` on a value of a class
    /// of the project.
    fn class_assign(
        &mut self,
        ins: &Listed,
        recv: Ex,
        m: ModuleId,
        name: &str,
        v: Ex,
        set: bool,
    ) -> R<()> {
        let program = self.cx.program;
        let key = vba_ast::Name::new(name);
        match program.module(m).symbols.get(&key).map(|(s, _)| *s) {
            Some(Symbol::Var(g)) => {
                let info = program.global(g);
                if !matches!(
                    info.visibility,
                    vba_ast::Visibility::Public
                        | vba_ast::Visibility::Global
                        | vba_ast::Visibility::Friend
                ) {
                    return Err(self.no(ins.opcode, "private field of another class"));
                }
                if info.with_events || info.auto_new {
                    return Err(self.no(ins.opcode, "WithEvents or As New field"));
                }
                let ty = info.ty.clone();
                let place = Place::Field {
                    recv: Box::new(recv),
                    global: g,
                };
                self.assign_to_place(ins, place, ty, v, set)
            }
            Some(Symbol::Property(grp)) => match if set { grp.set } else { grp.let_ } {
                Some(p) => self.property_assign(ins, Some(recv), p, Vec::new(), v, set),
                None => Err(self.no(ins.opcode, "property without a Let or Set")),
            },
            Some(_) => Err(self.no(ins.opcode, "member of the class that is not assignable")),
            None => Err(self.no(ins.opcode, "member of the class that is not available")),
        }
    }

    fn late_put(&mut self, ins: &Listed, recv: Ex, name: &str, v: Ex, set: bool) -> R<()> {
        if set {
            // An object reference into a late-bound member: passed on as it
            // is (`PutRef`).
            if !v.ty().is_object() {
                return Err(self.no(ins.opcode, "Set with a value that is not an object"));
            }
        } else if v.ty().is_object() {
            return Err(self.no(ins.opcode, "object assignment"));
        }
        let v = if set {
            v
        } else {
            self.coerce(v, &Type::Variant)
        };
        // The value is evaluated before the receiver: the compiled code
        // pushes it first, and the source lowering evaluates the right-hand
        // side of an assignment before the target.
        let vo = self.emit_ex(&v);
        let receiver = self.emit_ex(&recv);
        self.emit(InstKind::Invoke {
            dst: None,
            receiver,
            member: Member::Named(name.to_owned()),
            kind: if set {
                InvokeKind::PutRef
            } else {
                InvokeKind::Put
            },
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
            Some(Frame::ForEach { head, exit }) => {
                self.terminate(Terminator::Jump(head));
                self.switch_to(exit);
                Ok(())
            }
            _ => Err(self.no(op, "Next without For")),
        }
    }

    /// `For Each item In collection` (the loop header).
    fn for_each_open(&mut self, ins: &Listed) -> R<()> {
        let op = ins.opcode;
        let coll = self.pop(ins)?;
        self.expect_empty(ins)?;
        let Some(var) = self.for_var.take() else {
            return Err(self.no(op, "For Each without an item variable"));
        };
        let item_ty = var.ty();
        let Some(place) = place_of(&var) else {
            return Err(self.no(op, "For Each item is not a variable"));
        };
        if matches!(
            item_ty,
            Type::Udt(_) | Type::Array { .. } | Type::FixedString(_)
        ) {
            return Err(self.no(op, "For Each item of this type"));
        }
        // A class may be enumerated through an attribute that the compiled
        // code does not record; only collections and late-bound values are
        // lowered.
        if !matches!(coll.ty(), Type::Library(_) | Type::Object | Type::Variant) {
            return Err(self.no(op, "For Each over a value of this type"));
        }
        let c = self.emit_ex(&coll);
        let it = self.reg(Type::Variant);
        self.emit(InstKind::IterInit { dst: it, src: c });
        let head = self.new_block();
        let body_b = self.new_block();
        let exit = self.new_block();
        self.resume_at(body_b);
        self.jump_to(head);
        let elem = self.reg(Type::Variant);
        self.terminate(Terminator::ForEachNext {
            iter: it,
            item: elem,
            body: body_b,
            done: exit,
        });
        self.switch_to(body_b);
        let mut v = IrOperand::Reg(elem);
        if !(item_ty.is_variant() || item_ty == Type::Object) {
            let conv = self.reg(item_ty.clone());
            self.emit(InstKind::Convert {
                dst: conv,
                src: v,
                to: item_ty.clone(),
            });
            v = IrOperand::Reg(conv);
        }
        let mode = if item_ty.is_object() {
            StoreMode::Set
        } else {
            StoreMode::Let
        };
        self.store_place(&place, v, mode);
        self.frames.push(Frame::ForEach { head, exit });
        Ok(())
    }

    /// `With object` (after the object expression).
    fn with_open(&mut self, ins: &Listed) -> R<()> {
        let obj = self.pop(ins)?;
        self.expect_empty(ins)?;
        let ty = obj.ty();
        if !(ty.is_object() || ty.is_variant()) {
            return Err(self.no(ins.opcode, "With on a value that is not an object"));
        }
        let temp = self.new_temp(ty.clone());
        let o = self.emit_ex(&obj);
        self.emit(InstKind::StoreLocal {
            slot: temp,
            src: o,
            mode: if ty.is_object() {
                StoreMode::Set
            } else {
                StoreMode::Let
            },
        });
        let here = self.here();
        self.stmts[self.cur_stmt.0 as usize].resume_next = Some(here);
        self.frames.push(Frame::With { temp });
        self.with_stack.push(Ex::Local {
            id: temp,
            ty,
            bare: false,
        });
        Ok(())
    }

    fn with_close(&mut self, ins: &Listed) -> R<()> {
        match self.frames.pop() {
            Some(Frame::With { temp }) => {
                self.with_stack.pop();
                self.begin_stmt();
                self.emit(InstKind::StoreLocal {
                    slot: temp,
                    src: IrOperand::Const(Constant::Value(Variant::Object(
                        vba_value::ObjectRef::Nothing,
                    ))),
                    mode: StoreMode::Set,
                });
                Ok(())
            }
            _ => Err(self.no(ins.opcode, "End With without With")),
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
