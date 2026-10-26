//! Lowering on the local project (ignored; set `VBA_PCODE_PROJECT`).
#![allow(missing_docs)]

mod common;

use std::collections::BTreeMap;

use vba_pcode_ir::{LowerOptions, ModuleInput, Outcome, lower_project_with_options};

#[test]
#[ignore = "needs a local project (VBA_PCODE_PROJECT)"]
fn lowering_summary() {
    let Some(p) = common::load() else { return };
    let inputs: Vec<ModuleInput<'_>> = p
        .modules
        .iter()
        .map(|m| ModuleInput {
            name: &m.name,
            kind: common::role(m),
            cache: &m.cache,
        })
        .collect();
    let options = LowerOptions {
        host_members: common::host_member_names(&p),
    };
    let lp = lower_project_with_options(&p.project_stream, p.sys_kind, &inputs, &options)
        .expect("lowers");
    let total = lp.procedures.len();
    let lowered = lp.lowered_count();
    println!("procedures {total}, lowered {lowered}");
    let is_std = |name: &str| {
        p.modules
            .iter()
            .any(|m| m.name == name && m.kind == vba_project::ModuleKind::Standard)
    };
    let is_decl = |r: &vba_pcode_ir::ProcedureReport| matches!(&r.outcome, Outcome::Unsupported(u) if u.reason == "Declare");
    for (label, want_std) in [("standard", true), ("class-like", false)] {
        let all: Vec<_> = lp
            .procedures
            .iter()
            .filter(|r| is_std(&r.module) == want_std)
            .collect();
        let declares = all.iter().filter(|r| is_decl(r)).count();
        let ok = all.iter().filter(|r| r.outcome.is_lowered()).count();
        println!(
            "{label}-module procedures {} (of which Declare {declares}), lowered {ok}",
            all.len()
        );
        let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
        for r in &all {
            if let Outcome::Unsupported(u) = &r.outcome {
                *reasons
                    .entry(format!("{} | {}", u.opcode.label(), u.reason))
                    .or_default() += 1;
            }
        }
        let mut v: Vec<_> = reasons.into_iter().collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        for (k, n) in v.iter().take(40) {
            println!("{n:5} {k}");
        }
    }
    if std::env::var_os("SHOW_UNSUPPORTED").is_some() {
        for r in &lp.procedures {
            if let Outcome::Unsupported(u) = &r.outcome
                && !is_decl(r)
            {
                println!("UNSUP {}.{}: {}", r.module, r.procedure, u);
            }
        }
    }
    let unsupported_decls = lp
        .declarations
        .iter()
        .filter(|d| d.unsupported.is_some())
        .count();
    println!(
        "module-level declarations {}, left out {unsupported_decls}",
        lp.declarations.len()
    );
    for r in lp.procedures.iter().filter(|r| r.outcome.is_lowered()) {
        println!("LOWERED {}.{}", r.module, r.procedure);
    }
}
