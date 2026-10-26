//! Lowering on the local project (ignored; set `VBA_PCODE_PROJECT`).
#![allow(missing_docs)]

mod common;

use std::collections::BTreeMap;

use vba_pcode_ir::{ModuleInput, Outcome, lower_project};

#[test]
#[ignore = "needs a local project (VBA_PCODE_PROJECT)"]
fn lowering_summary() {
    let Some(p) = common::load() else { return };
    let inputs: Vec<ModuleInput<'_>> = p
        .modules
        .iter()
        .map(|m| ModuleInput {
            name: &m.name,
            class_like: m.kind != vba_project::ModuleKind::Standard,
            cache: &m.cache,
        })
        .collect();
    let lp = lower_project(&p.project_stream, p.sys_kind, &inputs).expect("lowers");
    let total = lp.procedures.len();
    let lowered = lp.lowered_count();
    println!("procedures {total}, lowered {lowered}");
    let std_names: Vec<&str> = p
        .modules
        .iter()
        .filter(|m| m.kind == vba_project::ModuleKind::Standard)
        .map(|m| m.name.as_str())
        .collect();
    let in_std = lp
        .procedures
        .iter()
        .filter(|r| std_names.contains(&r.module.as_str()))
        .count();
    let declares = lp
        .procedures
        .iter()
        .filter(|r| {
            std_names.contains(&r.module.as_str())
                && matches!(&r.outcome, Outcome::Unsupported(u) if u.reason == "Declare")
        })
        .count();
    println!(
        "standard-module procedures {in_std} (of which Declare {declares}), lowered {lowered}"
    );
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    for r in &lp.procedures {
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
    if std::env::var_os("SHOW_STANDARD").is_some() {
        let std_mods: Vec<&str> = p
            .modules
            .iter()
            .filter(|m| m.kind == vba_project::ModuleKind::Standard)
            .map(|m| m.name.as_str())
            .collect();
        for r in &lp.procedures {
            if let Outcome::Unsupported(u) = &r.outcome
                && std_mods.contains(&r.module.as_str())
                && u.reason != "Declare"
            {
                println!("UNSUP {}.{}: {}", r.module, r.procedure, u);
            }
        }
    }
    for r in lp
        .procedures
        .iter()
        .filter(|r| r.outcome.is_lowered())
        .take(60)
    {
        println!("LOWERED {}.{}", r.module, r.procedure);
    }
}
