//! Truncated, mutated and random SLTG input: typed errors, no panics.

mod common;

use common::sltg::build_sltg;
use common::sltg_libs::sample;
use proptest::prelude::*;
use vba_typelib::{TypeLib, load_all, parse_any, parse_sltg};

fn touch(lib: &TypeLib) {
    for t in &lib.types {
        for f in &t.functions {
            let _ = lib.describe(&f.ret);
            for p in &f.params {
                let _ = lib.describe(&p.ty);
            }
        }
        for v in &t.variables {
            let _ = lib.describe(&v.ty);
        }
        for i in &t.implements {
            let _ = lib.local(i.target);
            let _ = lib.import(i.target);
        }
    }
}

#[test]
fn every_truncation_is_an_error_or_a_valid_library() {
    let data = build_sltg(&sample());
    assert!(parse_sltg(&data).is_ok());
    for cut in 0..data.len() {
        if let Ok(l) = parse_sltg(&data[..cut]) {
            touch(&l);
        }
    }
    assert!(parse_sltg(&data[..data.len() / 2]).is_err());
    assert!(parse_sltg(&data[..data.len() - 1]).is_err());
}

#[test]
fn every_single_byte_corruption_is_survivable() {
    let data = build_sltg(&sample());
    for pos in 0..data.len() {
        for val in [0x00u8, 0x01, 0x04, 0x7f, 0x80, 0xfe, 0xff] {
            let mut d = data.clone();
            d[pos] = val;
            if let Ok(l) = parse_sltg(&d) {
                touch(&l);
            }
        }
    }
}

#[test]
fn every_word_set_to_an_extreme_is_survivable() {
    let data = build_sltg(&sample());
    for pos in (0..data.len() - 1).step_by(2) {
        for val in [0x0000u16, 0x7fff, 0x8000, 0xffff] {
            let mut d = data.clone();
            d[pos..pos + 2].copy_from_slice(&val.to_le_bytes());
            if let Ok(l) = parse_sltg(&d) {
                touch(&l);
            }
        }
    }
}

#[test]
fn huge_counts_are_rejected_cheaply() {
    let data = build_sltg(&sample());
    // Block count 65535.
    let mut d = data.clone();
    d[4..6].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(parse_sltg(&d).is_err());
    // Every block length 0xffffffff.
    let mut d = data;
    let n = u16::from_le_bytes([d[4], d[5]]) as usize;
    for i in 0..n - 1 {
        let o = 0x24 + 8 * i;
        d[o..o + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    }
    assert!(parse_sltg(&d).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn random_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let _ = parse_sltg(&bytes);
        let _ = parse_any(&bytes);
        let _ = vba_typelib::load(&bytes);
    }

    #[test]
    fn random_bytes_behind_a_valid_signature_never_panic(
        tail in proptest::collection::vec(any::<u8>(), 0..4096)
    ) {
        let mut d = b"SLTG".to_vec();
        d.extend_from_slice(&tail);
        if let Ok(l) = parse_sltg(&d) { touch(&l); }
    }

    #[test]
    fn mutated_library_never_panics(
        edits in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..24)
    ) {
        let mut d = build_sltg(&sample());
        for (i, v) in edits {
            let at = i.index(d.len());
            d[at] = v;
        }
        if let Ok(l) = parse_sltg(&d) { touch(&l); }
    }

    #[test]
    fn mutated_pe_never_panics(
        edits in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..24)
    ) {
        use common::pe::{Res, Ty, build_pe};
        let mut d = build_pe(false, &[Res {
            ty: Ty::Name("TYPELIB"), id: 1, lang: 0, data: build_sltg(&sample()),
        }]);
        for (i, v) in edits {
            let at = i.index(d.len());
            d[at] = v;
        }
        let _ = load_all(&d);
    }
}
