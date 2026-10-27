//! Work and output amplification in the decoder and disassembler, and
//! seeded structure-aware mutation of synthetic caches.
//!
//! Everything is synthetic and deterministic. Inputs derived from real files
//! are covered by the opt-in corpus tests, which read their data through
//! environment variables and are skipped when those are unset.

mod common;

use std::time::{Duration, Instant};

use common::synth::{self, CacheSpec};
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

const FUNCTION_AT: usize = 0;
const ARGS_AT: usize = 80;
const ARG_STRIDE: usize = 32;

/// An indirect table with one function record whose argument chain has
/// `chain` records; when `cyclic`, the last one points back to the first.
fn indirect_with_chain(chain: usize, cyclic: bool) -> Vec<u8> {
    let mut t = synth::function_record(synth::user_name(0).map_into());
    t[40..44].copy_from_slice(&(ARGS_AT as u32).to_le_bytes());
    t.resize(ARGS_AT + chain * ARG_STRIDE + 32, 0);
    for i in 0..chain {
        let at = ARGS_AT + i * ARG_STRIDE;
        let next = if i + 1 < chain {
            (ARGS_AT + (i + 1) * ARG_STRIDE) as u32
        } else if cyclic {
            ARGS_AT as u32
        } else {
            0
        };
        t[at + 20..at + 24].copy_from_slice(&next.to_le_bytes());
        t[at..at + 2].copy_from_slice(&0x0020u16.to_le_bytes());
    }
    t
}

trait MapInto {
    fn map_into(self) -> u16;
}
impl MapInto for [u8; 2] {
    fn map_into(self) -> u16 {
        u16::from_le_bytes(self)
    }
}

/// `n` PROC_HEADER instructions, all naming the record at 0, spread over
/// lines of at most 1000 instructions.
fn proc_header_lines(n: usize) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();
    let mut left = n;
    while left > 0 {
        let k = left.min(1000);
        let mut l = Vec::new();
        for _ in 0..k {
            l.extend_from_slice(&synth::word(150, 6));
            l.extend_from_slice(&(FUNCTION_AT as u32).to_le_bytes());
        }
        lines.push(l);
        left -= k;
    }
    lines
}

fn listed_args(d: &Disassembly) -> usize {
    d.lines
        .iter()
        .flat_map(|l| &l.instructions)
        .flat_map(|i| &i.operands)
        .map(|o| match o {
            Operand::Procedure(p) => p.args.len(),
            _ => 0,
        })
        .sum()
}

/// One procedure record with a 4000-record argument chain named by many
/// instructions: the listing must not copy the chain for each of them.
#[test]
fn procedure_listing_is_budgeted() {
    let spec = CacheSpec {
        lines: proc_header_lines(200),
        indirect: indirect_with_chain(4000, false),
        ..CacheSpec::default()
    };
    let bytes = synth::build_cache(&spec);
    let project = synth::build_project_stream(0x97, &["A"]);
    let pc = ProjectCache::parse(&project, 1).unwrap();
    let cache = ModuleCache::parse(&bytes, &pc.layout).unwrap();
    let started = Instant::now();
    let d = Disassembly::of_module(&cache, &pc.idents);
    let args = listed_args(&d);
    assert!(args <= 1 << 18, "{args} argument records listed");
    assert!(started.elapsed() < Duration::from_secs(30));
    // The map and coverage walks stay cheap too.
    let started = Instant::now();
    let _ = RecordMap::collect(&cache);
    let _ = IndirectMap::build(&cache);
    assert!(started.elapsed() < Duration::from_secs(10));
}

/// A cyclic argument chain named by tens of thousands of instructions costs
/// one walk, not one per instruction.
#[test]
fn cyclic_argument_chains_are_walked_once() {
    let spec = CacheSpec {
        lines: proc_header_lines(40_000),
        indirect: indirect_with_chain(64, true),
        ..CacheSpec::default()
    };
    let bytes = synth::build_cache(&spec);
    let project = synth::build_project_stream(0x97, &["A"]);
    let pc = ProjectCache::parse(&project, 1).unwrap();
    let cache = ModuleCache::parse(&bytes, &pc.layout).unwrap();
    let started = Instant::now();
    let map = RecordMap::collect(&cache);
    assert!(map.records.len() <= 2);
    let d = Disassembly::of_module(&cache, &pc.idents);
    assert!(listed_args(&d) <= 1 << 18);
    assert!(started.elapsed() < Duration::from_secs(20), "{:?}", started.elapsed());
}

fn valid_spec(rng: &mut Rng) -> CacheSpec {
    let mut lines = Vec::new();
    for _ in 0..1 + rng.below(12) {
        let mut l = Vec::new();
        for _ in 0..rng.below(6) {
            // Random but plausible stored opcodes with random operands.
            l.extend_from_slice(&synth::word(rng.below(260) as u16, rng.below(64) as u16));
            for _ in 0..rng.below(5) {
                l.push(rng.next() as u8);
            }
        }
        if l.len() % 2 == 1 {
            l.pop();
        }
        lines.push(l);
    }
    CacheSpec {
        lines,
        indirect: indirect_with_chain(1 + rng.below(6), rng.below(2) == 0),
        members: [None, Some(0), Some(3)][rng.below(3)],
        ..CacheSpec::default()
    }
}

/// Random instruction streams, then every u16/u32 field of the cache in turn
/// set to boundary values, through parse, coverage, disassembly and the
/// record and indirect maps.
#[test]
fn mutated_synthetic_caches_never_panic() {
    let project = synth::build_project_stream(0x97, &["A", "B", "C"]);
    let pc = ProjectCache::parse(&project, 1).unwrap();
    let layouts = [
        pc.layout,
        Layout::select(0xD7, 3).unwrap(),
        Layout::select(0x70, 1).unwrap(),
        Layout::select(0x50, 1).unwrap(),
    ];
    let mut rng = Rng(0x5EED_0030);
    let started = Instant::now();
    let exercise = |bytes: &[u8], layout: &Layout| {
        if let Ok(c) = ModuleCache::parse(bytes, layout) {
            let _ = c.coverage();
            let _ = c.code_area_check();
            let d = Disassembly::of_module(&c, &pc.idents);
            let _ = d.to_string();
            let _ = RecordMap::collect(&c);
            let _ = IndirectMap::build(&c).coverage();
        }
    };
    for _ in 0..200 {
        let base = synth::build_cache(&valid_spec(&mut rng));
        for layout in &layouts {
            exercise(&base, layout);
        }
        // Boundary values at every 2-aligned offset of the header region and
        // a sample of the rest.
        let step = 2 + 2 * rng.below(8);
        for at in (0..base.len().saturating_sub(4)).step_by(step) {
            for v in [0u32, 1, 0xFFFF, 0xFFFF_FFFF, 0x8000_0000, base.len() as u32 + 1] {
                let mut m = base.clone();
                m[at..at + 4].copy_from_slice(&v.to_le_bytes());
                exercise(&m, &layouts[0]);
            }
        }
        // Truncation and random damage.
        for _ in 0..8 {
            let mut m = base.clone();
            let cut = rng.below(m.len() + 1);
            m.truncate(cut);
            exercise(&m, &layouts[rng.below(layouts.len())]);
            let mut m = base.clone();
            for _ in 0..1 + rng.below(6) {
                let i = rng.below(m.len());
                m[i] = rng.next() as u8;
            }
            exercise(&m, &layouts[rng.below(layouts.len())]);
        }
    }
    assert!(started.elapsed() < Duration::from_secs(120), "{:?}", started.elapsed());
}

/// Project-stream walks over mutated valid streams and random bytes.
#[test]
fn mutated_project_streams_never_panic() {
    let base = synth::build_project_stream(0x97, &["Alpha", "Beta", "Gamma"]);
    let layouts = [
        Layout::select(0x97, 1).unwrap(),
        Layout::select(0xD7, 3).unwrap(),
        Layout::select(0x70, 1).unwrap(),
        Layout::select(0x50, 1).unwrap(),
    ];
    let mut rng = Rng(0x5EED_0031);
    for _ in 0..20_000 {
        let mut m = base.clone();
        for _ in 0..1 + rng.below(5) {
            if m.is_empty() {
                break;
            }
            match rng.below(4) {
                0 => {
                    let i = rng.below(m.len());
                    m[i] = [0, 0xFF, 0x7F, 0x80, rng.next() as u8][rng.below(5)];
                }
                1 if m.len() >= 2 => {
                    let i = rng.below(m.len() - 1);
                    m[i..i + 2].copy_from_slice(&[0xFF, 0xFF]);
                }
                2 => {
                    let n = rng.below(m.len() + 1);
                    m.truncate(n);
                }
                _ => {
                    let at = rng.below(m.len() + 1);
                    m.insert(at, rng.next() as u8);
                }
            }
        }
        for l in &layouts {
            let _ = parse_identifiers(&m, l);
        }
        let _ = ProjectCache::parse(&m, 1);
    }
}

/// The single hook a coverage-guided fuzzer needs: every byte-level entry
/// point of the crate on one input (read as a project stream and as a module
/// cache under each layout).
pub fn fuzz_entry(bytes: &[u8]) {
    let ident_layout = Layout::select(0x97, 1).unwrap();
    let idents = parse_identifiers(bytes, &ident_layout).unwrap_or(IdentTable {
        names: Vec::new(),
        announced: 0,
        dummies: 0,
        end: 0,
        stream_len: 0,
    });
    let _ = ProjectCache::parse(bytes, 1);
    for l in [
        Layout::select(0x97, 1).unwrap(),
        Layout::select(0xD7, 3).unwrap(),
        Layout::select(0x70, 1).unwrap(),
        Layout::select(0x50, 1).unwrap(),
    ] {
        let _ = decode_line(bytes, &l);
        if let Ok(c) = ModuleCache::parse(bytes, &l) {
            let _ = c.coverage();
            let _ = Disassembly::of_module(&c, &idents).to_string();
            let _ = RecordMap::collect(&c);
            let _ = IndirectMap::build(&c).coverage();
        }
    }
}

#[test]
fn fuzz_entry_accepts_arbitrary_bytes() {
    let mut rng = Rng(0x5EED_0032);
    for _ in 0..3000 {
        let n = rng.below(400);
        let b: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        fuzz_entry(&b);
    }
    fuzz_entry(&synth::build_cache(&CacheSpec::default()));
    fuzz_entry(&synth::build_project_stream(0x97, &["A"]));
}
