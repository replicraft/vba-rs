//! Truncated, mutated and random input: typed errors, no panics, limits.

mod common;

use common::builder::build;
use proptest::prelude::*;
use vba_typelib::{Limits, TypeLib, TypelibError, msft, parse_msft};

fn touch(lib: &TypeLib) {
    // Walk everything a consumer would, to catch latent panics.
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
        if let Some(a) = &t.alias {
            let _ = lib.describe(a);
        }
    }
}

#[test]
fn every_truncation_is_an_error_or_a_valid_library() {
    let data = build(&common::sample());
    assert!(parse_msft(&data).is_ok());
    for cut in 0..data.len() {
        if let Ok(l) = parse_msft(&data[..cut]) {
            touch(&l);
        }
    }
    // Cutting inside the segment directory or tables is always an error.
    assert!(parse_msft(&data[..data.len() / 2]).is_err());
}

#[test]
fn every_single_byte_corruption_is_survivable() {
    let data = build(&common::sample());
    for pos in 0..data.len() {
        for val in [0x00u8, 0x01, 0x7f, 0x80, 0xff] {
            let mut d = data.clone();
            d[pos] = val;
            if let Ok(l) = parse_msft(&d) {
                touch(&l);
            }
        }
    }
}

#[test]
fn huge_counts_are_rejected_cheaply() {
    let data = build(&common::sample());
    // nrtypeinfos = u32::MAX
    let mut d = data.clone();
    d[0x20..0x24].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        parse_msft(&d),
        Err(TypelibError::LimitExceeded(_) | TypelibError::OutOfRange { .. })
    ));
    // A small limit is honoured.
    let tight = Limits {
        max_types: 3,
        ..Limits::default()
    };
    assert_eq!(
        msft::parse(&data, tight),
        Err(TypelibError::LimitExceeded("too many types"))
    );
    let tight = Limits {
        max_members: 2,
        ..Limits::default()
    };
    assert_eq!(
        msft::parse(&data, tight),
        Err(TypelibError::LimitExceeded("too many members"))
    );
    let tight = Limits {
        max_params: 1,
        ..Limits::default()
    };
    assert_eq!(
        msft::parse(&data, tight),
        Err(TypelibError::LimitExceeded("too many parameters"))
    );
}

#[test]
fn type_description_cycle_is_cut_by_the_depth_limit() {
    // Point a PTR typedesc entry at itself.
    let mut spec = common::sample();
    // IShape.Area takes Ptr(R8): the only PTR entries come from the sample.
    spec.types.truncate(4);
    let data = build(&spec);
    let l = parse_msft(&data).unwrap();
    assert!(!l.types.is_empty());
    // Find the typedesc segment through the segment directory and make
    // every entry a pointer to offset 0.
    let n = u32::from_le_bytes(data[0x20..0x24].try_into().unwrap()) as usize;
    let sd = 0x54 + 4 * n;
    let seg9 = sd + 9 * 16;
    let off = u32::from_le_bytes(data[seg9..seg9 + 4].try_into().unwrap()) as usize;
    let len = u32::from_le_bytes(data[seg9 + 4..seg9 + 8].try_into().unwrap()) as usize;
    assert!(len >= 8);
    let mut d = data.clone();
    let mut p = off;
    while p + 8 <= off + len {
        d[p..p + 2].copy_from_slice(&26u16.to_le_bytes());
        d[p + 4..p + 8].copy_from_slice(&0u32.to_le_bytes());
        p += 8;
    }
    match parse_msft(&d) {
        Err(TypelibError::LimitExceeded("type description too deep")) => {}
        other => panic!("expected the depth limit, got {other:?}"),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn random_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let _ = parse_msft(&bytes);
        let _ = vba_typelib::load(&bytes);
    }

    #[test]
    fn random_bytes_behind_a_valid_signature_never_panic(
        tail in proptest::collection::vec(any::<u8>(), 0..4096)
    ) {
        let mut d = b"MSFT\x02\x00\x01\x00".to_vec();
        d.extend_from_slice(&tail);
        if let Ok(l) = parse_msft(&d) { touch(&l); }
    }

    #[test]
    fn mutated_library_never_panics(
        edits in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..24)
    ) {
        let mut d = build(&common::sample());
        for (i, v) in edits {
            let at = i.index(d.len());
            d[at] = v;
        }
        if let Ok(l) = parse_msft(&d) { touch(&l); }
    }

    #[test]
    fn mutated_pe_never_panics(
        edits in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..24)
    ) {
        use common::pe::{Res, Ty, build_pe};
        let mut d = build_pe(true, &[Res {
            ty: Ty::Name("TYPELIB"), id: 1, lang: 0, data: build(&common::sample()),
        }]);
        for (i, v) in edits {
            let at = i.index(d.len());
            d[at] = v;
        }
        let _ = vba_typelib::load_all(&d);
    }
}
