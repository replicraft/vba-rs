//! Built-in name evidence against the local project (ignored).
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

type Evidence = (usize, Option<BTreeSet<String>>, Vec<String>);

#[test]
#[ignore = "needs the local project (VBA_PCODE_PROJECT)"]
#[allow(clippy::needless_range_loop)]
fn explore_unknown_builtins() {
    let Some(p) = common::load() else { return };
    let v = version_word(&p.project_stream).unwrap();
    let layout = Layout::select(v, p.sys_kind).unwrap();
    let idents = parse_identifiers(&p.project_stream, &layout).unwrap();
    let (mut total_builtin, mut resolved) = (0usize, 0usize);
    let mut cands: BTreeMap<u16, Evidence> = BTreeMap::new();
    for m in &p.modules {
        let cache = ModuleCache::parse(&m.cache, &layout).unwrap();
        let dis = Disassembler::new(&cache, &idents);
        let src = common::logical_lines(&m.source);
        for i in 0..cache.lines.lines.len() {
            let l = dis.line(i);
            let toks: BTreeSet<String> = src[i].toks.iter().map(|t| norm(&t.text)).collect();
            let mut names = Vec::new();
            for ins in &l.instructions {
                for o in &ins.operands {
                    names_of(o, &mut names);
                }
            }
            for n in &names {
                match n {
                    Name::Builtin(_) => {
                        total_builtin += 1;
                        resolved += 1;
                    }
                    Name::UnknownBuiltin(k) => {
                        total_builtin += 1;
                        let e = cands.entry(*k).or_default();
                        e.0 += 1;
                        e.1 = Some(match e.1.take() {
                            None => toks.clone(),
                            Some(s) => s.intersection(&toks).cloned().collect(),
                        });
                        if e.2.len() < 6 {
                            e.2.push(String::from_utf8_lossy(&src[i].raw).trim().to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    println!(
        "builtin name operands {total_builtin}, resolved {resolved} ({:.2}%)",
        resolved as f64 * 100.0 / total_builtin as f64
    );
    assert!(resolved >= 392, "resolution regressed");
    for (k, (n, c, ex)) in &cands {
        println!("idx {k}: uses {n}, common tokens {c:?}");
        for e in ex {
            println!("     | {e}");
        }
    }
}
