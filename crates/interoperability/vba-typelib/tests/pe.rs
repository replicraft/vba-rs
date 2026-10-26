//! TYPELIB resources in synthetic PE32 / PE32+ images.

mod common;

use common::builder::{LibSpec, build};
use common::pe::{Res, Ty, build_pe};
use vba_typelib::{TypelibError, load, load_all, pe};

fn lib(name: &str) -> Vec<u8> {
    build(&LibSpec::new(name))
}

#[test]
fn reads_typelib_from_pe32_and_pe64() {
    for pe64 in [false, true] {
        let img = build_pe(
            pe64,
            &[Res {
                ty: Ty::Name("TYPELIB"),
                id: 1,
                lang: 0,
                data: lib("InPe"),
            }],
        );
        let found = pe::typelib_resources(&img).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, Some(1));
        assert_eq!(found[0].language, Some(0));
        assert!(found[0].name.is_none());
        let l = load(&img).unwrap();
        assert_eq!(l.name, "InPe");
    }
}

#[test]
fn several_resources_and_other_types_are_distinguished() {
    let img = build_pe(
        true,
        &[
            Res {
                ty: Ty::Id(16),
                id: 1,
                lang: 0x409,
                data: vec![1, 2, 3, 4, 5],
            },
            Res {
                ty: Ty::Name("TYPELIB"),
                id: 1,
                lang: 0,
                data: lib("First"),
            },
            Res {
                ty: Ty::Name("TYPELIB"),
                id: 2,
                lang: 0,
                data: lib("Second"),
            },
            Res {
                ty: Ty::Name("typelib2"),
                id: 3,
                lang: 0,
                data: lib("Decoy"),
            },
        ],
    );
    let all = load_all(&img).unwrap();
    let names: Vec<_> = all.iter().map(|(id, l)| (*id, l.name.as_str())).collect();
    assert_eq!(names, vec![(1, "First"), (2, "Second")]);
    assert_eq!(load(&img).unwrap().name, "First");
}

#[test]
fn pe_without_typelib_reports_no_resource() {
    let img = build_pe(
        false,
        &[Res {
            ty: Ty::Id(16),
            id: 1,
            lang: 0,
            data: vec![0; 8],
        }],
    );
    assert_eq!(load(&img), Err(TypelibError::NoTypelibResource));
    assert_eq!(
        load_all(&img).map(|_| ()),
        Err(TypelibError::NoTypelibResource)
    );
}

#[test]
fn raw_typelib_loads_as_id_one() {
    let raw = lib("Raw");
    assert_eq!(load(&raw).unwrap().name, "Raw");
    let all = load_all(&raw).unwrap();
    assert_eq!((all[0].0, all[0].1.name.as_str()), (1, "Raw"));
}

#[test]
fn damaged_pe_is_a_typed_error_never_a_panic() {
    let img = build_pe(
        true,
        &[Res {
            ty: Ty::Name("TYPELIB"),
            id: 1,
            lang: 0,
            data: lib("Cut"),
        }],
    );
    for cut in 0..img.len() {
        let _ = load_all(&img[..cut]);
    }
    // A data entry pointing outside the image.
    let mut bad = img.clone();
    for b in bad.iter_mut().skip(0x200).take(256) {
        *b = 0xff;
    }
    let r = load_all(&bad);
    assert!(r.is_err());
    assert!(matches!(load(b"MZ"), Err(TypelibError::Pe(_))));
}
