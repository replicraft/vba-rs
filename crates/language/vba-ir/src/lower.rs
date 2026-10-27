//! Lowering of the bound model to the basic-block IR.

use std::collections::HashMap;

use vba_ast::{BinaryOp, CompareMode, ExitKind, UnaryOp};
use vba_sema::bound::*;
use vba_sema::{
    Analysis, GlobalInfo, LabelId, LocalId, LocalKind, ModuleId, ProcId, Program, Type,
};
use vba_source::Span;
use vba_value::Variant;
use vba_value::ops::{ArithOp, CompareOp, CompareOptions, LogicOp, Overflow, StringNumberRule};

use crate::ir::*;

/// Why lowering was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LowerError {
    /// The analysis reported errors; an erroneous program has no IR.
    #[error("the program has {0} semantic or syntax error(s); fix them before lowering")]
    HasErrors(usize),
    /// A module had syntax errors even though no diagnostic was an error
    /// (defensive).
    #[error("module `{0}` has syntax errors")]
    SyntaxErrors(String),
    /// A construct the lowering cannot express.
    #[error("cannot lower: {0}")]
    Unsupported(String),
    /// The program needs more instructions than
    /// [`LowerLimits::max_instructions`] allows.
    #[error("the program needs more than {limit} IR instructions; lowering was stopped")]
    TooLarge {
        /// The budget.
        limit: u64,
    },
}

/// Size budgets of lowering. `None` means unlimited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct LowerLimits {
    /// Instructions over the whole program. Checked at every statement and
    /// expression, so a hostile body stops soon after the budget is spent.
    pub max_instructions: Option<u64>,
}

impl LowerLimits {
    /// No budget.
    pub const fn unlimited() -> Self {
        LowerLimits {
            max_instructions: None,
        }
    }

    /// Sets [`max_instructions`](LowerLimits::max_instructions).
    #[must_use]
    pub const fn with_max_instructions(mut self, n: Option<u64>) -> Self {
        self.max_instructions = n;
        self
    }
}

/// Lowers an analysed project.
pub fn lower(analysis: &Analysis) -> Result<IrProgram, LowerError> {
    lower_with_limits(analysis, &LowerLimits::unlimited())
}

/// Lowers an analysed project within size budgets.
pub fn lower_with_limits(
    analysis: &Analysis,
    limits: &LowerLimits,
) -> Result<IrProgram, LowerError> {
    let errors = analysis
        .diagnostics
        .iter()
        .filter(|d| d.severity == vba_diagnostics::Severity::Error)
        .count();
    if errors > 0 {
        return Err(LowerError::HasErrors(errors));
    }
    let program = &analysis.program;
    for m in &program.modules {
        if m.has_syntax_errors {
            return Err(LowerError::SyntaxErrors(m.name.to_string()));
        }
    }
    let mut functions = Vec::new();
    let mut budget = Budget::new(limits);
    for p in &program.procs {
        functions.push(lower_proc(program, p.id, &mut budget)?);
    }
    let mut globals_of: Vec<Vec<&GlobalInfo>> = vec![Vec::new(); program.modules.len()];
    for g in &program.globals {
        globals_of[g.module.index()].push(g);
    }
    let mut module_init = vec![None; program.modules.len()];
    let mut classes = Vec::new();
    for m in &program.modules {
        if let Some(f) = lower_module_init(program, m.id, &globals_of[m.id.index()], &mut budget)? {
            let id = FuncId(functions.len() as u32);
            functions.push(f);
            module_init[m.id.index()] = Some(id);
        }
    }
    for m in &program.modules {
        if !m.is_class_like() {
            classes.push(None);
            continue;
        }
        classes.push(Some(ClassInfo {
            module: m.id,
            initialize: m.initialize,
            terminate: m.terminate,
            init: module_init[m.id.index()],
            fields: globals_of[m.id.index()].iter().map(|g| g.id).collect(),
            implements: m.implements.clone(),
            handlers: m.handlers.clone(),
            events: m.events.clone(),
            predeclared: m.predeclared,
        }));
    }
    Ok(IrProgram {
        program: program.clone(),
        functions,
        module_init,
        classes,
    })
}

fn lower_module_init(
    program: &Program,
    m: ModuleId,
    all_globals: &[&GlobalInfo],
    budget: &mut Budget,
) -> Result<Option<Function>, LowerError> {
    let mi = program.module(m);
    let globals: Vec<_> = all_globals
        .iter()
        .filter(|g| g.fixed_bounds.is_some())
        .collect();
    if globals.is_empty() {
        return Ok(None);
    }
    let mut b = Builder::new(program, m, None, format!("{}.$init", mi.name));
    b.cap = budget.remaining();
    b.begin_stmt(Span::empty(0));
    for g in globals {
        let place = if mi.is_class_like() {
            let me = b.reg(Type::Class(m));
            b.emit(InstKind::LoadMe { dst: me });
            RefPlace::Field {
                obj: Operand::Reg(me),
                global: g.id,
            }
        } else {
            RefPlace::Global(g.id)
        };
        let r = b.reg(g.ty.clone());
        b.emit(InstKind::MakeRef { dst: r, place });
        let elem = match &g.ty {
            Type::Array { elem, .. } => (**elem).clone(),
            t => t.clone(),
        };
        b.emit(InstKind::DimFixed {
            r,
            bounds: g.fixed_bounds.clone().unwrap_or_default(),
            elem,
        });
    }
    b.finish_stmt_simple();
    b.terminate(Terminator::Return);
    budget.spend(b.count)?;
    Ok(Some(b.finish(FuncKind::ModuleInit(m))))
}

/// What is left of [`LowerLimits::max_instructions`].
struct Budget {
    limit: Option<u64>,
    left: u64,
}

impl Budget {
    fn new(limits: &LowerLimits) -> Budget {
        Budget {
            limit: limits.max_instructions,
            left: limits.max_instructions.unwrap_or(u64::MAX),
        }
    }

    fn remaining(&self) -> u64 {
        self.left
    }

    fn spend(&mut self, n: u64) -> Result<(), LowerError> {
        match self.left.checked_sub(n) {
            Some(l) => {
                self.left = l;
                Ok(())
            }
            None => Err(LowerError::TooLarge {
                limit: self.limit.unwrap_or(0),
            }),
        }
    }

    fn too_large(&self) -> LowerError {
        LowerError::TooLarge {
            limit: self.limit.unwrap_or(0),
        }
    }
}

fn lower_proc(program: &Program, id: ProcId, budget: &mut Budget) -> Result<Function, LowerError> {
    let p = program.proc(id);
    let mi = program.module(p.module);
    let mut b = Builder::new(
        program,
        p.module,
        Some(id),
        format!("{}.{}", mi.name, p.name),
    );
    b.cap = budget.remaining();
    // Entry: allocate fixed-size local arrays.
    let entry_stmt = b.begin_stmt(p.span);
    for (i, l) in p.locals.iter().enumerate() {
        if let Some(bounds) = &l.fixed_bounds {
            let slot = LocalId(i as u32);
            let r = b.reg(l.ty.clone());
            b.emit(InstKind::MakeRef {
                dst: r,
                place: RefPlace::Local(slot),
            });
            let elem = match &l.ty {
                Type::Array { elem, .. } => (**elem).clone(),
                t => t.clone(),
            };
            b.emit(InstKind::DimFixed {
                r,
                bounds: bounds.clone(),
                elem,
            });
        }
    }
    let _ = entry_stmt;
    b.finish_stmt_simple();
    let lowered = b.lower_block(&p.body);
    budget.spend(b.count)?;
    lowered.map_err(|e| match e {
        LowerError::TooLarge { .. } => budget.too_large(),
        e => e,
    })?;
    b.terminate(Terminator::Return);
    Ok(b.finish(FuncKind::Proc(id)))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoopKind {
    For,
    Do,
    While,
}

struct LoopFrame {
    kind: LoopKind,
    exit: BlockId,
}

struct BlockBuf {
    insts: Vec<Inst>,
    term: Option<(Terminator, StmtId, Span)>,
}

struct Builder<'p> {
    prog: &'p Program,
    module: ModuleId,
    proc: Option<ProcId>,
    name: String,
    regs: Vec<Type>,
    blocks: Vec<BlockBuf>,
    stmts: Vec<StmtInfo>,
    cur: BlockId,
    cur_stmt: StmtId,
    cur_span: Span,
    label_blocks: HashMap<LabelId, BlockId>,
    loops: Vec<LoopFrame>,
    line: Option<u64>,
    /// Registers a `For`/`For Each` initialisation defines and its head
    /// uses (see [`InstKind::Undefined`]).
    loop_witnesses: Vec<Reg>,
    /// Instructions emitted so far.
    count: u64,
    /// Instructions this function may emit (the budget that is left).
    cap: u64,
}

impl<'p> Builder<'p> {
    fn new(prog: &'p Program, module: ModuleId, proc: Option<ProcId>, name: String) -> Self {
        let mut b = Builder {
            prog,
            module,
            proc,
            name,
            regs: Vec::new(),
            blocks: Vec::new(),
            stmts: Vec::new(),
            cur: BlockId(0),
            cur_stmt: StmtId(0),
            cur_span: Span::empty(0),
            label_blocks: HashMap::new(),
            loops: Vec::new(),
            loop_witnesses: Vec::new(),
            line: None,
            count: 0,
            cap: u64::MAX,
        };
        b.new_block();
        b
    }

    // ---- basic building ------------------------------------------------

    fn reg(&mut self, ty: Type) -> Reg {
        self.regs.push(ty);
        Reg(self.regs.len() as u32 - 1)
    }

    fn new_block(&mut self) -> BlockId {
        self.blocks.push(BlockBuf {
            insts: Vec::new(),
            term: None,
        });
        BlockId(self.blocks.len() as u32 - 1)
    }

    fn switch_to(&mut self, b: BlockId) {
        self.cur = b;
    }

    fn emit(&mut self, kind: InstKind) {
        let inst = Inst {
            span: self.cur_span,
            stmt: self.cur_stmt,
            kind,
        };
        self.blocks[self.cur.0 as usize].insts.push(inst);
        self.count += 1;
    }

    /// Fails once the instruction budget is spent.
    fn check_budget(&self) -> Result<(), LowerError> {
        if self.count > self.cap {
            Err(LowerError::TooLarge { limit: 0 })
        } else {
            Ok(())
        }
    }

    fn is_terminated(&self) -> bool {
        self.blocks[self.cur.0 as usize].term.is_some()
    }

    fn terminate(&mut self, t: Terminator) {
        if !self.is_terminated() {
            let (s, sp) = (self.cur_stmt, self.cur_span);
            self.blocks[self.cur.0 as usize].term = Some((t, s, sp));
        }
    }

    /// Ends the current block with a jump and continues in `b`.
    fn jump_to(&mut self, b: BlockId) {
        self.terminate(Terminator::Jump(b));
        self.switch_to(b);
    }

    /// Ends the current block and continues in a fresh (possibly dead) one.
    fn terminate_and_continue(&mut self, t: Terminator) {
        self.terminate(t);
        let n = self.new_block();
        self.switch_to(n);
    }

    fn begin_stmt(&mut self, span: Span) -> StmtId {
        self.stmts.push(StmtInfo {
            span,
            line: self.line,
            resume_next: None,
        });
        let id = StmtId(self.stmts.len() as u32 - 1);
        self.cur_stmt = id;
        self.cur_span = span;
        id
    }

    fn here(&self) -> Point {
        Point {
            block: self.cur,
            index: self.blocks[self.cur.0 as usize].insts.len() as u32,
        }
    }

    fn finish_stmt_simple(&mut self) {
        let p = self.here();
        self.stmts[self.cur_stmt.0 as usize].resume_next = Some(p);
    }

    fn label_block(&mut self, l: LabelId) -> BlockId {
        if let Some(b) = self.label_blocks.get(&l) {
            return *b;
        }
        let b = self.new_block();
        self.label_blocks.insert(l, b);
        b
    }

    fn const_op(v: Variant) -> Operand {
        Operand::Const(Constant::Value(v))
    }

    // ---- finishing ----------------------------------------------------

    fn finish(mut self, kind: FuncKind) -> Function {
        let mut blocks = std::mem::take(&mut self.blocks);
        cleanup(&mut blocks, &mut self.stmts);
        let order = layout(&blocks, &self.stmts);
        let n = blocks.len();
        let mut map: Vec<Option<BlockId>> = vec![None; n];
        for (new, old) in order.iter().enumerate() {
            map[*old] = Some(BlockId(new as u32));
        }
        let remap = |b: BlockId| map[b.0 as usize].expect("edge to a pruned block");
        let mut slots: Vec<Option<BlockBuf>> = blocks.into_iter().map(Some).collect();
        let mut out = Vec::new();
        for old in &order {
            let buf = slots[*old].take().expect("block placed once");
            let (mut term, term_stmt, term_span) =
                buf.term
                    .unwrap_or((Terminator::Unreachable, StmtId(0), Span::empty(0)));
            map_targets(&mut term, &mut |b| remap(b));
            let mut insts = buf.insts;
            map_handlers(&mut insts, &mut |b| remap(b));
            out.push(Block {
                insts,
                term,
                term_stmt,
                term_span,
            });
        }
        for s in &mut self.stmts {
            s.resume_next = s.resume_next.and_then(|p| {
                map[p.block.0 as usize].map(|b| Point {
                    block: b,
                    index: p.index,
                })
            });
        }
        // A loop whose initialisation is unreachable (only its body is
        // entered, through a label) lost the definitions of the registers
        // its head uses: define them as unset so `Next` raises error 92.
        let mut defined = vec![false; self.regs.len()];
        for b in &out {
            for i in &b.insts {
                if let Some(d) = crate::verify::inst_def(&i.kind) {
                    defined[d.0 as usize] = true;
                }
            }
            if let Terminator::ForEachNext { item, .. } = &b.term {
                defined[item.0 as usize] = true;
            }
        }
        let mut missing: Vec<Reg> = Vec::new();
        for r in std::mem::take(&mut self.loop_witnesses) {
            if !defined[r.0 as usize] && !missing.contains(&r) {
                missing.push(r);
            }
        }
        if !missing.is_empty() && !out.is_empty() {
            let k = missing.len() as u32;
            let undefined: Vec<Inst> = missing
                .iter()
                .map(|r| Inst {
                    span: Span::empty(0),
                    stmt: StmtId(0),
                    kind: InstKind::Undefined { dst: *r },
                })
                .collect();
            out[0].insts.splice(0..0, undefined);
            for s in &mut self.stmts {
                if let Some(p) = &mut s.resume_next
                    && p.block == BlockId(0)
                {
                    p.index += k;
                }
            }
        }
        // Slots mirror the sema locals.
        let slots = match self.proc {
            Some(p) => self
                .prog
                .proc(p)
                .locals
                .iter()
                .map(|l| Slot {
                    name: l.name.clone(),
                    ty: l.ty.clone(),
                    role: match l.kind {
                        LocalKind::Param(i) => SlotRole::Param(i),
                        LocalKind::Result => SlotRole::Result,
                        LocalKind::Var | LocalKind::Implicit => SlotRole::Local,
                        LocalKind::Temp => SlotRole::Temp,
                    },
                })
                .collect(),
            None => Vec::new(),
        };
        Function {
            kind,
            name: self.name,
            slots,
            regs: self.regs,
            blocks: out,
            stmts: self.stmts,
        }
    }

    // ---- statements ---------------------------------------------------

    fn lower_block(&mut self, stmts: &[BStmt]) -> Result<(), LowerError> {
        for s in stmts {
            self.lower_stmt(s)?;
        }
        Ok(())
    }

    fn lower_stmt(&mut self, s: &BStmt) -> Result<(), LowerError> {
        self.check_budget()?;
        let saved = (self.cur_stmt, self.cur_span);
        let id = self.begin_stmt(s.span);
        let mut simple = true;
        match &s.kind {
            BStmtKind::Label(l) => {
                let b = self.label_block(*l);
                self.jump_to(b);
                let info = self.prog.label(self.proc.expect("proc"), *l);
                if let Some(line) = info.line {
                    self.line = Some(line);
                    self.emit(InstKind::SetLine { line });
                    self.stmts[id.0 as usize].line = Some(line);
                }
            }
            BStmtKind::Assign {
                target,
                value,
                mode,
            } => {
                let v = self.lower_expr(value)?;
                let mode = self.store_mode(target, *mode);
                self.store(target, v, mode)?;
            }
            BStmtKind::Call(e) => {
                self.lower_discard(e)?;
            }
            BStmtKind::If { arms, else_body } => {
                simple = false;
                let end = self.new_block();
                for (i, (cond, body)) in arms.iter().enumerate() {
                    if i > 0 {
                        self.begin_stmt(cond.span);
                    }
                    let arm_stmt = self.cur_stmt;
                    let c = self.lower_expr(cond)?;
                    let then_b = self.new_block();
                    let else_b = self.new_block();
                    self.stmts[arm_stmt.0 as usize].resume_next = Some(Point {
                        block: then_b,
                        index: 0,
                    });
                    self.terminate(Terminator::Branch {
                        cond: c,
                        then_block: then_b,
                        else_block: else_b,
                    });
                    self.switch_to(then_b);
                    self.lower_block(body)?;
                    self.terminate(Terminator::Jump(end));
                    self.switch_to(else_b);
                }
                if let Some(b) = else_body {
                    self.lower_block(b)?;
                }
                self.terminate(Terminator::Jump(end));
                self.switch_to(end);
            }
            BStmtKind::Select { subject, cases } => {
                simple = false;
                self.lower_select(id, subject, cases)?;
            }
            BStmtKind::For {
                counter,
                counter_ty,
                start,
                end,
                step,
                body,
            } => {
                simple = false;
                self.lower_for(id, counter, counter_ty, start, end, step.as_ref(), body)?;
            }
            BStmtKind::ForEach {
                item,
                item_ty,
                collection,
                body,
            } => {
                simple = false;
                self.lower_for_each(id, item, item_ty, collection, body)?;
            }
            BStmtKind::Do {
                pre,
                post,
                body,
                is_while,
            } => {
                simple = false;
                self.lower_do(id, pre.as_ref(), post.as_ref(), body, *is_while)?;
            }
            BStmtKind::With {
                local,
                object,
                body,
            } => {
                simple = false;
                let o = self.lower_expr(object)?;
                self.emit(InstKind::StoreLocal {
                    slot: *local,
                    src: o,
                    mode: if object.ty.is_object() {
                        StoreMode::Set
                    } else {
                        StoreMode::Let
                    },
                });
                let here = self.here();
                self.stmts[id.0 as usize].resume_next = Some(here);
                self.lower_block(body)?;
                self.begin_stmt(s.span);
                self.emit(InstKind::StoreLocal {
                    slot: *local,
                    src: Self::const_op(Variant::Object(vba_value::ObjectRef::Nothing)),
                    mode: StoreMode::Set,
                });
            }
            BStmtKind::Block(body) => {
                simple = false;
                self.stmts[id.0 as usize].resume_next = Some(self.here());
                self.lower_block(body)?;
            }
            BStmtKind::GoTo(l) => {
                let b = self.label_block(*l);
                self.terminate_and_continue(Terminator::Jump(b));
            }
            BStmtKind::GoSub(l) => {
                let target = self.label_block(*l);
                let ret = self.new_block();
                self.terminate(Terminator::GoSub { target, ret });
                self.switch_to(ret);
            }
            BStmtKind::Return => self.terminate_and_continue(Terminator::ReturnFromGoSub),
            BStmtKind::OnError(a) => match a {
                BOnError::GoTo(l) => {
                    let h = self.label_block(*l);
                    self.emit(InstKind::SetErrorMode {
                        mode: ErrMode::Handler(h),
                    });
                }
                BOnError::Disable => self.emit(InstKind::SetErrorMode {
                    mode: ErrMode::Propagate,
                }),
                BOnError::ClearError => self.emit(InstKind::ClearError),
                BOnError::ResumeNext => self.emit(InstKind::SetErrorMode {
                    mode: ErrMode::ResumeNext,
                }),
            },
            BStmtKind::OnGoTo {
                selector,
                targets,
                is_gosub,
            } => {
                simple = false;
                let sel = self.lower_expr(selector)?;
                let after = self.new_block();
                let mut blocks = Vec::new();
                let mut tramps = Vec::new();
                for t in targets {
                    let tb = self.label_block(*t);
                    if *is_gosub {
                        let tramp = self.new_block();
                        tramps.push((tramp, tb));
                        blocks.push(tramp);
                    } else {
                        blocks.push(tb);
                    }
                }
                self.stmts[id.0 as usize].resume_next = Some(Point {
                    block: after,
                    index: 0,
                });
                self.terminate(Terminator::Switch {
                    selector: sel,
                    targets: blocks,
                    default: after,
                });
                for (tramp, tb) in tramps {
                    self.switch_to(tramp);
                    self.terminate(Terminator::GoSub {
                        target: tb,
                        ret: after,
                    });
                }
                self.switch_to(after);
            }
            BStmtKind::Resume(r) => {
                let k = match r {
                    BResume::Same => ResumeKind::Same,
                    BResume::Next => ResumeKind::Next,
                    BResume::Label(l) => ResumeKind::Block(self.label_block(*l)),
                };
                self.terminate_and_continue(Terminator::Resume(k));
            }
            BStmtKind::Exit(k) => match k {
                ExitKind::Sub | ExitKind::Function | ExitKind::Property => {
                    self.terminate_and_continue(Terminator::Return)
                }
                ExitKind::For | ExitKind::Do => {
                    let want = if *k == ExitKind::For {
                        LoopKind::For
                    } else {
                        LoopKind::Do
                    };
                    let exit = self
                        .loops
                        .iter()
                        .rev()
                        .find(|f| f.kind == want)
                        .map(|f| f.exit)
                        .ok_or_else(|| LowerError::Unsupported("Exit outside loop".into()))?;
                    self.terminate_and_continue(Terminator::Jump(exit));
                }
            },
            BStmtKind::End => self.terminate_and_continue(Terminator::End),
            BStmtKind::Stop => self.terminate_and_continue(Terminator::Stop),
            BStmtKind::ReDim { preserve, items } => {
                for it in items {
                    let mut bounds = Vec::new();
                    for b in &it.bounds {
                        let lo = match &b.lower {
                            Some(e) => Some(self.lower_expr(e)?),
                            None => None,
                        };
                        let hi = self.lower_expr(&b.upper)?;
                        bounds.push((lo, hi));
                    }
                    let r = self.place_ref(&it.target)?;
                    self.emit(InstKind::ReDim {
                        r,
                        preserve: *preserve,
                        bounds,
                        elem: it.elem.clone(),
                    });
                }
            }
            BStmtKind::Erase(ps) => {
                for p in ps {
                    let r = self.place_ref(p)?;
                    self.emit(InstKind::Erase { r });
                }
            }
            BStmtKind::Io(io) => self.lower_io(io)?,
            BStmtKind::RaiseEvent { event, args } => {
                let args = self.lower_args(args)?;
                self.emit(InstKind::RaiseEvent {
                    event: *event,
                    args,
                });
            }
            BStmtKind::DebugPrint(items) => {
                let items = self.lower_out(items)?;
                self.emit(InstKind::DebugPrint { items });
            }
        }
        if simple {
            self.finish_stmt_simple();
        }
        self.cur_stmt = saved.0;
        self.cur_span = saved.1;
        Ok(())
    }

    fn lower_out(&mut self, items: &[BOutItem]) -> Result<Vec<OutItem>, LowerError> {
        items
            .iter()
            .map(|i| {
                Ok(match i {
                    BOutItem::Value(e) => OutItem::Value(self.lower_expr(e)?),
                    BOutItem::Semicolon => OutItem::Semicolon,
                    BOutItem::Comma => OutItem::Comma,
                })
            })
            .collect()
    }

    fn opt(&mut self, e: &Option<BExpr>) -> Result<Option<Operand>, LowerError> {
        match e {
            Some(e) => Ok(Some(self.lower_expr(e)?)),
            None => Ok(None),
        }
    }

    fn lower_io(&mut self, io: &IoStmt) -> Result<(), LowerError> {
        let op = match io {
            IoStmt::Open {
                path,
                mode,
                access,
                lock,
                file,
                record_len,
            } => IoOp::Open {
                path: self.lower_expr(path)?,
                mode: *mode,
                access: *access,
                lock: *lock,
                file: self.lower_expr(file)?,
                record_len: self.opt(record_len)?,
            },
            IoStmt::Close(fs) => IoOp::Close(
                fs.iter()
                    .map(|f| self.lower_expr(f))
                    .collect::<Result<_, _>>()?,
            ),
            IoStmt::Print { file, items } => IoOp::Print {
                file: self.lower_expr(file)?,
                items: self.lower_out(items)?,
            },
            IoStmt::Write { file, items } => IoOp::Write {
                file: self.lower_expr(file)?,
                items: self.lower_out(items)?,
            },
            IoStmt::Input { file, targets } => {
                let file = self.lower_expr(file)?;
                let mut ts = Vec::new();
                for t in targets {
                    ts.push(self.place_ref(t)?);
                }
                IoOp::Input { file, targets: ts }
            }
            IoStmt::LineInput { file, target } => IoOp::LineInput {
                file: self.lower_expr(file)?,
                target: self.place_ref(target)?,
            },
            IoStmt::Get {
                file,
                record,
                target,
            } => IoOp::Get {
                file: self.lower_expr(file)?,
                record: self.opt(record)?,
                target: match target {
                    Some(t) => Some(self.place_ref(t)?),
                    None => None,
                },
            },
            IoStmt::Put {
                file,
                record,
                value,
            } => IoOp::Put {
                file: self.lower_expr(file)?,
                record: self.opt(record)?,
                value: self.opt(value)?,
            },
            IoStmt::Seek { file, position } => IoOp::Seek {
                file: self.lower_expr(file)?,
                position: self.opt(position)?,
            },
            IoStmt::Lock { file, from, to } => IoOp::Lock {
                file: self.lower_expr(file)?,
                from: self.opt(from)?,
                to: self.opt(to)?,
            },
            IoStmt::Unlock { file, from, to } => IoOp::Unlock {
                file: self.lower_expr(file)?,
                from: self.opt(from)?,
                to: self.opt(to)?,
            },
            IoStmt::Width { file, width } => IoOp::Width {
                file: self.lower_expr(file)?,
                width: self.opt(width)?,
            },
            IoStmt::Name { old, new } => IoOp::Name {
                old: self.lower_expr(old)?,
                new: self.lower_expr(new)?,
            },
        };
        self.emit(InstKind::Io(op));
        Ok(())
    }

    fn compare_opts(&self, mode: CompareMode, l: &Type, r: &Type) -> CompareOptions {
        let mode = match mode {
            CompareMode::Binary => vba_value::CompareMode::Binary,
            CompareMode::Text | CompareMode::Database => vba_value::CompareMode::Text,
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

    fn lower_select(
        &mut self,
        stmt: StmtId,
        subject: &BExpr,
        cases: &[BCase],
    ) -> Result<(), LowerError> {
        let subj = self.lower_expr(subject)?;
        let end = self.new_block();
        let compare = self.prog.module(self.module).options.compare;
        let mut else_body: Option<&BCase> = None;
        let body_blocks: Vec<BlockId> = cases
            .iter()
            .filter(|c| !c.is_else)
            .map(|_| self.new_block())
            .collect();
        self.stmts[stmt.0 as usize].resume_next = Some(Point {
            block: body_blocks.first().copied().unwrap_or(end),
            index: 0,
        });
        let mut next_body = body_blocks.iter();
        for case in cases {
            if case.is_else {
                else_body = Some(case);
                continue;
            }
            let body_b = *next_body.next().expect("one body block per case");
            let miss = self.new_block();
            let n = case.items.len();
            for (i, item) in case.items.iter().enumerate() {
                let next = if i + 1 == n { miss } else { self.new_block() };
                match item {
                    BCaseItem::Value(v) => {
                        let vo = self.lower_expr(v)?;
                        let c = self.cmp(CompareOp::Eq, &subj, &subject.ty, vo, &v.ty, compare);
                        self.terminate(Terminator::Branch {
                            cond: c,
                            then_block: body_b,
                            else_block: next,
                        });
                    }
                    BCaseItem::Is(op, v) => {
                        let vo = self.lower_expr(v)?;
                        let cop = compare_op(*op);
                        let c = self.cmp(cop, &subj, &subject.ty, vo, &v.ty, compare);
                        self.terminate(Terminator::Branch {
                            cond: c,
                            then_block: body_b,
                            else_block: next,
                        });
                    }
                    BCaseItem::Range(lo, hi) => {
                        let lo_o = self.lower_expr(lo)?;
                        let c1 = self.cmp(CompareOp::Ge, &subj, &subject.ty, lo_o, &lo.ty, compare);
                        let second = self.new_block();
                        self.terminate(Terminator::Branch {
                            cond: c1,
                            then_block: second,
                            else_block: next,
                        });
                        self.switch_to(second);
                        let hi_o = self.lower_expr(hi)?;
                        let c2 = self.cmp(CompareOp::Le, &subj, &subject.ty, hi_o, &hi.ty, compare);
                        self.terminate(Terminator::Branch {
                            cond: c2,
                            then_block: body_b,
                            else_block: next,
                        });
                    }
                }
                self.switch_to(next);
            }
            if n == 0 {
                self.terminate(Terminator::Jump(miss));
                self.switch_to(miss);
            }
            // `miss` is where the chain ended; emit the body out of line.
            let miss_cur = self.cur;
            self.switch_to(body_b);
            self.lower_block(&case.body)?;
            self.terminate(Terminator::Jump(end));
            self.switch_to(miss_cur);
        }
        if let Some(c) = else_body {
            self.lower_block(&c.body)?;
        }
        self.terminate(Terminator::Jump(end));
        self.switch_to(end);
        Ok(())
    }

    fn cmp(
        &mut self,
        op: CompareOp,
        l: &Operand,
        lt: &Type,
        r: Operand,
        rt: &Type,
        mode: CompareMode,
    ) -> Operand {
        let ty = if lt.is_variant() || rt.is_variant() {
            Type::Variant
        } else {
            Type::Boolean
        };
        let dst = self.reg(ty.clone());
        let opts = self.compare_opts(mode, lt, rt);
        self.emit(InstKind::Binary {
            dst,
            op: BinOp::Compare(op, opts),
            ty,
            overflow: overflow_for(lt, rt),
            lhs: l.clone(),
            rhs: r,
            guard: None,
        });
        Operand::Reg(dst)
    }

    #[allow(clippy::too_many_arguments)]
    fn lower_for(
        &mut self,
        stmt: StmtId,
        counter: &BPlace,
        counter_ty: &Type,
        start: &BExpr,
        end: &BExpr,
        step: Option<&BExpr>,
        body: &[BStmt],
    ) -> Result<(), LowerError> {
        let s = self.lower_expr(start)?;
        let e = self.lower_expr(end)?;
        let st = match step {
            Some(x) => self.lower_expr(x)?,
            None => Self::const_op(Variant::Integer(1)),
        };
        // Error 92 when `Next` is reached without this initialisation: a
        // register only the initialisation defines is the witness.
        let guard = match (&e, &st, &s) {
            (Operand::Reg(r), _, _) | (_, Operand::Reg(r), _) | (_, _, Operand::Reg(r)) => *r,
            _ => {
                let g = self.reg(Type::Boolean);
                self.emit(InstKind::Copy {
                    dst: g,
                    src: Self::const_op(Variant::Boolean(true)),
                });
                g
            }
        };
        for o in [&e, &st, &s] {
            if let Operand::Reg(r) = o {
                self.loop_witnesses.push(*r);
            }
        }
        self.loop_witnesses.push(guard);
        self.store(counter, s, StoreMode::Let)?;
        let head = self.new_block();
        let body_b = self.new_block();
        let next_b = self.new_block();
        let exit = self.new_block();
        self.stmts[stmt.0 as usize].resume_next = Some(Point {
            block: body_b,
            index: 0,
        });
        self.jump_to(head);
        let cur = self.load_place(counter, counter_ty)?;
        let c = self.reg(Type::Boolean);
        self.emit(InstKind::ForTest {
            dst: c,
            counter: cur,
            end: e,
            step: st.clone(),
        });
        self.terminate(Terminator::Branch {
            cond: Operand::Reg(c),
            then_block: body_b,
            else_block: exit,
        });
        self.switch_to(body_b);
        self.loops.push(LoopFrame {
            kind: LoopKind::For,
            exit,
        });
        self.lower_block(body)?;
        self.loops.pop();
        self.jump_to(next_b);
        self.begin_stmt(self.stmts[stmt.0 as usize].span);
        let cur = self.load_place(counter, counter_ty)?;
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
            lhs: cur,
            rhs: st,
            guard: Some(guard),
        });
        self.store(counter, Operand::Reg(nv), StoreMode::Let)?;
        self.terminate(Terminator::Jump(head));
        self.switch_to(exit);
        Ok(())
    }

    fn load_place(&mut self, p: &BPlace, ty: &Type) -> Result<Operand, LowerError> {
        let dst = self.reg(ty.clone());
        match p {
            BPlace::Local(l) => self.emit(InstKind::LoadLocal { dst, slot: *l }),
            BPlace::Global(g) => self.emit(InstKind::LoadGlobal { dst, global: *g }),
            BPlace::Field { receiver, global } => {
                let obj = self.lower_expr(receiver)?;
                self.emit(InstKind::LoadField {
                    dst,
                    obj,
                    global: *global,
                });
            }
            other => {
                let r = self.place_ref(other)?;
                self.emit(InstKind::LoadRef { dst, r });
            }
        }
        Ok(Operand::Reg(dst))
    }

    fn lower_for_each(
        &mut self,
        stmt: StmtId,
        item: &BPlace,
        item_ty: &Type,
        collection: &BExpr,
        body: &[BStmt],
    ) -> Result<(), LowerError> {
        let c = self.lower_expr(collection)?;
        let it = self.reg(Type::Variant);
        self.emit(InstKind::IterInit { dst: it, src: c });
        self.loop_witnesses.push(it);
        let head = self.new_block();
        let body_b = self.new_block();
        let exit = self.new_block();
        self.stmts[stmt.0 as usize].resume_next = Some(Point {
            block: body_b,
            index: 0,
        });
        self.jump_to(head);
        let elem = self.reg(Type::Variant);
        self.terminate(Terminator::ForEachNext {
            iter: it,
            item: elem,
            body: body_b,
            done: exit,
        });
        self.switch_to(body_b);
        let mut v = Operand::Reg(elem);
        if !(item_ty.is_variant() || *item_ty == Type::Object) {
            let conv = self.reg(item_ty.clone());
            self.emit(InstKind::Convert {
                dst: conv,
                src: v,
                to: item_ty.clone(),
            });
            v = Operand::Reg(conv);
        }
        let mode = if item_ty.is_object() {
            StoreMode::Set
        } else {
            StoreMode::Let
        };
        self.store(item, v, mode)?;
        self.loops.push(LoopFrame {
            kind: LoopKind::For,
            exit,
        });
        self.lower_block(body)?;
        self.loops.pop();
        self.terminate(Terminator::Jump(head));
        self.switch_to(exit);
        Ok(())
    }

    fn lower_do(
        &mut self,
        stmt: StmtId,
        pre: Option<&(bool, BExpr)>,
        post: Option<&(bool, BExpr)>,
        body: &[BStmt],
        is_while: bool,
    ) -> Result<(), LowerError> {
        let head = self.new_block();
        let body_b = self.new_block();
        let exit = self.new_block();
        self.stmts[stmt.0 as usize].resume_next = Some(Point {
            block: body_b,
            index: 0,
        });
        self.jump_to(head);
        if let Some((until, cond)) = pre {
            let c = self.lower_expr(cond)?;
            let (t, e) = if *until {
                (exit, body_b)
            } else {
                (body_b, exit)
            };
            self.terminate(Terminator::Branch {
                cond: c,
                then_block: t,
                else_block: e,
            });
        } else {
            self.terminate(Terminator::Jump(body_b));
        }
        self.switch_to(body_b);
        self.loops.push(LoopFrame {
            kind: if is_while {
                LoopKind::While
            } else {
                LoopKind::Do
            },
            exit,
        });
        self.lower_block(body)?;
        self.loops.pop();
        if let Some((until, cond)) = post {
            self.begin_stmt(cond.span);
            let c = self.lower_expr(cond)?;
            let (t, e) = if *until { (exit, head) } else { (head, exit) };
            self.terminate(Terminator::Branch {
                cond: c,
                then_block: t,
                else_block: e,
            });
        } else {
            self.terminate(Terminator::Jump(head));
        }
        self.switch_to(exit);
        Ok(())
    }

    // ---- stores and places ----------------------------------------------

    fn store_mode(&self, target: &BPlace, mode: AssignMode) -> StoreMode {
        match mode {
            AssignMode::Let => StoreMode::Let,
            AssignMode::LSet => StoreMode::LSet,
            AssignMode::RSet => StoreMode::RSet,
            AssignMode::Set => match target {
                BPlace::Global(g) | BPlace::Field { global: g, .. }
                    if self.prog.global(*g).with_events =>
                {
                    StoreMode::SetWithEvents
                }
                _ => StoreMode::Set,
            },
        }
    }

    fn store(&mut self, target: &BPlace, v: Operand, mode: StoreMode) -> Result<(), LowerError> {
        match target {
            BPlace::Local(l) => self.emit(InstKind::StoreLocal {
                slot: *l,
                src: v,
                mode,
            }),
            BPlace::Global(g) => self.emit(InstKind::StoreGlobal {
                global: *g,
                src: v,
                mode,
            }),
            BPlace::Field { receiver, global } => {
                let obj = self.lower_expr(receiver)?;
                self.emit(InstKind::StoreField {
                    obj,
                    global: *global,
                    src: v,
                    mode,
                });
            }
            BPlace::Index { .. } | BPlace::UdtField { .. } => {
                let r = self.place_ref(target)?;
                self.emit(InstKind::StoreRef { r, src: v, mode });
            }
            BPlace::Property { call, .. } => {
                let recv = match &call.receiver {
                    Some(r) => Some(self.lower_expr(r)?),
                    None => None,
                };
                let mut args = self.lower_args(&call.args)?;
                args.push(CallArg::Val(v));
                self.emit(InstKind::Call {
                    dst: None,
                    callee: callee_of(&call.target),
                    receiver: recv,
                    args,
                });
            }
            BPlace::Late {
                late,
                receiver_place,
                ..
            } => {
                let recv = match receiver_place {
                    Some(p) => Operand::Reg(self.place_ref(p)?),
                    None => self.lower_expr(&late.receiver)?,
                };
                let mut args = self.lower_late_args(&late.args)?;
                args.push(LateArg {
                    name: None,
                    arg: CallArg::Val(v),
                });
                self.emit(InstKind::Invoke {
                    dst: None,
                    receiver: recv,
                    member: member_of(&late.member),
                    kind: match late.kind {
                        LateKind::PropertyPutRef => InvokeKind::PutRef,
                        _ => InvokeKind::Put,
                    },
                    args,
                    static_receiver: late.static_receiver.clone(),
                    dispid: late.dispid,
                });
            }
        }
        Ok(())
    }

    fn place_ref(&mut self, p: &BPlace) -> Result<Reg, LowerError> {
        let place = match p {
            BPlace::Local(l) => RefPlace::Local(*l),
            BPlace::Global(g) => RefPlace::Global(*g),
            BPlace::Field { receiver, global } => RefPlace::Field {
                obj: self.lower_expr(receiver)?,
                global: *global,
            },
            BPlace::Index { base, indices, .. } => {
                let base = self.place_ref(base)?;
                let mut idx = Vec::new();
                for i in indices {
                    idx.push(self.lower_expr(i)?);
                }
                RefPlace::Elem { base, indices: idx }
            }
            BPlace::UdtField {
                base, udt, index, ..
            } => RefPlace::UdtField {
                base: self.place_ref(base)?,
                udt: *udt,
                index: *index,
            },
            BPlace::Property { .. } | BPlace::Late { .. } => {
                return Err(LowerError::Unsupported(
                    "a property cannot be passed or referenced by reference".into(),
                ));
            }
        };
        let ty = match p {
            BPlace::Local(l) => self
                .proc
                .map(|pid| self.prog.proc(pid).locals[l.index()].ty.clone())
                .unwrap_or(Type::Variant),
            BPlace::Global(g) | BPlace::Field { global: g, .. } => self.prog.global(*g).ty.clone(),
            BPlace::Index { elem, .. } => elem.clone(),
            BPlace::UdtField { ty, .. } => ty.clone(),
            _ => Type::Variant,
        };
        let dst = self.reg(ty);
        self.emit(InstKind::MakeRef { dst, place });
        Ok(dst)
    }

    // ---- expressions ------------------------------------------------------

    fn new_target(&self, t: &Type) -> Option<NewTarget> {
        match t {
            Type::Class(m) => Some(NewTarget::Class(*m)),
            Type::Library(l) => Some(NewTarget::Library(l.clone())),
            _ => None,
        }
    }

    fn ensure_new(&mut self, place: RefPlace, ty: &Type) {
        if let Some(target) = self.new_target(ty) {
            let r = self.reg(ty.clone());
            self.emit(InstKind::MakeRef { dst: r, place });
            self.emit(InstKind::EnsureNew { r, target });
        }
    }

    fn lower_args(&mut self, args: &[BArg]) -> Result<Vec<CallArg>, LowerError> {
        let mut out = Vec::new();
        for a in args {
            out.push(match a {
                BArg::Value(e) => CallArg::Val(self.lower_expr(e)?),
                BArg::Ref(p) => {
                    let r = self.place_ref(p)?;
                    self.ensure_new_ref(p, r);
                    CallArg::Ref(r)
                }
                BArg::Missing => CallArg::Missing,
            });
        }
        Ok(out)
    }

    /// Passing an `As New` variable by reference instantiates it first, as
    /// any other use of the variable does.
    fn ensure_new_ref(&mut self, p: &BPlace, r: Reg) {
        let ty = match p {
            BPlace::Local(l) => self
                .proc
                .map(|pid| &self.prog.proc(pid).locals[l.index()])
                .filter(|li| li.auto_new)
                .map(|li| li.ty.clone()),
            BPlace::Global(g) | BPlace::Field { global: g, .. } => {
                let gi = self.prog.global(*g);
                gi.auto_new.then(|| gi.ty.clone())
            }
            BPlace::Index { base, elem, .. } if self.auto_new_array_root(base) => {
                Some(elem.clone())
            }
            _ => None,
        };
        if let Some(target) = ty.and_then(|t| self.new_target(&t)) {
            self.emit(InstKind::EnsureNew { r, target });
        }
    }

    /// Whether the variable at the root of an element place is an array
    /// declared `As New`.
    fn auto_new_array_root(&self, p: &BPlace) -> bool {
        match p {
            BPlace::Local(l) => self
                .proc
                .is_some_and(|pid| self.prog.proc(pid).locals[l.index()].auto_new),
            BPlace::Global(g) | BPlace::Field { global: g, .. } => self.prog.global(*g).auto_new,
            _ => false,
        }
    }

    /// The place of the variable behind `e` when it is an array declared
    /// `As New` (a local, a module variable or a field of a plain receiver).
    fn auto_new_array_place(&mut self, e: &BExpr) -> Result<Option<RefPlace>, LowerError> {
        Ok(match &e.kind {
            BExprKind::Local(l) => {
                let auto = self
                    .proc
                    .is_some_and(|p| self.prog.proc(p).locals[l.index()].auto_new);
                auto.then_some(RefPlace::Local(*l))
            }
            BExprKind::Global(g) => self
                .prog
                .global(*g)
                .auto_new
                .then_some(RefPlace::Global(*g)),
            BExprKind::Field { receiver, global }
                if self.prog.global(*global).auto_new
                    && matches!(
                        receiver.kind,
                        BExprKind::Me | BExprKind::Local(_) | BExprKind::Global(_)
                    ) =>
            {
                Some(RefPlace::Field {
                    obj: self.lower_expr(receiver)?,
                    global: *global,
                })
            }
            _ => None,
        })
    }

    fn lower_late_args(&mut self, args: &[BLateArg]) -> Result<Vec<LateArg>, LowerError> {
        let mut out = Vec::new();
        for a in args {
            let arg = match &a.arg {
                BArg::Value(e) => CallArg::Val(self.lower_expr(e)?),
                BArg::Ref(p) => CallArg::Ref(self.place_ref(p)?),
                BArg::Missing => CallArg::Missing,
            };
            out.push(LateArg {
                name: a.name.as_ref().map(|n| n.to_string()),
                arg,
            });
        }
        Ok(out)
    }

    fn lower_discard(&mut self, e: &BExpr) -> Result<(), LowerError> {
        match &e.kind {
            BExprKind::Call(c) => {
                self.lower_call(c, &e.ty, false)?;
            }
            BExprKind::Late(l) => {
                self.lower_late(l, &e.ty, false)?;
            }
            _ => {
                self.lower_expr(e)?;
            }
        }
        Ok(())
    }

    fn lower_call(&mut self, c: &BCall, ty: &Type, want: bool) -> Result<Option<Reg>, LowerError> {
        let recv = match &c.receiver {
            Some(r) => Some(self.lower_expr(r)?),
            None => None,
        };
        let args = self.lower_args(&c.args)?;
        let dst = want.then(|| self.reg(ty.clone()));
        self.emit(InstKind::Call {
            dst,
            callee: callee_of(&c.target),
            receiver: recv,
            args,
        });
        Ok(dst)
    }

    fn lower_late(&mut self, l: &BLate, ty: &Type, want: bool) -> Result<Option<Reg>, LowerError> {
        let recv = self.lower_expr(&l.receiver)?;
        let args = self.lower_late_args(&l.args)?;
        let dst = want.then(|| self.reg(ty.clone()));
        self.emit(InstKind::Invoke {
            dst,
            receiver: recv,
            member: member_of(&l.member),
            kind: match l.kind {
                LateKind::Get => InvokeKind::Get,
                LateKind::PropertyPut => InvokeKind::Put,
                LateKind::PropertyPutRef => InvokeKind::PutRef,
            },
            args,
            static_receiver: l.static_receiver.clone(),
            dispid: l.dispid,
        });
        Ok(dst)
    }

    fn lower_expr(&mut self, e: &BExpr) -> Result<Operand, LowerError> {
        self.check_budget()?;
        let saved = self.cur_span;
        self.cur_span = e.span;
        let r = self.lower_expr_inner(e);
        self.cur_span = saved;
        r
    }

    fn lower_expr_inner(&mut self, e: &BExpr) -> Result<Operand, LowerError> {
        let ty = &e.ty;
        Ok(match &e.kind {
            BExprKind::Const(c) => Operand::Const(c.clone()),
            BExprKind::Local(l) => {
                let li = self.proc.map(|p| &self.prog.proc(p).locals[l.index()]);
                if let Some(li) = li
                    && li.auto_new
                {
                    self.ensure_new(RefPlace::Local(*l), &li.ty.clone());
                }
                let dst = self.reg(ty.clone());
                self.emit(InstKind::LoadLocal { dst, slot: *l });
                Operand::Reg(dst)
            }
            BExprKind::Global(g) => {
                if self.prog.global(*g).auto_new {
                    self.ensure_new(RefPlace::Global(*g), ty);
                }
                let dst = self.reg(ty.clone());
                self.emit(InstKind::LoadGlobal { dst, global: *g });
                Operand::Reg(dst)
            }
            BExprKind::Field { receiver, global } => {
                let obj = self.lower_expr(receiver)?;
                if self.prog.global(*global).auto_new {
                    self.ensure_new(
                        RefPlace::Field {
                            obj: obj.clone(),
                            global: *global,
                        },
                        ty,
                    );
                }
                let dst = self.reg(ty.clone());
                self.emit(InstKind::LoadField {
                    dst,
                    obj,
                    global: *global,
                });
                Operand::Reg(dst)
            }
            BExprKind::Me => {
                let dst = self.reg(ty.clone());
                self.emit(InstKind::LoadMe { dst });
                Operand::Reg(dst)
            }
            BExprKind::DefaultInstance(m) => {
                let dst = self.reg(ty.clone());
                self.emit(InstKind::LoadDefaultInstance { dst, module: *m });
                Operand::Reg(dst)
            }
            BExprKind::LibGlobal(name) => {
                let dst = self.reg(ty.clone());
                self.emit(InstKind::LoadLibGlobal {
                    dst,
                    name: name.clone(),
                });
                Operand::Reg(dst)
            }
            BExprKind::Unary { op, operand } => {
                let o = self.lower_expr(operand)?;
                let dst = self.reg(ty.clone());
                self.emit(InstKind::Unary {
                    dst,
                    op: match op {
                        UnaryOp::Neg => UnOp::Neg,
                        UnaryOp::Plus => UnOp::Plus,
                        UnaryOp::Not => UnOp::Not,
                    },
                    ty: ty.clone(),
                    overflow: if operand.ty.is_variant() {
                        Overflow::Promote
                    } else {
                        Overflow::Error
                    },
                    src: o,
                });
                Operand::Reg(dst)
            }
            BExprKind::Binary {
                op,
                lhs,
                rhs,
                compare,
            } => {
                let l = self.lower_expr(lhs)?;
                let r = self.lower_expr(rhs)?;
                let dst = self.reg(ty.clone());
                let bop = match op {
                    BinaryOp::Add => BinOp::Arith(ArithOp::Add),
                    BinaryOp::Sub => BinOp::Arith(ArithOp::Sub),
                    BinaryOp::Mul => BinOp::Arith(ArithOp::Mul),
                    BinaryOp::Div => BinOp::Arith(ArithOp::Div),
                    BinaryOp::IntDiv => BinOp::Arith(ArithOp::IntDiv),
                    BinaryOp::Mod => BinOp::Arith(ArithOp::Mod),
                    BinaryOp::Pow => BinOp::Arith(ArithOp::Pow),
                    BinaryOp::Concat => BinOp::Concat,
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Gt
                    | BinaryOp::Le
                    | BinaryOp::Ge => BinOp::Compare(
                        compare_op(*op),
                        self.compare_opts(*compare, &lhs.ty, &rhs.ty),
                    ),
                    BinaryOp::Like => BinOp::Like(*compare),
                    BinaryOp::Is => BinOp::Is,
                    BinaryOp::And => BinOp::Logic(LogicOp::And),
                    BinaryOp::Or => BinOp::Logic(LogicOp::Or),
                    BinaryOp::Xor => BinOp::Logic(LogicOp::Xor),
                    BinaryOp::Eqv => BinOp::Logic(LogicOp::Eqv),
                    BinaryOp::Imp => BinOp::Logic(LogicOp::Imp),
                };
                self.emit(InstKind::Binary {
                    dst,
                    op: bop,
                    ty: ty.clone(),
                    overflow: overflow_for(&lhs.ty, &rhs.ty),
                    lhs: l,
                    rhs: r,
                    guard: None,
                });
                Operand::Reg(dst)
            }
            BExprKind::Convert(inner) => {
                let o = self.lower_expr(inner)?;
                let dst = self.reg(ty.clone());
                self.emit(InstKind::Convert {
                    dst,
                    src: o,
                    to: ty.clone(),
                });
                Operand::Reg(dst)
            }
            BExprKind::Call(c) => match self.lower_call(c, ty, true)? {
                Some(r) => Operand::Reg(r),
                None => unreachable!("value call has a destination"),
            },
            BExprKind::Index { base, indices } => {
                let b = self.lower_expr(base)?;
                let mut idx = Vec::new();
                for i in indices {
                    idx.push(self.lower_expr(i)?);
                }
                // Elements of an array declared `As New` are instantiated on use.
                if self.new_target(ty).is_some()
                    && let Some(arr) = self.auto_new_array_place(base)?
                {
                    let arr_ref = self.reg(base.ty.clone());
                    self.emit(InstKind::MakeRef {
                        dst: arr_ref,
                        place: arr,
                    });
                    self.ensure_new(
                        RefPlace::Elem {
                            base: arr_ref,
                            indices: idx.clone(),
                        },
                        ty,
                    );
                }
                let dst = self.reg(ty.clone());
                self.emit(InstKind::Index {
                    dst,
                    array: b,
                    indices: idx,
                });
                Operand::Reg(dst)
            }
            BExprKind::UdtField { base, udt, index } => {
                let b = self.lower_expr(base)?;
                let dst = self.reg(ty.clone());
                self.emit(InstKind::UdtField {
                    dst,
                    base: b,
                    udt: *udt,
                    index: *index,
                });
                Operand::Reg(dst)
            }
            BExprKind::Late(l) => match self.lower_late(l, ty, true)? {
                Some(r) => Operand::Reg(r),
                None => unreachable!("value access has a destination"),
            },
            BExprKind::New(t) => {
                let dst = self.reg(ty.clone());
                self.emit(InstKind::New {
                    dst,
                    target: t.clone(),
                });
                Operand::Reg(dst)
            }
            BExprKind::AddressOf(t) => {
                let dst = self.reg(ty.clone());
                self.emit(InstKind::AddressOf {
                    dst,
                    func: match t {
                        AddressTarget::Proc(p) => FuncRef::Proc(*p),
                        AddressTarget::Declare(d) => FuncRef::Declare(*d),
                    },
                });
                Operand::Reg(dst)
            }
            BExprKind::TypeOfIs { expr, ty: t } => {
                let o = self.lower_expr(expr)?;
                let dst = self.reg(Type::Boolean);
                self.emit(InstKind::TypeOfIs {
                    dst,
                    src: o,
                    ty: t.clone(),
                });
                Operand::Reg(dst)
            }
        })
    }
}

fn compare_op(op: BinaryOp) -> CompareOp {
    match op {
        BinaryOp::Eq => CompareOp::Eq,
        BinaryOp::Ne => CompareOp::Ne,
        BinaryOp::Lt => CompareOp::Lt,
        BinaryOp::Gt => CompareOp::Gt,
        BinaryOp::Le => CompareOp::Le,
        _ => CompareOp::Ge,
    }
}

fn overflow_for(l: &Type, r: &Type) -> Overflow {
    if l.is_variant() || r.is_variant() {
        Overflow::Promote
    } else {
        Overflow::Error
    }
}

fn callee_of(t: &CallTarget) -> Callee {
    match t {
        CallTarget::Proc(p) => Callee::Proc(*p),
        CallTarget::Declare(d) => Callee::Declare(*d),
        CallTarget::Intrinsic { name, dollar } => Callee::Intrinsic {
            name: name.clone(),
            dollar: *dollar,
        },
    }
}

fn member_of(m: &LateMember) -> Member {
    match m {
        LateMember::Named(n) => Member::Named(n.to_string()),
        LateMember::Default => Member::Default,
    }
}

// ---- control-flow cleanup ------------------------------------------------

/// Applies `f` to every block target of a terminator.
fn map_targets(t: &mut Terminator, f: &mut dyn FnMut(BlockId) -> BlockId) {
    match t {
        Terminator::Jump(b) => *b = f(*b),
        Terminator::Branch {
            then_block,
            else_block,
            ..
        } => {
            *then_block = f(*then_block);
            *else_block = f(*else_block);
        }
        Terminator::Switch {
            targets, default, ..
        } => {
            for b in targets.iter_mut() {
                *b = f(*b);
            }
            *default = f(*default);
        }
        Terminator::GoSub { target, ret } => {
            *target = f(*target);
            *ret = f(*ret);
        }
        Terminator::ForEachNext { body, done, .. } => {
            *body = f(*body);
            *done = f(*done);
        }
        Terminator::Resume(ResumeKind::Block(b)) => *b = f(*b),
        _ => {}
    }
}

/// Applies `f` to the handler block of every `On Error GoTo`.
fn map_handlers(insts: &mut [Inst], f: &mut dyn FnMut(BlockId) -> BlockId) {
    for i in insts {
        if let InstKind::SetErrorMode {
            mode: ErrMode::Handler(h),
        } = &mut i.kind
        {
            *h = f(*h);
        }
    }
}

fn block_succs(b: &BlockBuf) -> Vec<BlockId> {
    let mut t = match &b.term {
        Some((t, ..)) => t.clone(),
        None => return Vec::new(),
    };
    let mut v = Vec::new();
    map_targets(&mut t, &mut |x| {
        v.push(x);
        x
    });
    v
}

fn block_handlers(b: &BlockBuf) -> Vec<BlockId> {
    b.insts
        .iter()
        .filter_map(|i| match i.kind {
            InstKind::SetErrorMode {
                mode: ErrMode::Handler(h),
            } => Some(h),
            _ => None,
        })
        .collect()
}

/// Whether the procedure ever sets `On Error Resume Next`.
fn has_resume_next(blocks: &[BlockBuf]) -> bool {
    blocks.iter().any(|b| {
        b.insts.iter().any(|i| {
            matches!(
                i.kind,
                InstKind::SetErrorMode {
                    mode: ErrMode::ResumeNext
                }
            )
        })
    })
}

/// The blocks control can enter dynamically from `b`: error handlers, and the
/// statement after a `Return` or `Resume` whose own error (3, 20) is skipped by
/// `Resume Next`.
fn block_dynamic(b: &BlockBuf, stmts: &[StmtInfo], resumable: bool) -> Vec<BlockId> {
    let mut v = block_handlers(b);
    if resumable
        && let Some((Terminator::ReturnFromGoSub | Terminator::Resume(_), stmt, _)) = &b.term
        && let Some(p) = stmts[stmt.0 as usize].resume_next
    {
        v.push(p.block);
    }
    v
}

/// Jump threading and block merging.
///
/// * a block that only jumps is bypassed;
/// * a block entered only by the unconditional jump of its single
///   predecessor is appended to that predecessor, unless control can also
///   reach it dynamically (error handlers, `GoSub` return points,
///   `Resume label` targets).
fn cleanup(blocks: &mut [BlockBuf], stmts: &mut [StmtInfo]) {
    let n = blocks.len();
    let resumable = has_resume_next(blocks);
    // Jump threading.
    let mut fwd: Vec<BlockId> = (0..n as u32).map(BlockId).collect();
    for (start, slot) in fwd.iter_mut().enumerate() {
        let mut cur = start;
        for _ in 0..n {
            let b = &blocks[cur];
            match &b.term {
                Some((Terminator::Jump(t), ..)) if b.insts.is_empty() && t.0 as usize != cur => {
                    cur = t.0 as usize;
                }
                _ => break,
            }
        }
        *slot = BlockId(cur as u32);
    }
    for b in blocks.iter_mut() {
        if let Some((t, ..)) = &mut b.term {
            map_targets(t, &mut |x| fwd[x.0 as usize]);
        }
        map_handlers(&mut b.insts, &mut |x| fwd[x.0 as usize]);
    }
    for s in stmts.iter_mut() {
        if let Some(p) = &mut s.resume_next
            && p.index == 0
        {
            p.block = fwd[p.block.0 as usize];
        }
    }
    // Reachability and predecessor counts.
    let mut reach = vec![false; n];
    let mut stack = vec![0usize];
    reach[0] = true;
    while let Some(i) = stack.pop() {
        let mut next = block_succs(&blocks[i]);
        next.extend(block_dynamic(&blocks[i], stmts, resumable));
        for b in next {
            if !reach[b.0 as usize] {
                reach[b.0 as usize] = true;
                stack.push(b.0 as usize);
            }
        }
    }
    let mut preds = vec![0u32; n];
    let mut pinned = vec![false; n];
    pinned[0] = true;
    for (i, b) in blocks.iter().enumerate() {
        if !reach[i] {
            continue;
        }
        for s in block_succs(b) {
            preds[s.0 as usize] += 1;
        }
        for h in block_dynamic(b, stmts, resumable) {
            pinned[h.0 as usize] = true;
        }
        match &b.term {
            Some((Terminator::GoSub { ret, .. }, ..)) => pinned[ret.0 as usize] = true,
            Some((Terminator::Resume(ResumeKind::Block(t)), ..)) => pinned[t.0 as usize] = true,
            _ => {}
        }
    }
    // Merging.
    let mut merged_into: Vec<Option<(usize, usize)>> = vec![None; n]; // (a, offset)
    for a in 0..n {
        if !reach[a] {
            continue;
        }
        loop {
            if merged_into[a].is_some() {
                break;
            }
            let Some((Terminator::Jump(t), ..)) = &blocks[a].term else {
                break;
            };
            let t = t.0 as usize;
            if t == a || pinned[t] || preds[t] != 1 || merged_into[t].is_some() {
                break;
            }
            let offset = blocks[a].insts.len();
            let moved = std::mem::take(&mut blocks[t].insts);
            blocks[a].insts.extend(moved);
            blocks[a].term = blocks[t].term.take();
            merged_into[t] = Some((a, offset));
            reach[t] = false;
        }
    }
    for s in stmts.iter_mut() {
        if let Some(p) = &mut s.resume_next {
            // Follow chains of merges (a block may itself have absorbed
            // others).
            let mut b = p.block.0 as usize;
            let mut idx = p.index;
            while let Some((a, off)) = merged_into[b] {
                idx += off as u32;
                b = a;
            }
            *p = Point {
                block: BlockId(b as u32),
                index: idx,
            };
        }
    }
}

/// The order of the reachable blocks: reverse postorder of the main flow
/// followed by the flows only reachable through error handlers.
fn layout(blocks: &[BlockBuf], stmts: &[StmtInfo]) -> Vec<usize> {
    let n = blocks.len();
    let resumable = has_resume_next(blocks);
    let mut seen = vec![false; n];
    let mut order: Vec<usize> = Vec::new();
    let mut roots: Vec<usize> = vec![0];
    let mut ri = 0;
    while ri < roots.len() {
        let root = roots[ri];
        ri += 1;
        if seen[root] {
            continue;
        }
        // Iterative DFS; successors are explored in source order.
        let mut post: Vec<usize> = Vec::new();
        let mut stack: Vec<(usize, Vec<BlockId>)> = Vec::new();
        seen[root] = true;
        let mut first = block_succs(&blocks[root]);
        first.reverse();
        roots.extend(
            block_dynamic(&blocks[root], stmts, resumable)
                .iter()
                .map(|h| h.0 as usize),
        );
        stack.push((root, first));
        while let Some((b, succs)) = stack.last_mut() {
            if let Some(next) = succs.pop() {
                let ni = next.0 as usize;
                if !seen[ni] {
                    seen[ni] = true;
                    roots.extend(
                        block_dynamic(&blocks[ni], stmts, resumable)
                            .iter()
                            .map(|h| h.0 as usize),
                    );
                    let mut s = block_succs(&blocks[ni]);
                    s.reverse();
                    stack.push((ni, s));
                }
            } else {
                post.push(*b);
                stack.pop();
            }
        }
        post.reverse();
        order.extend(post);
    }
    order
}
