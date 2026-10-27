//! Built-in name evidence over every project of `VBA_PCODE_CORPUS` and
//! `VBA_PCODE_STREAMS` (ignored; local material only): per unknown built-in
//! index the number of uses, of projects and the tokens shared by all source
//! lines that use it, and any contradiction of a confirmed entry.
mod common;

use std::collections::{BTreeMap, BTreeSet};

use vba_pcode::*;

fn norm(t: &[u8]) -> String {
    let s: String = t
        .iter()
        .map(|&b| char::from(b))
        .collect::<String>()
        .to_lowercase();
    s.trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches(|c| "%&!#@$".contains(c))
        .to_string()
}

/// Every name reference reachable from an operand.
fn names_of(op: &Operand, out: &mut Vec<Name>) {
    match op {
        Operand::Name { name, .. } => out.push(name.clone()),
        Operand::Procedure(p) => {
            out.push(p.name.clone());
            if let Some(l) = &p.library {
                out.push(l.clone());
            }
            for a in &p.args {
                out.push(a.name.clone());
                if let ResolvedType::Named(n) = &a.ty {
                    out.push(n.clone());
                }
            }
            if let ResolvedType::Named(n) = &p.return_type {
                out.push(n.clone());
            }
        }
        Operand::Variable(v) => {
            out.push(v.name.clone());
            if let ResolvedType::Named(n) = &v.ty {
                out.push(n.clone());
            }
        }
        Operand::UserType { name, .. } => out.push(name.clone()),
        Operand::Object { class: Some(c), .. } => out.push(c.clone()),
        Operand::NameList(v) => out.extend(v.iter().cloned()),
        _ => {}
    }
}

type Evidence = (usize, BTreeSet<usize>, Option<BTreeSet<String>>);

#[test]
#[ignore = "needs a local corpus (VBA_PCODE_CORPUS / VBA_PCODE_STREAMS)"]
fn corpus_builtin_evidence() {
    let mut cands: BTreeMap<u16, Evidence> = BTreeMap::new();
    let mut contradictions = 0usize;
    let mut confirmed_uses = 0usize;
    for (n, path) in common::corpus_files().iter().enumerate() {
        let Some(p) = common::load_path(path) else {
            continue;
        };
        if p.compiled_only {
            continue;
        }
        let Ok(v) = version_word(&p.project_stream) else {
            continue;
        };
        let Ok(layout) = Layout::select(v, p.sys_kind) else {
            continue;
        };
        let Ok(idents) = parse_identifiers(&p.project_stream, &layout) else {
            continue;
        };
        let user: BTreeSet<String> = idents
            .names
            .iter()
            .map(|n| n.iter().map(|&b| char::from(b)).collect::<String>().to_lowercase())
            .collect();
        for m in &p.modules {
            let Ok(cache) = ModuleCache::parse(&m.cache, &layout) else {
                continue;
            };
            let dis = Disassembler::new(&cache, &idents);
            let src = common::logical_lines(&m.source);
            if src.len() != cache.lines.lines.len() {
                continue;
            }
            for (i, sl) in src.iter().enumerate() {
                let l = dis.line(i);
                let all: BTreeSet<String> = sl.toks.iter().map(|t| norm(&t.text)).collect();
                // A built-in index is never a user identifier: drop those, and
                // punctuation and literals.
                let mut names = Vec::new();
                for ins in &l.instructions {
                    for o in &ins.operands {
                        names_of(o, &mut names);
                    }
                }
                // Tokens explained by the other resolved operands of the line.
                let explained: BTreeSet<String> = names
                    .iter()
                    .filter_map(|n| n.spelling().map(str::to_lowercase))
                    .collect();
                let toks: BTreeSet<String> = all
                    .iter()
                    .filter(|t| {
                        t.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                            && !user.contains(*t)
                            && !explained.contains(*t)
                    })
                    .cloned()
                    .collect();
                for nm in &names {
                    match nm {
                        Name::Builtin(s) => {
                            confirmed_uses += 1;
                            if !all.contains(&s.to_lowercase()) {
                                contradictions += 1;
                                println!(
                                    "CONTRADICTION {s} | {}",
                                    String::from_utf8_lossy(&sl.raw).trim()
                                );
                            }
                        }
                        Name::UnknownBuiltin(k) => {
                            let e = cands.entry(*k).or_default();
                            e.0 += 1;
                            e.1.insert(n);
                            e.2 = Some(match e.2.take() {
                                None => toks.clone(),
                                Some(s) => s.intersection(&toks).cloned().collect(),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    println!("confirmed uses {confirmed_uses}, contradictions {contradictions}");
    for (k, (uses, files, common_toks)) in &cands {
        println!(
            "idx {k}: uses {uses} projects {} common {:?}",
            files.len(),
            common_toks
        );
    }
}
