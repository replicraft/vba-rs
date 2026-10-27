//! Mutation of the module caches and project streams of local real files.
//!
//! The files named by `VBA_PCODE_CORPUS` (a directory) and
//! `VBA_PCODE_PROJECT` (one file) are untrusted local data that is never
//! committed. These tests read them through the variables only and return at
//! once when both are unset; only counts are printed.
//!
//! `VBA_PCODE_CORPUS=<dir> cargo test -p vba-pcode --test hostile_corpus -- --nocapture`

mod common;

use std::time::{Duration, Instant};

use vba_pcode::*;

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

fn projects() -> Vec<common::Project> {
    let mut v: Vec<common::Project> = common::corpus_files()
        .iter()
        .filter_map(|p| common::load_path(p))
        .collect();
    v.extend(common::load());
    v
}

fn exercise(cache: &[u8], layout: &Layout, idents: &IdentTable) {
    let Ok(c) = ModuleCache::parse(cache, layout) else {
        return;
    };
    let _ = c.coverage();
    let _ = c.code_area_check();
    let d = Disassembly::of_module(&c, idents);
    let _ = d.to_string();
    let _ = RecordMap::collect(&c);
    let _ = IndirectMap::build(&c).coverage();
}

#[test]
fn mutated_real_caches_never_panic() {
    let projects = projects();
    if projects.is_empty() {
        return;
    }
    let mut rng = Rng(0x5EED_0050);
    let (mut modules, mut runs) = (0usize, 0usize);
    let started = Instant::now();
    for p in &projects {
        let Ok(v) = version_word(&p.project_stream) else {
            continue;
        };
        let Ok(layout) = Layout::select(v, p.sys_kind) else {
            continue;
        };
        let Ok(idents) = parse_identifiers(&p.project_stream, &layout) else {
            continue;
        };
        // Project stream: boundary values in the leading words, random damage.
        for _ in 0..60 {
            let mut m = p.project_stream.clone();
            for _ in 0..1 + rng.below(4) {
                let i = rng.below(m.len().saturating_sub(1));
                m[i] = rng.next() as u8;
            }
            let _ = parse_identifiers(&m, &layout);
            let _ = ProjectCache::parse(&m, p.sys_kind);
            runs += 1;
        }
        for m in p.modules.iter().take(8) {
            modules += 1;
            exercise(&m.cache, &layout, &idents);
            // Every u32 of the header region and of the table length fields.
            let head = m.cache.len().min(0x120);
            for at in (0..head.saturating_sub(4)).step_by(2) {
                for v in [0u32, 1, 0xFFFF_FFFF, 0x8000_0000, m.cache.len() as u32 + 1] {
                    let mut x = m.cache.clone();
                    x[at..at + 4].copy_from_slice(&v.to_le_bytes());
                    exercise(&x, &layout, &idents);
                    runs += 1;
                }
            }
            for _ in 0..40 {
                let mut x = m.cache.clone();
                match rng.below(4) {
                    0 => {
                        for _ in 0..1 + rng.below(8) {
                            let i = rng.below(x.len());
                            x[i] ^= 1 << rng.below(8);
                        }
                    }
                    1 => {
                        let n = rng.below(x.len() + 1);
                        x.truncate(n);
                    }
                    2 => {
                        let s = rng.below(x.len());
                        let e = (s + 1 + rng.below(300)).min(x.len());
                        let chunk = x[s..e].to_vec();
                        let at = rng.below(x.len() + 1);
                        x.splice(at..at, chunk);
                    }
                    _ => {
                        let s = rng.below(x.len());
                        let e = (s + 1 + rng.below(300)).min(x.len());
                        x.drain(s..e);
                    }
                }
                exercise(&x, &layout, &idents);
                runs += 1;
            }
            assert!(
                started.elapsed() < Duration::from_secs(1200),
                "time budget exhausted"
            );
        }
    }
    println!("{} projects, {modules} modules, {runs} mutations", projects.len());
}
