//! Hostile IR: valid programs are lowered, then mutated (registers, slots,
//! ids, types, operands, terminators, whole instructions). For every mutant
//! `vba_ir::verify` must not panic, and when it accepts the mutant the VM
//! must not panic either: it may run, fail with any error, or hit a limit.

mod common;

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use common::module;
use proptest::prelude::*;
use vba_host::{HeadlessHost, ResourceLimits};
use vba_ir::{
    Block, BlockId, CallArg, ErrMode, InstKind, IrProgram, Operand, RefPlace, Reg, ResumeKind,
    StmtId, Terminator,
};
use vba_project::ModuleKind;
use vba_sema::bound::Constant;
use vba_sema::{EventId, GlobalId, LocalId, ModuleId, ProcId, Type, UdtId};
use vba_value::Variant;
use vba_vm::{BufferSink, NoServices, Vm, VmConfig};

const POOL: &[&[(&str, ModuleKind, &str)]] = &[
    &[(
        "M",
        ModuleKind::Standard,
        "Public Sub Main()\nDim a(1 To 5) As Long, i As Long, s As String, d As Double\nFor i = 1 To 5\na(i) = i * i\ns = s & CStr(a(i))\nd = d + a(i) / 3\nNext\nDebug.Print s; d; UBound(a)\nDim v As Variant\nv = Array(1, \"x\", 2.5)\nFor Each v In v\nDebug.Print v\nNext\nEnd Sub\n",
    )],
    &[(
        "M",
        ModuleKind::Standard,
        "Private Type Pt\nX As Long\nS As String\nEnd Type\nSub Bump(ByRef n As Long, p As Pt)\nn = n + 1\np.X = p.X + n\np.S = p.S & \"a\"\nEnd Sub\nPublic Sub Main()\nDim p As Pt, n As Long, k As Long\nFor k = 1 To 4\nBump n, p\nNext\nSelect Case p.X\nCase 0\nDebug.Print \"zero\"\nCase Is > 5\nDebug.Print \"big\"; p.S\nCase Else\nDebug.Print \"other\"\nEnd Select\nGoSub done\nExit Sub\ndone:\nDebug.Print n\nReturn\nEnd Sub\n",
    )],
    &[(
        "M",
        ModuleKind::Standard,
        "Function Div(ByVal a As Long, ByVal b As Long) As Long\nDiv = a \\ b\nEnd Function\nSub Thrower(ByVal n As Long)\nIf n Mod 2 = 0 Then Err.Raise 1000 + n, \"T\", \"boom\"\nEnd Sub\nPublic Sub Main()\nOn Error GoTo h\nDim i As Long\nFor i = 0 To 3\nDebug.Print Div(10, i)\nThrower i\nNext\nExit Sub\nh:\nDebug.Print \"err \" & Err.Number\nResume Next\nEnd Sub\n",
    )],
    &[
        (
            "M",
            ModuleKind::Standard,
            "Public Sub Main()\nDim o As Acc, c As New Collection, v As Variant\nSet o = New Acc\no.Add 3\no.Add 4\nc.Add o\nc.Add \"k\", \"key\"\nFor Each v In c\nDebug.Print TypeName(v)\nNext\nDebug.Print o.Total; o.Count; c.Count\nSet o = Nothing\nDebug.Print \"done\"\nEnd Sub\n",
        ),
        (
            "Acc",
            ModuleKind::Class,
            "Public Total As Long\nPrivate mN As Long\nPublic Sub Add(ByVal n As Long)\nTotal = Total + n\nmN = mN + 1\nEnd Sub\nPublic Property Get Count() As Long\nCount = mN\nEnd Property\nPrivate Sub Class_Terminate()\nDebug.Print \"bye \" & Total\nEnd Sub\n",
        ),
    ],
    &[(
        "M",
        ModuleKind::Standard,
        "Public Sub Main()\nDim v As Variant, w As Variant, s As String\nv = 5\nw = \"7\"\nDebug.Print v + w; v & w; v * 2.5; CStr(v) & s\nv = Null\nDebug.Print IsNull(v + 1)\nDim x() As Long\nReDim x(1 To 3)\nx(2) = 9\nReDim Preserve x(1 To 6)\nDebug.Print x(2); UBound(x)\nOn Error Resume Next\nDebug.Print x(10)\nDebug.Print Err.Number\nErase x\nEnd Sub\n",
    )],
];

thread_local! {
    static POOL_IR: RefCell<Vec<IrProgram>> = const { RefCell::new(Vec::new()) };
    static LAST_PANIC: RefCell<String> = const { RefCell::new(String::new()) };
}

fn compile_pool() -> Vec<IrProgram> {
    POOL.iter()
        .map(|mods| {
            let mods: Vec<_> = mods.iter().map(|(n, k, s)| module(n, *k, s)).collect();
            let a = vba_sema::analyze_sources(
                &mods,
                vba_profile::default_profile(),
                &vba_sema::NoLibraries,
                &vba_sema::Options::default(),
            );
            assert!(!a.has_errors(), "pool program does not compile");
            let ir = vba_ir::lower(&a).expect("lower");
            vba_ir::verify(&ir).expect("pool program verifies");
            ir
        })
        .collect()
}

// ---- mutation -----------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
    /// An index that is usually in range and often just outside it.
    fn bad(&mut self, n: usize) -> u32 {
        match self.below(6) {
            0 => n as u32,
            1 => n as u32 + 1,
            2 => u32::MAX,
            3 => 0,
            _ => self.below(n + 2) as u32,
        }
    }
}

fn random_type(rng: &mut Rng) -> Type {
    match rng.below(14) {
        0 => Type::Boolean,
        1 => Type::Long,
        2 => Type::Double,
        3 => Type::String,
        4 => Type::Variant,
        5 => Type::Object,
        6 => Type::Class(ModuleId(rng.bad(3))),
        7 => Type::Udt(UdtId(rng.bad(2))),
        8 => Type::Array {
            elem: Box::new(random_type(rng)),
            rank: Some(rng.below(3) as u8),
        },
        9 => Type::FixedString(rng.next() as u32 % 100_000),
        10 => Type::Byte,
        11 => Type::Currency,
        12 => Type::Nothing,
        _ => Type::Date,
    }
}

fn random_const(rng: &mut Rng) -> Operand {
    Operand::Const(Constant::Value(match rng.below(8) {
        0 => Variant::Empty,
        1 => Variant::Null,
        2 => Variant::Long(rng.next() as i32),
        3 => Variant::Double(f64::from_bits(rng.next())),
        4 => Variant::from_text("abc"),
        5 => Variant::Boolean(true),
        6 => Variant::Long(0),
        _ => Variant::Integer(rng.next() as i16),
    }))
}

fn operands(k: &mut InstKind) -> Vec<&mut Operand> {
    let mut v: Vec<&mut Operand> = Vec::new();
    match k {
        InstKind::Copy { src, .. }
        | InstKind::Unary { src, .. }
        | InstKind::Convert { src, .. }
        | InstKind::StoreLocal { src, .. }
        | InstKind::StoreGlobal { src, .. }
        | InstKind::StoreRef { src, .. }
        | InstKind::TypeOfIs { src, .. }
        | InstKind::IterInit { src, .. } => v.push(src),
        InstKind::Binary { lhs, rhs, .. } => {
            v.push(lhs);
            v.push(rhs);
        }
        InstKind::LoadField { obj, .. } => v.push(obj),
        InstKind::StoreField { obj, src, .. } => {
            v.push(obj);
            v.push(src);
        }
        InstKind::Index { array, indices, .. } => {
            v.push(array);
            v.extend(indices.iter_mut());
        }
        InstKind::UdtField { base, .. } => v.push(base),
        InstKind::Call { receiver, args, .. } => {
            v.extend(receiver.iter_mut());
            for a in args {
                if let CallArg::Val(o) = a {
                    v.push(o);
                }
            }
        }
        InstKind::Invoke { receiver, args, .. } => {
            v.push(receiver);
            for a in args {
                if let CallArg::Val(o) = &mut a.arg {
                    v.push(o);
                }
            }
        }
        InstKind::MakeRef { place, .. } => match place {
            RefPlace::Field { obj, .. } => v.push(obj),
            RefPlace::Elem { indices, .. } => v.extend(indices.iter_mut()),
            _ => {}
        },
        InstKind::ReDim { bounds, .. } => {
            for (l, h) in bounds {
                v.extend(l.iter_mut());
                v.push(h);
            }
        }
        InstKind::ForTest {
            counter, end, step, ..
        } => {
            v.push(counter);
            v.push(end);
            v.push(step);
        }
        _ => {}
    }
    v
}

fn regs(k: &mut InstKind) -> Vec<&mut Reg> {
    let mut v: Vec<&mut Reg> = Vec::new();
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
        | InstKind::Index { dst, .. }
        | InstKind::UdtField { dst, .. }
        | InstKind::New { dst, .. }
        | InstKind::AddressOf { dst, .. }
        | InstKind::TypeOfIs { dst, .. }
        | InstKind::IterInit { dst, .. }
        | InstKind::Undefined { dst }
        | InstKind::ForTest { dst, .. } => v.push(dst),
        InstKind::MakeRef { dst, place } => {
            v.push(dst);
            match place {
                RefPlace::Elem { base, .. } | RefPlace::UdtField { base, .. } => v.push(base),
                _ => {}
            }
        }
        InstKind::LoadRef { dst, r } => {
            v.push(dst);
            v.push(r);
        }
        InstKind::StoreRef { r, .. }
        | InstKind::EnsureNew { r, .. }
        | InstKind::ReDim { r, .. }
        | InstKind::DimFixed { r, .. }
        | InstKind::Erase { r } => v.push(r),
        InstKind::Call { dst, args, .. } => {
            v.extend(dst.iter_mut());
            for a in args {
                if let CallArg::Ref(r) = a {
                    v.push(r);
                }
            }
        }
        InstKind::Invoke { dst, args, .. } => {
            v.extend(dst.iter_mut());
            for a in args {
                if let CallArg::Ref(r) = &mut a.arg {
                    v.push(r);
                }
            }
        }
        _ => {}
    }
    v
}

fn mutate_ids(k: &mut InstKind, rng: &mut Rng) {
    match k {
        InstKind::LoadLocal { slot, .. } | InstKind::StoreLocal { slot, .. } => {
            *slot = LocalId(rng.bad(8));
        }
        InstKind::LoadGlobal { global, .. }
        | InstKind::StoreGlobal { global, .. }
        | InstKind::LoadField { global, .. }
        | InstKind::StoreField { global, .. } => *global = GlobalId(rng.bad(8)),
        InstKind::LoadDefaultInstance { module, .. } => *module = ModuleId(rng.bad(3)),
        InstKind::UdtField { udt, index, .. } => {
            if rng.below(2) == 0 {
                *udt = UdtId(rng.bad(2));
            } else {
                *index = rng.bad(4);
            }
        }
        InstKind::Call {
            callee: vba_ir::Callee::Proc(p),
            ..
        } => *p = ProcId(rng.bad(6)),
        InstKind::RaiseEvent { event, .. } => *event = EventId(rng.bad(2)),
        InstKind::MakeRef { place, .. } => match place {
            RefPlace::Local(l) => *l = LocalId(rng.bad(8)),
            RefPlace::Global(g) => *g = GlobalId(rng.bad(8)),
            RefPlace::Field { global, .. } => *global = GlobalId(rng.bad(8)),
            RefPlace::UdtField { udt, index, .. } => {
                if rng.below(2) == 0 {
                    *udt = UdtId(rng.bad(2));
                } else {
                    *index = rng.bad(4);
                }
            }
            RefPlace::Elem { .. } => {}
        },
        InstKind::SetErrorMode { mode } => {
            *mode = match rng.below(3) {
                0 => ErrMode::Propagate,
                1 => ErrMode::ResumeNext,
                _ => ErrMode::Handler(BlockId(rng.bad(6))),
            };
        }
        InstKind::SetLine { line } => *line = rng.next(),
        _ => {}
    }
}

fn mutate_types(k: &mut InstKind, rng: &mut Rng) {
    let t = random_type(rng);
    match k {
        InstKind::Unary { ty, .. }
        | InstKind::Binary { ty, .. }
        | InstKind::TypeOfIs { ty, .. } => *ty = t,
        InstKind::Convert { to, .. } => *to = t,
        InstKind::ReDim { elem, .. } | InstKind::DimFixed { elem, .. } => *elem = t,
        _ => {}
    }
    if let InstKind::DimFixed { bounds, .. } = k
        && rng.below(2) == 0
    {
        bounds.push((rng.next() as i32, rng.next() as i32));
    }
}

fn random_terminator(rng: &mut Rng, nblocks: usize, nregs: usize) -> Terminator {
    let blk = |rng: &mut Rng| BlockId(rng.bad(nblocks));
    match rng.below(10) {
        0 => Terminator::Jump(blk(rng)),
        1 => Terminator::Branch {
            cond: Operand::Reg(Reg(rng.bad(nregs))),
            then_block: blk(rng),
            else_block: blk(rng),
        },
        2 => Terminator::Switch {
            selector: random_const(rng),
            targets: vec![blk(rng), blk(rng)],
            default: blk(rng),
        },
        3 => Terminator::GoSub {
            target: blk(rng),
            ret: blk(rng),
        },
        4 => Terminator::ReturnFromGoSub,
        5 => Terminator::Resume(match rng.below(3) {
            0 => ResumeKind::Same,
            1 => ResumeKind::Next,
            _ => ResumeKind::Block(blk(rng)),
        }),
        6 => Terminator::Return,
        7 => Terminator::End,
        8 => Terminator::Unreachable,
        _ => Terminator::Stop,
    }
}

fn mutate_terminator(t: &mut Terminator, rng: &mut Rng, nblocks: usize, nregs: usize) {
    match t {
        Terminator::Jump(b) => *b = BlockId(rng.bad(nblocks)),
        Terminator::Branch {
            cond,
            then_block,
            else_block,
        } => match rng.below(3) {
            0 => *then_block = BlockId(rng.bad(nblocks)),
            1 => *else_block = BlockId(rng.bad(nblocks)),
            _ => *cond = Operand::Reg(Reg(rng.bad(nregs))),
        },
        Terminator::Switch {
            targets, default, ..
        } => match rng.below(3) {
            0 => *default = BlockId(rng.bad(nblocks)),
            1 => targets.clear(),
            _ => {
                if let Some(b) = targets.first_mut() {
                    *b = BlockId(rng.bad(nblocks));
                }
            }
        },
        Terminator::ForEachNext {
            iter,
            item,
            body,
            done,
        } => match rng.below(4) {
            0 => *iter = Reg(rng.bad(nregs)),
            1 => *item = Reg(rng.bad(nregs)),
            2 => *body = BlockId(rng.bad(nblocks)),
            _ => *done = BlockId(rng.bad(nblocks)),
        },
        other => *other = random_terminator(rng, nblocks, nregs),
    }
}

fn mutate(ir: &mut IrProgram, rng: &mut Rng) {
    let nf = ir.functions.len();
    if nf == 0 {
        return;
    }
    let fi = rng.below(nf);
    let op = rng.below(18);
    let nblocks = ir.functions[fi].blocks.len();
    let nregs = ir.functions[fi].regs.len();
    if op == 17 {
        // Program-level tables.
        match rng.below(4) {
            0 => {
                if let Some(slot) = ir.module_init.iter_mut().find(|s| s.is_some()) {
                    *slot = Some(vba_ir::FuncId(rng.bad(nf)));
                }
            }
            1 => {
                if let Some(Some(ci)) = ir.classes.iter_mut().find(|c| c.is_some()) {
                    ci.initialize = Some(ProcId(rng.bad(nf)));
                }
            }
            2 => {
                if let Some(Some(ci)) = ir.classes.iter_mut().find(|c| c.is_some()) {
                    ci.fields.push(GlobalId(rng.bad(8)));
                }
            }
            _ => {
                if let Some(Some(ci)) = ir.classes.iter_mut().find(|c| c.is_some()) {
                    ci.terminate = Some(ProcId(rng.bad(nf)));
                }
            }
        }
        return;
    }
    let f = &mut ir.functions[fi];
    match op {
        8 => {
            let n = rng.below(nregs + 1);
            f.regs.truncate(n);
            return;
        }
        9 => {
            let n = rng.below(f.slots.len() + 1);
            f.slots.truncate(n);
            return;
        }
        10 => {
            let n = rng.below(f.blocks.len() + 1);
            f.blocks.truncate(n);
            return;
        }
        11 => {
            let n = rng.below(f.stmts.len() + 1);
            f.stmts.truncate(n);
            return;
        }
        12 => {
            let n = f.slots.len().max(1);
            if let Some(s) = f.slots.get_mut(rng.below(n)) {
                s.ty = random_type(rng);
            }
            return;
        }
        13 => {
            if !f.regs.is_empty() {
                let i = rng.below(f.regs.len());
                f.regs[i] = random_type(rng);
            }
            return;
        }
        _ => {}
    }
    if f.blocks.is_empty() {
        return;
    }
    let bi = rng.below(f.blocks.len());
    let ninst = f.blocks[bi].insts.len();
    // Operations on the terminator.
    if op == 6 || ninst == 0 {
        mutate_terminator(&mut f.blocks[bi].term, rng, nblocks, nregs);
        return;
    }
    if op == 7 {
        f.blocks[bi].term = random_terminator(rng, nblocks, nregs);
        return;
    }
    if op == 14 {
        f.blocks[bi].term_stmt = StmtId(rng.bad(f.stmts.len()));
        return;
    }
    let ii = rng.below(ninst);
    match op {
        0 => {
            let b = &mut f.blocks[bi];
            let mut rs = regs(&mut b.insts[ii].kind);
            if !rs.is_empty() {
                let n = rs.len();
                *rs[rng.below(n)] = Reg(rng.bad(nregs));
            }
        }
        1 => {
            let b = &mut f.blocks[bi];
            let mut os = operands(&mut b.insts[ii].kind);
            if !os.is_empty() {
                let n = os.len();
                let o = &mut os[rng.below(n)];
                **o = if rng.below(2) == 0 {
                    Operand::Reg(Reg(rng.bad(nregs)))
                } else {
                    random_const(rng)
                };
            }
        }
        2 => mutate_ids(&mut f.blocks[bi].insts[ii].kind, rng),
        3 => {
            f.blocks[bi].insts.remove(ii);
        }
        4 => {
            let i = f.blocks[bi].insts[ii].clone();
            f.blocks[bi].insts.insert(ii, i);
        }
        5 => {
            let j = rng.below(ninst);
            f.blocks[bi].insts.swap(ii, j);
        }
        15 => mutate_types(&mut f.blocks[bi].insts[ii].kind, rng),
        16 => {
            // Replace the instruction by one taken from anywhere in the program.
            let donor = {
                let fj = rng.below(nf);
                let blocks: &Vec<Block> = &ir.functions[fj].blocks;
                if blocks.is_empty() {
                    return;
                }
                let b = &blocks[rng.below(blocks.len())];
                if b.insts.is_empty() {
                    return;
                }
                b.insts[rng.below(b.insts.len())].clone()
            };
            ir.functions[fi].blocks[bi].insts[ii] = donor;
        }
        _ => {
            f.blocks[bi].insts[ii].stmt = StmtId(rng.bad(f.stmts.len()));
        }
    }
}

// ---- running mutants -------------------------------------------------------

fn install_hook() {
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let msg = info
            .payload()
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| {
                info.payload()
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_owned())
            })
            .unwrap_or_default();
        LAST_PANIC.with(|p| *p.borrow_mut() = format!("{msg} at {loc}"));
    }));
}

fn run_vm(ir: IrProgram) {
    let host = Rc::new(
        HeadlessHost::new().with_limits(
            ResourceLimits::default()
                .with_max_steps(Some(20_000))
                .with_max_memory_bytes(Some(8 << 20))
                .with_max_call_depth(40),
        ),
    );
    let sink = BufferSink::new();
    let Ok(mut vm) = Vm::new(
        Rc::new(ir),
        host,
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink),
        VmConfig {
            max_string_units: 1 << 16,
            max_array_elements: 1 << 16,
            ..VmConfig::default()
        },
    ) else {
        return;
    };
    if let Some(main) = vm.find_procedure("Main") {
        let _ = vm.run_procedure(main, vec![]);
    }
}

fn check_mutant(ir: IrProgram, tag: &str) -> Result<bool, String> {
    let verdict = catch_unwind(AssertUnwindSafe(|| vba_ir::verify(&ir)));
    match verdict {
        Err(_) => Err(format!(
            "verify panicked on a mutant ({tag}): {}",
            LAST_PANIC.with(|p| p.borrow().clone())
        )),
        Ok(Err(_)) => Ok(false),
        Ok(Ok(())) => {
            let r = catch_unwind(AssertUnwindSafe(|| run_vm(ir)));
            match r {
                Ok(()) => Ok(true),
                Err(_) => Err(format!(
                    "the VM panicked on verified IR ({tag}): {}",
                    LAST_PANIC.with(|p| p.borrow().clone())
                )),
            }
        }
    }
}

fn mutant(prog: usize, seed: u64, count: usize) -> IrProgram {
    let mut ir = POOL_IR.with(|p| {
        let mut p = p.borrow_mut();
        if p.is_empty() {
            *p = compile_pool();
        }
        p[prog % POOL.len()].clone()
    });
    let mut rng = Rng(seed | 1);
    for _ in 0..count {
        mutate(&mut ir, &mut rng);
    }
    ir
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1500, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn mutated_ir_is_rejected_or_runs_without_panicking(
        prog in 0usize..POOL.len(),
        seed in any::<u64>(),
        count in 1usize..4,
    ) {
        install_hook();
        let ir = mutant(prog, seed, count);
        let tag = format!("program {prog}, seed {seed}, {count} mutations");
        if let Err(e) = check_mutant(ir, &tag) {
            eprintln!("{e}");
            return Err(TestCaseError::fail(e));
        }
    }
}

/// The mutator must not be vacuous: a fair share of mutants is rejected by
/// the verifier (the mutations bite) and a fair share is accepted (the VM
/// is exercised on hostile input).
#[test]
fn mutants_are_both_rejected_and_run() {
    install_hook();
    let (mut rejected, mut ran) = (0, 0);
    for seed in 0..600u64 {
        let ir = mutant(
            seed as usize,
            seed.wrapping_mul(0x9E37_79B9_7F4A_7C15),
            1 + (seed % 3) as usize,
        );
        match check_mutant(ir, "coverage") {
            Ok(true) => ran += 1,
            Ok(false) => rejected += 1,
            Err(e) => panic!("{e}"),
        }
    }
    assert!(
        rejected > 60,
        "only {rejected} of 600 mutants were rejected"
    );
    assert!(ran > 60, "only {ran} of 600 mutants passed verification");
}

/// Lists the distinct panic sites over many mutants (a debugging aid:
/// `cargo test -p vba-vm --test hostile_ir survey -- --ignored --nocapture`).
#[test]
#[ignore]
fn survey() {
    install_hook();
    let mut sites: std::collections::BTreeMap<String, String> = Default::default();
    let n: u64 = std::env::var("SURVEY_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30_000);
    for seed in 0..n {
        let s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let ir = mutant(seed as usize, s, 1 + (seed % 8) as usize);
        if let Err(e) = check_mutant(
            ir,
            &format!("program {}, seed {s}", seed % POOL.len() as u64),
        ) {
            let site = e.split(": ").skip(1).collect::<Vec<_>>().join(": ");
            sites.entry(site).or_insert(e);
        }
    }
    for (site, e) in &sites {
        println!("{site}\n    first: {e}");
    }
}

// ---- regressions -------------------------------------------------------------
//
// Found by the fuzz above: the class table of an `IrProgram` was not
// validated, so a field, an initialiser or a terminator that names a global
// or procedure that does not exist passed `verify` and then panicked the VM
// on an out-of-range index.

fn class_pool_program() -> IrProgram {
    compile_pool().swap_remove(3)
}

fn first_class(ir: &mut IrProgram) -> &mut vba_ir::ClassInfo {
    ir.classes
        .iter_mut()
        .flatten()
        .next()
        .expect("a class module")
}

#[test]
fn verify_rejects_a_class_field_that_is_not_a_global() {
    let mut ir = class_pool_program();
    first_class(&mut ir).fields.push(GlobalId(10_000));
    assert!(vba_ir::verify(&ir).is_err());
}

#[test]
fn verify_rejects_class_procedures_that_do_not_exist() {
    let mut ir = class_pool_program();
    first_class(&mut ir).initialize = Some(ProcId(10_000));
    assert!(vba_ir::verify(&ir).is_err());
    let mut ir = class_pool_program();
    first_class(&mut ir).terminate = Some(ProcId(10_000));
    assert!(vba_ir::verify(&ir).is_err());
    let mut ir = class_pool_program();
    first_class(&mut ir).init = Some(vba_ir::FuncId(10_000));
    assert!(vba_ir::verify(&ir).is_err());
}

#[test]
fn verify_rejects_a_class_entry_for_the_wrong_module() {
    let mut ir = class_pool_program();
    first_class(&mut ir).module = ModuleId(10_000);
    assert!(vba_ir::verify(&ir).is_err());
}

#[test]
fn a_class_table_naming_a_missing_global_does_not_panic_the_vm() {
    let mut ir = class_pool_program();
    first_class(&mut ir).fields.push(GlobalId(10_000));
    // Whatever the verifier says, the VM must not panic.
    assert!(!check_mutant(ir, "regression").is_err());
}
