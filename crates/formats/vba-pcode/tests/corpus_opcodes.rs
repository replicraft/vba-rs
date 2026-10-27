//! Opcode histogram over every project of `VBA_PCODE_CORPUS` and
//! `VBA_PCODE_STREAMS` (ignored; local material only): how often each
//! canonical opcode occurs, in how many projects, and which occurring
//! opcodes the evidence table still lists as unseen.
mod common;

use std::collections::{BTreeMap, BTreeSet};

use vba_pcode::*;

#[test]
#[ignore = "needs a local corpus (VBA_PCODE_CORPUS / VBA_PCODE_STREAMS)"]
fn corpus_opcode_histogram() {
    // opcode -> (occurrences, projects)
    let mut hist: BTreeMap<u16, (usize, BTreeSet<usize>)> = BTreeMap::new();
    let mut undecoded = 0usize;
    for (n, path) in common::corpus_files().iter().enumerate() {
        let Some(p) = common::load_path(path) else {
            continue;
        };
        let Ok(v) = version_word(&p.project_stream) else {
            continue;
        };
        let Ok(layout) = Layout::select(v, p.sys_kind) else {
            continue;
        };
        let Ok(idents) = parse_identifiers(&p.project_stream, &layout) else {
            continue;
        };
        for m in &p.modules {
            let Ok(cache) = ModuleCache::parse(&m.cache, &layout) else {
                continue;
            };
            let dis = Disassembler::new(&cache, &idents);
            for i in 0..cache.lines.lines.len() {
                let Some(d) = dis.decode(i) else { continue };
                if d.error.is_some() {
                    undecoded += 1;
                }
                for ins in &d.instructions {
                    let e = hist.entry(ins.opcode.canonical()).or_default();
                    e.0 += 1;
                    e.1.insert(n);
                }
            }
        }
    }
    println!("undecoded lines {undecoded}");
    println!("distinct opcodes {}", hist.len());
    for (k, (count, projects)) in &hist {
        let op = Opcode::from_canonical(*k).expect("canonical");
        println!(
            "OP {k:3} {:34} {:?} count {count} projects {}",
            op.label(),
            op.evidence(),
            projects.len()
        );
    }
}
