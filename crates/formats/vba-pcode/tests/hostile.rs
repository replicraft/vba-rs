//! Property tests: hostile input never panics and invariants hold.

mod common;

use common::synth::{self, CacheSpec};
use proptest::prelude::*;
use vba_pcode::*;

fn layouts() -> Vec<Layout> {
    vec![
        Layout::select(0x97, 1).unwrap(),
        Layout::select(0xD7, 3).unwrap(),
        Layout::select(0x70, 1).unwrap(),
        Layout::select(0x50, 1).unwrap(),
    ]
}

fn valid_cache() -> Vec<u8> {
    let mut spec = CacheSpec::default();
    let mut l = synth::word(32, 0).to_vec();
    l.extend_from_slice(&synth::user_name(0));
    l.extend_from_slice(&synth::word(224, 0));
    l.extend_from_slice(&0u16.to_le_bytes());
    l.extend_from_slice(&1u16.to_le_bytes());
    l.extend_from_slice(b"x\0");
    spec.lines = vec![l, Vec::new(), synth::word(105, 0).to_vec()];
    spec.members = Some(2);
    synth::build_cache(&spec)
}

proptest! {
    #[test]
    fn decode_line_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        for l in layouts() {
            let d = decode_line(&bytes, &l);
            // Instruction offsets strictly increase and stay inside the line.
            let mut last = None;
            for i in &d.instructions {
                prop_assert!(i.offset < bytes.len());
                if let Some(p) = last {
                    prop_assert!(i.offset > p);
                }
                last = Some(i.offset);
            }
            if d.is_complete() && !bytes.is_empty() {
                prop_assert!(!d.instructions.is_empty());
            }
        }
    }

    #[test]
    fn module_parse_never_panics_on_noise(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        for l in layouts() {
            if let Ok(c) = ModuleCache::parse(&bytes, &l) {
                let mut pos = 0;
                for r in &c.regions {
                    prop_assert_eq!(r.span.start, pos);
                    pos = r.span.end;
                }
                prop_assert_eq!(pos, bytes.len());
            }
        }
    }

    #[test]
    fn mutated_valid_cache_never_panics(
        flips in proptest::collection::vec((0usize..4096, any::<u8>()), 1..12),
        cut in proptest::option::of(0usize..4096),
    ) {
        let project = synth::build_project_stream(0x97, &["A", "B"]);
        let pc = ProjectCache::parse(&project, 1).unwrap();
        let mut bytes = valid_cache();
        for (i, b) in flips {
            let n = bytes.len();
            bytes[i % n] = b;
        }
        if let Some(c) = cut {
            let n = bytes.len();
            bytes.truncate(c % (n + 1));
        }
        for l in layouts() {
            if let Ok(c) = ModuleCache::parse(&bytes, &l) {
                // Region map re-encodes the input exactly.
                let mut rebuilt = Vec::new();
                for r in &c.regions {
                    rebuilt.extend_from_slice(&bytes[r.span.clone()]);
                }
                prop_assert_eq!(&rebuilt, &bytes);
                let _ = c.coverage();
                let _ = c.code_area_check();
                // Disassembly is total.
                let d = Disassembly::of_module(&c, &pc.idents);
                let _ = d.to_string();
            }
        }
    }

    #[test]
    fn identifier_walk_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..400), tail in any::<bool>()) {
        let mut v = synth::build_project_stream(0x97, &["Alpha", "Beta"]);
        if tail {
            // Hostile bytes appended to a valid prefix.
            v.truncate(0x1E);
            v.extend_from_slice(&bytes);
        } else {
            v = bytes;
            if v.len() >= 2 { v[0] = 0xCC; v[1] = 0x61; }
        }
        for l in layouts() {
            let _ = parse_identifiers(&v, &l);
        }
    }

    #[test]
    fn record_parsers_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..200), off in any::<u32>()) {
        for l in layouts() {
            let _ = FunctionRecord::parse(&bytes, off, &l);
            let _ = VariableRecord::parse(&bytes, off, &l);
            let _ = ArgumentRecord::parse(&bytes, off, &l);
            let _ = argument_chain(&bytes, Some(off), &l, 16);
            let _ = type_descriptor(&bytes, &bytes, off, &l);
            let _ = type_from_field(off, &bytes, &bytes, &l);
        }
        let _ = UdtRecord::parse(&bytes, off);
        let _ = DeclarationRecord::parse(&bytes, off as u16);
        let _ = object_entry(&bytes, off as usize);
    }

    #[test]
    fn synthetic_caches_decompose_and_decode(
        lines in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..40), 0..20),
        members in proptest::option::of(0usize..5),
        refs in proptest::collection::vec("[a-z\\\\*]{0,12}", 0..4),
    ) {
        // Keep code even-sized so the alignment/slot rules apply uniformly.
        let lines: Vec<Vec<u8>> = lines.into_iter().map(|mut l| { if l.len() % 2 == 1 { l.pop(); } l }).collect();
        let spec = CacheSpec { lines: lines.clone(), members, references: refs, ..CacheSpec::default() };
        let bytes = synth::build_cache(&spec);
        let layout = Layout::select(0x97, 1).unwrap();
        let c = ModuleCache::parse(&bytes, &layout).unwrap();
        prop_assert_eq!(c.lines.lines.len(), lines.len());
        prop_assert_eq!(c.members.as_ref().map(|m| m.records.len()), members);
        let chk = c.code_area_check();
        prop_assert_eq!((chk.holes, chk.overlaps), (0, 0));
        prop_assert_eq!(c.coverage().unattributed, 0);
        // Each line's bytes are recovered.
        for (i, l) in lines.iter().enumerate() {
            let r = c.lines.code_range(i, bytes.len()).unwrap();
            match r {
                Some(r) => prop_assert_eq!(&bytes[r], &l[..]),
                None => prop_assert!(l.is_empty()),
            }
        }
    }
}
