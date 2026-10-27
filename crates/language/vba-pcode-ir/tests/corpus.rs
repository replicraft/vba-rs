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
    let mut shapes: BTreeMap<String, usize> = BTreeMap::new();
    // (version word, platform kind) -> (files, procedures, lowered)
    let mut layouts: BTreeMap<(u16, u32), (usize, usize, usize)> = BTreeMap::new();
    let mut roots: BTreeMap<String, usize> = BTreeMap::new();
    let mut roots_raw: BTreeMap<String, usize> = BTreeMap::new();
    let mut var_roots: BTreeMap<String, usize> = BTreeMap::new();
    let mut decl_reasons: BTreeMap<String, usize> = BTreeMap::new();
    let (mut decls, mut decls_ok) = (0usize, 0usize);
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
            project_name: Some(p.name.clone()),
        };
        let v = vba_pcode::version_word(&p.project_stream).unwrap_or(0);
        match lower_project_with_options(&p.project_stream, p.sys_kind, &inputs, &options) {
            Ok(lp) => {
                files += 1;
                let t = lp.procedures.len();
                let l = lp.lowered_count();
                procs += t;
                lowered += l;
                let e = layouts.entry((v, p.sys_kind)).or_default();
                e.0 += 1;
                e.1 += t;
                e.2 += l;
                for d in &lp.declarations {
                    decls += 1;
                    match &d.unsupported {
                        None => decls_ok += 1,
                        Some(r) => {
                            if std::env::var_os("SHOW_RAW_DECL").is_some() {
                                println!("DECL {} {}: {r}", d.module, d.name);
                            }
                            *decl_reasons.entry(shape(r)).or_default() += 1
                        }
                    }
                }
                // Root cause: follow "calls X" and "instances run X" to the
                // procedure that is itself refused for another reason.
                let by_name: std::collections::HashMap<
                    (String, String),
                    &vba_pcode_ir::Unsupported,
                > = lp
                    .procedures
                    .iter()
                    .filter_map(|r| match &r.outcome {
                        Outcome::Unsupported(u) => {
                            Some(((r.module.to_lowercase(), r.procedure.to_lowercase()), u))
                        }
                        Outcome::Lowered => None,
                    })
                    .collect();
                for r in &lp.procedures {
                    let Outcome::Unsupported(u0) = &r.outcome else {
                        continue;
                    };
                    let mut u = u0;
                    let mut module = r.module.to_lowercase();
                    for _ in 0..30 {
                        let next = if let Some(rest) = u.reason.strip_prefix("calls `") {
                            rest.split_once('`').and_then(|(m, _)| {
                                m.split_once('.')
                                    .map(|(a, b)| (a.to_lowercase(), b.to_lowercase()))
                            })
                        } else if let Some(rest) = u.reason.strip_prefix("instances run `") {
                            rest.split_once('`')
                                .map(|(b, _)| (module.clone(), b.to_lowercase()))
                        } else {
                            None
                        };
                        match next.and_then(|k| by_name.get(&k).map(|v| (k, *v))) {
                            Some(((m, _), nu)) => {
                                module = m;
                                u = nu;
                            }
                            None => break,
                        }
                    }
                    *roots
                        .entry(format!("{} | {}", u.opcode.label(), shape(&u.reason)))
                        .or_default() += 1;
                    *roots_raw
                        .entry(format!("{} | {}", u.opcode.label(), u.reason))
                        .or_default() += 1;
                    // A module-level variable of a user-defined type that
                    // is not generated: why its type is not.
                    if u.reason.starts_with("name `")
                        && let Some(var) = u.reason.split('`').nth(1)
                        && let Some(d) = lp.declarations.iter().find(|d| {
                            d.module.to_lowercase() == module
                                && d.name == var
                                && d.unsupported.is_some()
                        })
                    {
                        let ty = d
                            .unsupported
                            .as_deref()
                            .and_then(|r| r.split('`').nth(1))
                            .and_then(|t| {
                                lp.declarations.iter().find(|x| {
                                    x.module.to_lowercase() == module
                                        && x.name == t
                                        && x.unsupported.is_some()
                                })
                            });
                        if let Some(t) = ty {
                            *var_roots
                                .entry(format!(
                                    "variable `{var}`: type {}",
                                    shape(t.unsupported.as_deref().unwrap_or(""))
                                ))
                                .or_default() += 1;
                        }
                    }
                }
                let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
                for r in &lp.procedures {
                    if let Outcome::Unsupported(u) = &r.outcome {
                        *reasons
                            .entry(format!("{} | {}", u.opcode.label(), u.reason))
                            .or_default() += 1;
                        *total_reasons
                            .entry(format!("{} | {}", u.opcode.label(), u.reason))
                            .or_default() += 1;
                        *shapes
                            .entry(format!("{} | {}", u.opcode.label(), shape(&u.reason)))
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
    println!("-- coverage per layout (version word, platform kind)");
    for ((v, sys), (f, t, l)) in &layouts {
        println!("v={v:#x} sys={sys}: files {f}, procedures {t}, lowered {l}");
    }
    println!("-- refusal shapes (names collapsed), one per procedure");
    let mut top: Vec<_> = shapes.into_iter().collect();
    top.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (k, n) in &top {
        println!("{n:5} {k}");
    }
    println!(
        "-- root causes (calls and instance blocks followed to the procedure refused for its own reason)"
    );
    let mut top: Vec<_> = roots.into_iter().collect();
    top.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (k, n) in &top {
        println!("{n:5} {k}");
    }
    if std::env::var_os("SHOW_RAW_ROOTS").is_some() {
        let mut top: Vec<_> = roots_raw.into_iter().collect();
        top.sort_by_key(|x| std::cmp::Reverse(x.1));
        for (k, n) in top.iter().take(60) {
            println!("RAWROOT {n:5} {k}");
        }
    }
    println!(
        "-- module-level declarations (variables, constants, types, enums, Implements): {decls}, generated {decls_ok}"
    );
    let mut top: Vec<_> = decl_reasons.into_iter().collect();
    top.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (k, n) in &top {
        println!("{n:5} {k}");
    }
    if std::env::var_os("SHOW_RAW_ROOTS").is_some() {
        let mut top: Vec<_> = var_roots.into_iter().collect();
        top.sort_by_key(|x| std::cmp::Reverse(x.1));
        for (k, n) in &top {
            println!("VARROOT {n:5} {k}");
        }
    }
    println!("-- refusal reasons (raw), top 40");
    let mut top: Vec<_> = total_reasons.into_iter().collect();
    top.sort_by_key(|x| std::cmp::Reverse(x.1));
    for (k, n) in top.iter().take(40) {
        println!("{n:5} {k}");
    }
}

/// A reason with the quoted names replaced, so that buckets add up.
fn shape(reason: &str) -> String {
    let mut out = String::new();
    let mut quoted = false;
    for c in reason.chars() {
        match (c, quoted) {
            ('`', false) => {
                quoted = true;
                out.push_str("`_`");
            }
            ('`', true) => quoted = false,
            (_, true) => {}
            (c, false) => out.push(c),
        }
    }
    for lead in ["local of unsupported type ", "calls "] {
        if let Some(i) = out.find(lead) {
            out.truncate(i + lead.len());
            out.push('_');
        }
    }
    out
}
