//! Hostile caches through the lowering: random instruction streams, mutated
//! valid programs, and procedures that multiply work (a long argument chain
//! named by many headers).
//!
//! Synthetic and deterministic (seeded xorshift, fixed counts). Whatever the
//! input, lowering must return, and a lowered program must pass the IR
//! verifier. Mutation of local real files is in `corpus.rs`-style opt-in
//! tests and is not part of this file.
#![allow(missing_docs)]

mod common;

use std::time::{Duration, Instant};

use common::synth::*;
use vba_pcode::Opcode;
use vba_pcode_ir::{ModuleInput, lower_project};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const MENU: &[Opcode] = &[
    Opcode::ArgMissing,
    Opcode::BlockElseBranch,
    Opcode::BlockElseifBranch,
    Opcode::BlockIfClose,
    Opcode::BoolConjunction,
    Opcode::BoolDisjunction,
    Opcode::BoolNegate,
    Opcode::BranchGoto,
    Opcode::CaseDefault,
    Opcode::CaseIsEqual,
    Opcode::CaseListEnd,
    Opcode::CaseRange,
    Opcode::CaseValue,
    Opcode::CompareEqual,
    Opcode::CompareLess,
    Opcode::ConstInt16Decimal,
    Opcode::ConstInt32Decimal,
    Opcode::ConstFloat64,
    Opcode::ConstText,
    Opcode::ConvertToType,
    Opcode::DoClose,
    Opcode::DoCloseUntil,
    Opcode::DoOpen,
    Opcode::DoOpenWhile,
    Opcode::ErrHandlerSet,
    Opcode::ExprParentheses,
    Opcode::ForAdvance,
    Opcode::ForCounted,
    Opcode::ForCountedStep,
    Opcode::ForVarExprClose,
    Opcode::ForVarExprOpen,
    Opcode::IfBlockOpen,
    Opcode::IfSingleLine,
    Opcode::LabelNamed,
    Opcode::LeaveFor,
    Opcode::LeaveFunction,
    Opcode::NumAdd,
    Opcode::NumDivide,
    Opcode::NumNegate,
    Opcode::NumSubtract,
    Opcode::ReadSimple,
    Opcode::ReadSimpleParams,
    Opcode::StmtBegin,
    Opcode::SwitchBegin,
    Opcode::SwitchEnd,
    Opcode::TextJoin,
    Opcode::WhileClose,
    Opcode::WhileOpen,
    Opcode::WriteSimple,
    Opcode::InvokeSimple,
    Opcode::OutItemLast,
    Opcode::DimMarker,
    Opcode::HaltEnd,
];

fn lower_ok(project: &[u8], cache: &[u8]) {
    let started = Instant::now();
    let r = lower_project(
        project,
        1,
        &[ModuleInput {
            name: "M",
            kind: vba_project::ModuleKind::Standard,
            cache,
        }],
    );
    if let Ok(lp) = r {
        vba_ir::verify(&lp.ir).expect("a lowered program verifies");
    }
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "lowering took {:?}",
        started.elapsed()
    );
}

/// One procedure whose body is random instructions with random trailing
/// bytes, lowered many times.
#[test]
fn random_instruction_streams_lower_or_fail_cleanly() {
    let mut rng = Rng(0x5EED_0060);
    for _ in 0..600 {
        let mut t = Tables::default();
        let a = t.name("a");
        let f = t.function("F", Some(T_LONG), &[("a", T_LONG, true)]);
        let mut lines = vec![Line::new().rec(Opcode::ProcHeader, 6, f)];
        for _ in 0..1 + rng.below(10) {
            let mut l = Line::new();
            for _ in 0..1 + rng.below(6) {
                let op = MENU[rng.below(MENU.len())];
                l = match rng.below(4) {
                    0 => l.opm(op, rng.below(32) as u16),
                    1 => l.name(op, a),
                    2 => l.name_args(op, 0, a, rng.below(4) as u16),
                    _ => l.op(op),
                };
                for _ in 0..rng.below(5) {
                    l.0.push(rng.next() as u8);
                }
            }
            if l.0.len() % 2 == 1 {
                l.0.pop();
            }
            lines.push(l);
        }
        lines.push(Line::new().op(Opcode::ProcEndFunction));
        let project = build_project_stream(&t.idents);
        lower_ok(&project, &build_cache(&lines, &t.ind));
    }
}

fn valid_program() -> (Vec<u8>, Vec<u8>) {
    let mut t = Tables::default();
    let (a, b) = (t.name("a"), t.name("b"));
    let n = t.name("Calc");
    let f = t.function(
        "Calc",
        Some(T_LONG),
        &[("a", T_LONG, true), ("b", T_LONG, true)],
    );
    let lines = vec![
        Line::new().opm(Opcode::OptionStmt, 4),
        Line::new().rec(Opcode::ProcHeader, 6, f),
        Line::new()
            .read(a)
            .read(b)
            .op(Opcode::NumMultiply)
            .read(a)
            .op(Opcode::NumAdd)
            .write(n),
        Line::new().op(Opcode::ProcEndFunction),
    ];
    (build_project_stream(&t.idents), build_cache(&lines, &t.ind))
}

#[test]
fn mutated_valid_programs_lower_or_fail_cleanly() {
    let (project, cache) = valid_program();
    lower_ok(&project, &cache);
    let mut rng = Rng(0x5EED_0061);
    for _ in 0..4000 {
        let mut c = cache.clone();
        let mut p = project.clone();
        for _ in 0..1 + rng.below(4) {
            match rng.below(5) {
                0 => {
                    let i = rng.below(c.len());
                    c[i] = rng.next() as u8;
                }
                1 if c.len() >= 4 => {
                    let i = rng.below(c.len() - 3);
                    c[i..i + 4].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
                }
                2 => {
                    let n = rng.below(c.len() + 1);
                    c.truncate(n);
                }
                3 => {
                    let i = rng.below(p.len());
                    p[i] = rng.next() as u8;
                }
                _ => {
                    let i = rng.below(c.len());
                    c[i] ^= 1 << rng.below(8);
                }
            }
            if c.is_empty() || p.is_empty() {
                break;
            }
        }
        lower_ok(&p, &c);
    }
}

/// Many procedure headers naming one record with a long argument chain: the
/// shell built from the cache must stay bounded.
#[test]
fn shared_long_argument_chains_stay_bounded() {
    let mut t = Tables::default();
    let args: Vec<(String, u8, bool)> = (0..1500)
        .map(|i| (format!("p{}", i % 50), T_LONG, true))
        .collect();
    let refs: Vec<(&str, u8, bool)> = args.iter().map(|(n, t, b)| (n.as_str(), *t, *b)).collect();
    let f = t.function("F", Some(T_LONG), &refs);
    let mut lines = vec![Line::new().opm(Opcode::OptionStmt, 4)];
    for _ in 0..400 {
        lines.push(Line::new().rec(Opcode::ProcHeader, 6, f));
        lines.push(Line::new().op(Opcode::ProcEndFunction));
    }
    let project = build_project_stream(&t.idents);
    lower_ok(&project, &build_cache(&lines, &t.ind));
}

/// Mutated module caches of the local real files named by
/// `VBA_PCODE_CORPUS` and `VBA_PCODE_PROJECT`; returns at once when both are
/// unset. Only counts are printed.
#[test]
fn mutated_real_caches_lower_or_fail_cleanly() {
    let mut projects: Vec<common::Project> = common::corpus_files()
        .iter()
        .filter_map(|p| common::load_path(p))
        .collect();
    projects.extend(common::load());
    if projects.is_empty() {
        return;
    }
    let mut rng = Rng(0x5EED_0062);
    let (mut modules, mut runs) = (0usize, 0usize);
    for p in &projects {
        for m in p.modules.iter().take(6) {
            modules += 1;
            for round in 0..25 {
                let mut c = m.cache.clone();
                match round % 4 {
                    0 => {
                        for _ in 0..1 + rng.below(8) {
                            let i = rng.below(c.len());
                            c[i] = rng.next() as u8;
                        }
                    }
                    1 => {
                        let n = rng.below(c.len() + 1);
                        c.truncate(n);
                    }
                    2 if c.len() >= 4 => {
                        let i = rng.below(c.len() - 3);
                        c[i..i + 4].copy_from_slice(&[0xFF; 4]);
                    }
                    _ => {
                        let i = rng.below(c.len());
                        c[i] ^= 1 << rng.below(8);
                    }
                }
                let started = Instant::now();
                if let Ok(lp) = lower_project(
                    &p.project_stream,
                    p.sys_kind,
                    &[ModuleInput {
                        name: &m.name,
                        kind: common::role(m),
                        cache: &c,
                    }],
                ) {
                    vba_ir::verify(&lp.ir).expect("a lowered program verifies");
                }
                assert!(started.elapsed() < Duration::from_secs(60));
                runs += 1;
            }
        }
    }
    println!(
        "{} projects, {modules} modules, {runs} mutations",
        projects.len()
    );
}
