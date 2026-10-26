//! Indirect-table classification metrics on the local material (ignored).
#![allow(missing_docs)]
mod common;

use std::collections::{BTreeMap, BTreeSet};

use vba_pcode::*;

#[test]
#[ignore = "needs the local reference project"]
fn indirect_table_coverage() {
    let Some(p) = common::load() else { return };
    let v = version_word(&p.project_stream).unwrap();
    let layout = Layout::select(v, p.sys_kind).unwrap();
    let (mut table, mut reach, mut shape, mut arrays, mut unknown) = (0, 0, 0, 0, 0);
    let mut per_module: Vec<f64> = Vec::new();
    let mut by_kind: BTreeMap<IndirectKind, (usize, usize)> = BTreeMap::new();
    // How many shape-found records are the target of some pointer-looking
    // dword elsewhere in the table.
    let (mut shaped, mut shaped_referenced) = (0usize, 0usize);
    let mut rec_only = 0usize;
    for m in &p.modules {
        let cache = ModuleCache::parse(&m.cache, &layout).unwrap();
        let t = &m.cache[cache.tables.indirect.clone()];
        let map = IndirectMap::build(&cache);
        let c = map.coverage();
        table += c.table;
        reach += c.reachable;
        shape += c.by_shape;
        arrays += c.arrays;
        unknown += c.unknown;
        per_module.push(c.understood_percent());
        rec_only += RecordMap::collect(&cache)
            .coverage(&cache)
            .map(|c| c.covered)
            .unwrap_or(0);
        // Regions never overlap and stay inside the table.
        let mut end = 0;
        for (r, k) in &map.regions {
            assert!(r.start >= end && r.end <= t.len());
            end = r.end;
            let e = by_kind.entry(*k).or_default();
            e.0 += 1;
            e.1 += r.len();
        }
        let targets: BTreeSet<u32> = t
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        for (r, k) in &map.regions {
            if k.by_shape() && *k != IndirectKind::SlotRecord {
                shaped += 1;
                shaped_referenced += targets.contains(&(r.start as u32)) as usize;
            }
        }
        let slots = map
            .regions
            .iter()
            .filter(|(_, k)| *k == IndirectKind::SlotRecord)
            .count();
        let slots_ref = map
            .regions
            .iter()
            .filter(|(r, k)| *k == IndirectKind::SlotRecord && targets.contains(&(r.start as u32)))
            .count();
        shaped += slots;
        shaped_referenced += slots_ref;
    }
    per_module.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |n: usize| n as f64 * 100.0 / table as f64;
    println!("indirect table {table} bytes");
    println!(
        "before: reachable records {rec_only} ({:.1}%)",
        pct(rec_only)
    );
    println!(
        "after: reachable {reach} ({:.1}%), by shape {shape} ({:.1}%), arrays/fill {arrays} ({:.1}%), unknown {unknown} ({:.1}%)",
        pct(reach),
        pct(shape),
        pct(arrays),
        pct(unknown)
    );
    println!(
        "understood share per module: min {:.1}%, median {:.1}%, max {:.1}%",
        per_module[0],
        per_module[per_module.len() / 2],
        per_module[per_module.len() - 1]
    );
    println!("regions by kind (count, bytes): {by_kind:?}");
    println!("shape-found records referenced by a dword elsewhere: {shaped_referenced}/{shaped}");
    assert!(unknown * 100 / table <= 18);
    assert!(shaped_referenced * 100 / shaped >= 99);
}
