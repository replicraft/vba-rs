//! Lowering over a directory of local project files (never committed), one
//! line per file. Set `VBA_PCODE_CORPUS` to the directory. Run with
//! `cargo test -p vba-pcode-ir --test corpus -- --ignored --nocapture`.
#![allow(missing_docs)]

mod common;

use std::collections::BTreeMap;

use vba_pcode_ir::{LowerOptions, ModuleInput, Outcome, lower_project_with_options};

#[test]
#[ignore = "needs a local corpus (VBA_PCODE_CORPUS)"]
fn lowering_over_the_corpus() {
    let mut total_reasons: BTreeMap<String, usize> = BTreeMap::new();
    let (mut procs, mut lowered, mut files, mut refused) = (0, 0, 0, 0);
    for (n, path) in common::corpus_files().iter().enumerate() {
        let Some(p) = common::load_path(path) else {
            continue;
        };
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
        let v = vba_pcode::version_word(&p.project_stream).unwrap_or(0);
        match lower_project_with_options(&p.project_stream, p.sys_kind, &inputs, &options) {
            Ok(lp) => {
                files += 1;
                let t = lp.procedures.len();
                let l = lp.lowered_count();
                procs += t;
                lowered += l;
                let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
                for r in &lp.procedures {
                    if let Outcome::Unsupported(u) = &r.outcome {
                        *reasons
                            .entry(format!("{} | {}", u.opcode.label(), u.reason))
                            .or_default() += 1;
                        *total_reasons
                            .entry(format!("{} | {}", u.opcode.label(), u.reason))
                            .or_default() += 1;
                    }
                }
                let mut top: Vec<_> = reasons.into_iter().collect();
                top.sort_by_key(|x| std::cmp::Reverse(x.1));
                println!(
                    "#{n} v={v:#x} sys={} procedures {t} lowered {l} unsupported-top {:?}",
                    p.sys_kind,
                    top.iter().take(3).collect::<Vec<_>>()
                );
            }
            Err(e) => {
                refused += 1;
                println!("#{n} v={v:#x} sys={} refused: {e}", p.sys_kind);
            }
        }
    }
    println!("files lowered {files}, refused {refused}, procedures {procs}, lowered {lowered}");
    let mut top: Vec<_> = total_reasons.into_iter().collect();
    top.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (k, n) in top.iter().take(40) {
        println!("{n:5} {k}");
    }
}
