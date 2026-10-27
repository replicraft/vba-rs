//! The IR verifier.
//!
//! Checked invariants:
//!
//! * structure: functions line up with procedures, block 0 exists, every
//!   block, statement, slot, global, procedure, event and UDT identifier is
//!   in range, resume points lie inside their blocks;
//! * registers: every register is defined exactly once and every use is
//!   dominated by its definition along all normal control-flow paths
//!   (error-handler entries start with no registers, `Resume` edges are
//!   not normal edges);
//! * calls: argument counts match the callee signature, receivers are present
//!   for class members and a result register exists only for callees that
//!   return a value;
//! * conditions: branch conditions are `Boolean` or `Variant`;
//! * `For Each`: the iterator register is defined by `IterInit`.

use crate::ir::*;
use vba_sema::{ProcId, Type};

/// A violated invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyError {
    /// Function name.
    pub function: String,
    /// Block, if the problem is local to one.
    pub block: Option<BlockId>,
    /// Description.
    pub message: String,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.block {
            Some(b) => write!(f, "{} b{}: {}", self.function, b.0, self.message),
            None => write!(f, "{}: {}", self.function, self.message),
        }
    }
}

/// Verifies a whole program; returns all violations found.
pub fn verify(p: &IrProgram) -> Result<(), Vec<VerifyError>> {
    let mut errs = Vec::new();
    if p.functions.len() < p.program.procs.len() {
        errs.push(VerifyError {
            function: "<program>".into(),
            block: None,
            message: "fewer functions than procedures".into(),
        });
        return Err(errs);
    }
    if p.classes.len() != p.program.modules.len() || p.module_init.len() != p.program.modules.len()
    {
        errs.push(VerifyError {
            function: "<program>".into(),
            block: None,
            message: "class/module tables do not match the module list".into(),
        });
    }
    for (i, f) in p.functions.iter().enumerate() {
        if i < p.program.procs.len() && f.kind != FuncKind::Proc(ProcId(i as u32)) {
            errs.push(VerifyError {
                function: f.name.clone(),
                block: None,
                message: "function i must implement procedure i".into(),
            });
        }
        verify_function(p, f, &mut errs);
    }
    verify_classes(p, &mut errs);
    for m in p.module_init.iter().flatten() {
        if m.0 as usize >= p.functions.len() {
            errs.push(VerifyError {
                function: "<program>".into(),
                block: None,
                message: "module initialiser out of range".into(),
            });
        }
    }
    if errs.is_empty() { Ok(()) } else { Err(errs) }
}

/// The class table refers to modules, procedures, functions, variables and
/// events by index; the interpreter indexes with them directly.
fn verify_classes(p: &IrProgram, errs: &mut Vec<VerifyError>) {
    let mut bad = |what: String| {
        errs.push(VerifyError {
            function: "<program>".into(),
            block: None,
            message: format!("class table: {what}"),
        });
    };
    let nprocs = p.program.procs.len();
    let nglobals = p.program.globals.len();
    for (i, ci) in p.classes.iter().enumerate() {
        let Some(ci) = ci else { continue };
        if ci.module.index() != i {
            bad(format!("entry {i} describes module {}", ci.module.0));
        }
        for (what, pid) in [("initialize", ci.initialize), ("terminate", ci.terminate)] {
            if pid.is_some_and(|q| q.index() >= nprocs) {
                bad(format!("{what} procedure of module {i} out of range"));
            }
        }
        if ci.init.is_some_and(|f| f.0 as usize >= p.functions.len()) {
            bad(format!("initialiser function of module {i} out of range"));
        }
        if ci.fields.iter().any(|g| g.index() >= nglobals) {
            bad(format!("field of module {i} is not a variable"));
        }
        if ci
            .events
            .iter()
            .any(|e| e.index() >= p.program.events.len())
        {
            bad(format!("event of module {i} out of range"));
        }
        for im in &ci.implements {
            if im
                .members
                .iter()
                .any(|(a, b)| a.index() >= nprocs || b.index() >= nprocs)
            {
                bad(format!("implemented member of module {i} out of range"));
            }
        }
        for h in &ci.handlers {
            if h.var.index() >= nglobals || h.proc.index() >= nprocs {
                bad(format!("event handler of module {i} out of range"));
            }
        }
    }
}

struct Ctx<'a> {
    p: &'a IrProgram,
    f: &'a Function,
    errs: &'a mut Vec<VerifyError>,
}

impl Ctx<'_> {
    fn err(&mut self, block: Option<BlockId>, msg: impl Into<String>) {
        self.errs.push(VerifyError {
            function: self.f.name.clone(),
            block,
            message: msg.into(),
        });
    }
}

fn op_regs(o: &Operand, out: &mut Vec<Reg>) {
    if let Operand::Reg(r) = o {
        out.push(*r);
    }
}

fn opt_regs(o: &Option<Operand>, out: &mut Vec<Reg>) {
    if let Some(o) = o {
        op_regs(o, out);
    }
}

fn arg_regs(a: &CallArg, out: &mut Vec<Reg>) {
    match a {
        CallArg::Val(o) => op_regs(o, out),
        CallArg::Ref(r) => out.push(*r),
        CallArg::Missing => {}
    }
}

fn out_regs(items: &[OutItem], out: &mut Vec<Reg>) {
    for i in items {
        if let OutItem::Value(o) = i {
            op_regs(o, out);
        }
    }
}

/// The registers read by an instruction.
pub fn inst_uses(k: &InstKind) -> Vec<Reg> {
    let mut u = Vec::new();
    match k {
        InstKind::Copy { src, .. } | InstKind::Convert { src, .. } => op_regs(src, &mut u),
        InstKind::Unary { src, .. } => op_regs(src, &mut u),
        InstKind::Binary { lhs, rhs, .. } => {
            op_regs(lhs, &mut u);
            op_regs(rhs, &mut u);
        }
        InstKind::LoadLocal { .. }
        | InstKind::LoadGlobal { .. }
        | InstKind::LoadMe { .. }
        | InstKind::LoadDefaultInstance { .. }
        | InstKind::LoadLibGlobal { .. }
        | InstKind::New { .. }
        | InstKind::AddressOf { .. }
        | InstKind::SetErrorMode { .. }
        | InstKind::ClearError
        | InstKind::Stop
        | InstKind::SetLine { .. }
        | InstKind::Undefined { .. } => {}
        InstKind::StoreLocal { src, .. } | InstKind::StoreGlobal { src, .. } => {
            op_regs(src, &mut u)
        }
        InstKind::LoadField { obj, .. } => op_regs(obj, &mut u),
        InstKind::StoreField { obj, src, .. } => {
            op_regs(obj, &mut u);
            op_regs(src, &mut u);
        }
        InstKind::MakeRef { place, .. } => match place {
            RefPlace::Local(_) | RefPlace::Global(_) => {}
            RefPlace::Field { obj, .. } => op_regs(obj, &mut u),
            RefPlace::Elem { base, indices } => {
                u.push(*base);
                for i in indices {
                    op_regs(i, &mut u);
                }
            }
            RefPlace::UdtField { base, .. } => u.push(*base),
        },
        InstKind::LoadRef { r, .. } => u.push(*r),
        InstKind::StoreRef { r, src, .. } => {
            u.push(*r);
            op_regs(src, &mut u);
        }
        InstKind::Index { array, indices, .. } => {
            op_regs(array, &mut u);
            for i in indices {
                op_regs(i, &mut u);
            }
        }
        InstKind::UdtField { base, .. } => op_regs(base, &mut u),
        InstKind::Call { receiver, args, .. } => {
            opt_regs(receiver, &mut u);
            for a in args {
                arg_regs(a, &mut u);
            }
        }
        InstKind::Invoke { receiver, args, .. } => {
            op_regs(receiver, &mut u);
            for a in args {
                arg_regs(&a.arg, &mut u);
            }
        }
        InstKind::EnsureNew { r, .. } | InstKind::Erase { r } => u.push(*r),
        InstKind::TypeOfIs { src, .. } => op_regs(src, &mut u),
        InstKind::ReDim { r, bounds, .. } => {
            u.push(*r);
            for (l, h) in bounds {
                opt_regs(l, &mut u);
                op_regs(h, &mut u);
            }
        }
        InstKind::DimFixed { r, .. } => u.push(*r),
        InstKind::IterInit { src, .. } => op_regs(src, &mut u),
        InstKind::ForTest {
            counter, end, step, ..
        } => {
            op_regs(counter, &mut u);
            op_regs(end, &mut u);
            op_regs(step, &mut u);
        }
        InstKind::RaiseEvent { args, .. } => {
            for a in args {
                arg_regs(a, &mut u);
            }
        }
        InstKind::DebugPrint { items } => out_regs(items, &mut u),
        InstKind::Io(op) => match op {
            IoOp::Open {
                path,
                file,
                record_len,
                ..
            } => {
                op_regs(path, &mut u);
                op_regs(file, &mut u);
                opt_regs(record_len, &mut u);
            }
            IoOp::Close(fs) => fs.iter().for_each(|f| op_regs(f, &mut u)),
            IoOp::Print { file, items } | IoOp::Write { file, items } => {
                op_regs(file, &mut u);
                out_regs(items, &mut u);
            }
            IoOp::Input { file, targets } => {
                op_regs(file, &mut u);
                u.extend(targets.iter().copied());
            }
            IoOp::LineInput { file, target } => {
                op_regs(file, &mut u);
                u.push(*target);
            }
            IoOp::Get {
                file,
                record,
                target,
            } => {
                op_regs(file, &mut u);
                opt_regs(record, &mut u);
                u.extend(target.iter().copied());
            }
            IoOp::Put {
                file,
                record,
                value,
            } => {
                op_regs(file, &mut u);
                opt_regs(record, &mut u);
                opt_regs(value, &mut u);
            }
            IoOp::Seek { file, position } => {
                op_regs(file, &mut u);
                opt_regs(position, &mut u);
            }
            IoOp::Lock { file, from, to } | IoOp::Unlock { file, from, to } => {
                op_regs(file, &mut u);
                opt_regs(from, &mut u);
                opt_regs(to, &mut u);
            }
            IoOp::Width { file, width } => {
                op_regs(file, &mut u);
                opt_regs(width, &mut u);
            }
            IoOp::Name { old, new } => {
                op_regs(old, &mut u);
                op_regs(new, &mut u);
            }
        },
    }
    u
}

/// The register written by an instruction.
pub fn inst_def(k: &InstKind) -> Option<Reg> {
    match k {
        InstKind::Copy { dst, .. }
        | InstKind::Unary { dst, .. }
        | InstKind::Binary { dst, .. }
        | InstKind::Convert { dst, .. }
        | InstKind::LoadLocal { dst, .. }
        | InstKind::LoadGlobal { dst, .. }
        | InstKind::LoadField { dst, .. }
        | InstKind::LoadMe { dst }
        | InstKind::LoadDefaultInstance { dst, .. }
        | InstKind::LoadLibGlobal { dst, .. }
        | InstKind::MakeRef { dst, .. }
        | InstKind::LoadRef { dst, .. }
        | InstKind::Index { dst, .. }
        | InstKind::UdtField { dst, .. }
        | InstKind::New { dst, .. }
        | InstKind::AddressOf { dst, .. }
        | InstKind::TypeOfIs { dst, .. }
        | InstKind::IterInit { dst, .. }
        | InstKind::Undefined { dst }
        | InstKind::ForTest { dst, .. } => Some(*dst),
        InstKind::Call { dst, .. } | InstKind::Invoke { dst, .. } => *dst,
        _ => None,
    }
}

/// Normal successor blocks of a terminator (without `Resume` edges).
pub fn normal_successors(t: &Terminator) -> Vec<BlockId> {
    match t {
        Terminator::Jump(b) => vec![*b],
        Terminator::Branch {
            then_block,
            else_block,
            ..
        } => vec![*then_block, *else_block],
        Terminator::Switch {
            targets, default, ..
        } => {
            let mut v = targets.clone();
            v.push(*default);
            v
        }
        Terminator::GoSub { target, ret } => vec![*target, *ret],
        Terminator::ForEachNext { body, done, .. } => vec![*body, *done],
        _ => Vec::new(),
    }
}

fn term_uses(t: &Terminator) -> Vec<Reg> {
    let mut u = Vec::new();
    match t {
        Terminator::Branch { cond, .. } => op_regs(cond, &mut u),
        Terminator::Switch { selector, .. } => op_regs(selector, &mut u),
        Terminator::ForEachNext { iter, .. } => u.push(*iter),
        _ => {}
    }
    u
}

fn term_def(t: &Terminator) -> Option<Reg> {
    match t {
        Terminator::ForEachNext { item, .. } => Some(*item),
        _ => None,
    }
}

fn verify_function(p: &IrProgram, f: &Function, errs: &mut Vec<VerifyError>) {
    let mut cx = Ctx { p, f, errs };
    let nblocks = f.blocks.len();
    if nblocks == 0 {
        cx.err(None, "function has no blocks");
        return;
    }
    // Slots against the procedure.
    if let FuncKind::Proc(pid) = f.kind {
        if pid.index() >= p.program.procs.len() {
            cx.err(None, "procedure id out of range");
            return;
        }
        let pi = p.program.proc(pid);
        if f.slots.len() != pi.locals.len() {
            cx.err(None, "slot count differs from the procedure's locals");
        }
        for (i, prm) in pi.params.iter().enumerate() {
            match f.slots.get(i) {
                Some(s) if s.role == SlotRole::Param(i as u32) && s.ty == prm.ty => {}
                _ => cx.err(None, format!("slot {i} must be parameter {i}")),
            }
        }
        if pi.has_result && !f.slots.iter().any(|s| s.role == SlotRole::Result) {
            cx.err(None, "function without a result slot");
        }
    }
    // Register definitions: exactly once.
    let mut def_count = vec![0u32; f.regs.len()];
    let in_range = |r: Reg| (r.0 as usize) < f.regs.len();
    for (bi, b) in f.blocks.iter().enumerate() {
        let bid = BlockId(bi as u32);
        for inst in &b.insts {
            if inst.stmt.0 as usize >= f.stmts.len() {
                cx.err(Some(bid), "instruction statement id out of range");
            }
            if let Some(d) = inst_def(&inst.kind) {
                if in_range(d) {
                    def_count[d.0 as usize] += 1;
                } else {
                    cx.err(Some(bid), format!("register r{} out of range", d.0));
                }
            }
            for u in inst_uses(&inst.kind) {
                if !in_range(u) {
                    cx.err(Some(bid), format!("register r{} out of range", u.0));
                }
            }
            check_inst(&mut cx, bid, &inst.kind);
        }
        if b.term_stmt.0 as usize >= f.stmts.len() {
            cx.err(Some(bid), "terminator statement id out of range");
        }
        if let Some(d) = term_def(&b.term) {
            if in_range(d) {
                def_count[d.0 as usize] += 1;
            } else {
                cx.err(Some(bid), format!("register r{} out of range", d.0));
            }
        }
        for u in term_uses(&b.term) {
            if !in_range(u) {
                cx.err(Some(bid), format!("register r{} out of range", u.0));
            }
        }
        for s in normal_successors(&b.term) {
            if s.0 as usize >= nblocks {
                cx.err(Some(bid), format!("jump to missing block b{}", s.0));
            }
        }
        if let Terminator::Resume(ResumeKind::Block(t)) = &b.term
            && t.0 as usize >= nblocks
        {
            cx.err(Some(bid), format!("resume to missing block b{}", t.0));
        }
        if let Terminator::Switch { targets, .. } = &b.term
            && targets.is_empty()
        {
            cx.err(Some(bid), "switch without targets");
        }
        if let Terminator::Branch { cond, .. } = &b.term
            && let Operand::Reg(r) = cond
            && in_range(*r)
            && !matches!(f.regs[r.0 as usize], Type::Boolean | Type::Variant)
        {
            cx.err(Some(bid), "branch condition must be Boolean or Variant");
        }
        if let Terminator::ForEachNext { iter, .. } = &b.term {
            // The iterator is defined by `iter.init`. The definition may be
            // missing altogether when the `For Each` statement is only
            // reachable through a label in its body (`GoTo` into the body);
            // the run time then raises error 92 at `Next`.
            let defined_otherwise = f.blocks.iter().flat_map(|b| &b.insts).any(|i| {
                inst_def(&i.kind) == Some(*iter)
                    && !matches!(
                        i.kind,
                        InstKind::IterInit { .. } | InstKind::Undefined { .. }
                    )
            });
            if defined_otherwise {
                cx.err(
                    Some(bid),
                    "foreach.next iterator is not defined by iter.init",
                );
            }
        }
    }
    for (r, c) in def_count.iter().enumerate() {
        if *c > 1 {
            cx.err(None, format!("register r{r} is defined {c} times"));
        }
    }
    for (si, s) in f.stmts.iter().enumerate() {
        if let Some(pt) = s.resume_next {
            match f.blocks.get(pt.block.0 as usize) {
                Some(b) if pt.index as usize <= b.insts.len() => {}
                _ => cx.err(None, format!("stmt#{si} resume point is outside its block")),
            }
        }
    }
    if !cx.errs.is_empty() {
        return;
    }
    definite_assignment(&mut cx);
}

fn check_inst(cx: &mut Ctx<'_>, bid: BlockId, k: &InstKind) {
    let p = cx.p;
    let nslots = cx.f.slots.len();
    let slot_ok = |s: vba_sema::LocalId| s.index() < nslots;
    let global_ok = |g: vba_sema::GlobalId| g.index() < p.program.globals.len();
    let mut bad = Vec::new();
    match k {
        InstKind::LoadLocal { slot, .. } | InstKind::StoreLocal { slot, .. } => {
            if !slot_ok(*slot) {
                bad.push(format!("slot s{} out of range", slot.0));
            }
        }
        InstKind::LoadGlobal { global, .. }
        | InstKind::StoreGlobal { global, .. }
        | InstKind::LoadField { global, .. }
        | InstKind::StoreField { global, .. } => {
            if !global_ok(*global) {
                bad.push(format!("global g{} out of range", global.0));
            }
        }
        InstKind::LoadDefaultInstance { module, .. } => {
            if module.index() >= p.program.modules.len() {
                bad.push("module out of range".into());
            }
        }
        InstKind::MakeRef { place, .. } => match place {
            RefPlace::Local(s) if !slot_ok(*s) => bad.push(format!("slot s{} out of range", s.0)),
            RefPlace::Global(g) | RefPlace::Field { global: g, .. } if !global_ok(*g) => {
                bad.push(format!("global g{} out of range", g.0))
            }
            RefPlace::UdtField { udt, index, .. }
                if (*index as usize) >= p.program.udt_fields(*udt).len() =>
            {
                bad.push("udt field out of range".into());
            }
            _ => {}
        },
        InstKind::UdtField { udt, index, .. } => {
            if (*index as usize) >= p.program.udt_fields(*udt).len() {
                bad.push("udt field out of range".into());
            }
        }
        InstKind::SetErrorMode {
            mode: ErrMode::Handler(h),
        } => {
            if h.0 as usize >= cx.f.blocks.len() {
                bad.push(format!("error handler block b{} missing", h.0));
            }
        }
        InstKind::RaiseEvent { event, args } => match p.program.events.get(event.index()) {
            Some(e) if args.len() == e.params.len() => {}
            Some(_) => bad.push("event argument count differs from the declaration".into()),
            None => bad.push("event out of range".into()),
        },
        InstKind::Call {
            dst,
            callee,
            receiver,
            args,
        } => match callee {
            Callee::Proc(pid) => match p.program.procs.get(pid.index()) {
                None => bad.push("callee out of range".into()),
                Some(pi) => {
                    let fixed = pi.params.iter().filter(|q| !q.param_array).count();
                    let has_pa = pi.params.iter().any(|q| q.param_array);
                    let ok = if has_pa {
                        args.len() >= fixed
                    } else {
                        args.len() == fixed
                    };
                    // Property Let/Set calls carry the value as an extra
                    // final argument when used as an assignment.
                    let is_put = matches!(
                        pi.kind,
                        vba_ast::ProcKind::PropertyLet | vba_ast::ProcKind::PropertySet
                    );
                    if !ok && !(is_put && args.len() == fixed) {
                        bad.push(format!(
                            "call to {} with {} argument(s), expected {}",
                            pi.name,
                            args.len(),
                            fixed
                        ));
                    }
                    if dst.is_some() && !pi.has_result {
                        bad.push(format!("{} returns no value but a result is used", pi.name));
                    }
                    if p.program.module(pi.module).is_class_like() && receiver.is_none() {
                        bad.push(format!(
                            "call to class member {} without a receiver",
                            pi.name
                        ));
                    }
                }
            },
            Callee::Declare(d) => match p.program.declares.get(d.index()) {
                None => bad.push("declare out of range".into()),
                Some(di) => {
                    if args.len() != di.params.len() {
                        bad.push(format!(
                            "call to declare {} with wrong argument count",
                            di.name
                        ));
                    }
                    if dst.is_some() && !di.is_function {
                        bad.push(format!("declare {} returns no value", di.name));
                    }
                }
            },
            Callee::Intrinsic { .. } => {}
        },
        _ => {}
    }
    for m in bad {
        cx.err(Some(bid), m);
    }
}

fn definite_assignment(cx: &mut Ctx<'_>) {
    let f = cx.f;
    let n = f.blocks.len();
    let nregs = f.regs.len();
    // Registers are defined exactly once (checked by the caller). Most are
    // used only in the block that defines them, and for those the check is a
    // walk through that block. Only registers used in another block take part
    // in the data-flow, so the sets are as wide as the number of such
    // registers and not as wide as all registers; otherwise a function with
    // many blocks would need blocks x registers bits.
    let mut def_block = vec![u32::MAX; nregs];
    for (bi, b) in f.blocks.iter().enumerate() {
        for i in &b.insts {
            if let Some(r) = inst_def(&i.kind) {
                def_block[r.0 as usize] = bi as u32;
            }
        }
        if let Some(r) = term_def(&b.term) {
            def_block[r.0 as usize] = bi as u32;
        }
    }
    let mut dense = vec![u32::MAX; nregs];
    let mut shared = 0usize;
    {
        let mut mark = |r: Reg, bi: usize| {
            if def_block[r.0 as usize] != bi as u32 && dense[r.0 as usize] == u32::MAX {
                dense[r.0 as usize] = shared as u32;
                shared += 1;
            }
        };
        for (bi, b) in f.blocks.iter().enumerate() {
            for i in &b.insts {
                for u in inst_uses(&i.kind) {
                    mark(u, bi);
                }
            }
            for u in term_uses(&b.term) {
                mark(u, bi);
            }
        }
    }
    let words = shared.div_ceil(64).max(1);
    // Roots with an empty entry set: the entry and every handler entry.
    let mut root = vec![false; n];
    root[0] = true;
    for b in &f.blocks {
        for i in &b.insts {
            if let InstKind::SetErrorMode {
                mode: ErrMode::Handler(h),
            } = i.kind
            {
                root[h.0 as usize] = true;
            }
        }
    }
    let mut inset: Vec<Option<Vec<u64>>> = vec![None; n];
    for (i, r) in root.iter().enumerate() {
        if *r {
            inset[i] = Some(vec![0; words]);
        }
    }
    let defs_of = |b: &Block| -> Vec<u64> {
        let mut d = vec![0u64; words];
        let mut add = |r: Reg| {
            let x = dense[r.0 as usize];
            if x != u32::MAX {
                d[x as usize / 64] |= 1 << (x % 64);
            }
        };
        for i in &b.insts {
            if let Some(r) = inst_def(&i.kind) {
                add(r);
            }
        }
        if let Some(r) = term_def(&b.term) {
            add(r);
        }
        d
    };
    let block_defs: Vec<Vec<u64>> = f.blocks.iter().map(defs_of).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for bi in 0..n {
            let Some(inb) = inset[bi].clone() else {
                continue;
            };
            let out: Vec<u64> = inb
                .iter()
                .zip(&block_defs[bi])
                .map(|(a, b)| a | b)
                .collect();
            for s in normal_successors(&f.blocks[bi].term) {
                let si = s.0 as usize;
                if root[si] {
                    continue;
                }
                let new = match &inset[si] {
                    None => out.clone(),
                    Some(cur) => cur.iter().zip(&out).map(|(a, b)| a & b).collect(),
                };
                if inset[si].as_ref() != Some(&new) {
                    inset[si] = Some(new);
                    changed = true;
                }
            }
        }
    }
    // `stamp[r] == bi + 1` once `r` is defined in block `bi` (for registers
    // that never leave their block).
    let mut stamp = vec![0u32; nregs];
    for (bi, b) in f.blocks.iter().enumerate() {
        let Some(mut have) = inset[bi].clone() else {
            continue;
        };
        let bid = BlockId(bi as u32);
        let tag = bi as u32 + 1;
        let check = |cx: &mut Ctx<'_>, have: &[u64], stamp: &[u32], r: Reg| {
            let x = dense[r.0 as usize];
            let ok = if x == u32::MAX {
                stamp[r.0 as usize] == tag
            } else {
                have[x as usize / 64] & (1 << (x % 64)) != 0
            };
            if !ok {
                cx.err(
                    Some(bid),
                    format!("register r{} used before it is defined", r.0),
                );
            }
        };
        for i in &b.insts {
            for u in inst_uses(&i.kind) {
                check(cx, &have, &stamp, u);
            }
            if let Some(d) = inst_def(&i.kind) {
                stamp[d.0 as usize] = tag;
                let x = dense[d.0 as usize];
                if x != u32::MAX {
                    have[x as usize / 64] |= 1 << (x % 64);
                }
            }
        }
        for u in term_uses(&b.term) {
            check(cx, &have, &stamp, u);
        }
    }
}
