//! Tolerant reading of `dir` streams with unrecognized records.

mod common;

use common::*;
use vba_format::{
    Error, Limits, ReadMode, WriteOptions, decompress, keys, read_project, read_project_with,
    write_project,
};

/// A `dir` stream with unknown records at several places: project level
/// (after SYSKIND, before the references, before the module count, before the
/// terminator) and module level (between records and before a terminator).
fn dir_with_unknown(modules: &[ModSpec]) -> Vec<u8> {
    let mut v = Vec::new();
    let info = project_info();
    v.extend(&info[..10]); // PROJECTSYSKIND
    v.extend(rec(0x0777, b"after-syskind"));
    v.extend(&info[10..]);
    v.extend(rec(0x0778, b""));
    v.extend(spec_references());
    v.extend(rec(0x0779, &[1, 2, 3]));
    v.extend(rec(0x000F, &u16le(modules.len() as u16)));
    v.extend(rec(0x0013, &u16le(0xFFFF)));
    for (i, m) in modules.iter().enumerate() {
        let rec_bytes = module_record(m);
        if i == 1 {
            let pos = rec_bytes
                .windows(6)
                .position(|w| w == [0x1E, 0x00, 0x04, 0x00, 0x00, 0x00])
                .unwrap();
            v.extend(&rec_bytes[..pos]);
            v.extend(rec(0x07A0, b"in-module-1"));
            v.extend(&rec_bytes[pos..]);
        } else if i == 2 {
            let end = rec_bytes.len() - 6;
            v.extend(&rec_bytes[..end]);
            v.extend(rec(0x07A1, b"end-of-module-2"));
            v.extend(&rec_bytes[end..]);
        } else {
            v.extend(rec_bytes);
        }
    }
    v.extend(rec(0x07A2, b"between-last-module-and-terminator"));
    v.extend(u16le(0x0010));
    v.extend(u32le(0));
    v
}

/// A container whose project has the specification's three modules and the
/// given decompressed `dir` stream.
fn container(dir: &[u8]) -> Vec<u8> {
    let mut streams = vec![
        ("VBA/dir".to_owned(), vba_format::compress(dir)),
        (
            "VBA/_VBA_PROJECT".to_owned(),
            vec![0xCC, 0x61, 0xFF, 0xFF, 0x00, 0x01, 0x00],
        ),
        ("PROJECT".to_owned(), project_stream("")),
    ];
    for (name, src) in [
        ("ThisWorkbook", SRC_THISWORKBOOK),
        ("Sheet1", SRC_SHEET1),
        ("UserForm1", SRC_USERFORM1),
    ] {
        streams.push((format!("VBA/{name}"), module_stream(&[], src.as_bytes())));
    }
    build_cfb(&streams, &[])
}

fn dir_of(bin: &[u8]) -> Vec<u8> {
    let (streams, _) = dump_cfb(bin);
    let (_, d) = streams
        .iter()
        .find(|(p, _)| p.eq_ignore_ascii_case("/VBA/dir"))
        .unwrap();
    decompress(d, 1 << 20).unwrap()
}

fn unknown_ids(mut b: &[u8]) -> Vec<u16> {
    let mut ids = Vec::new();
    while !b.is_empty() {
        ids.push(u16::from_le_bytes([b[4], b[5]]));
        let len = u32::from_le_bytes([b[6], b[7], b[8], b[9]]) as usize;
        b = &b[10 + len..];
    }
    ids
}

#[test]
fn strict_mode_rejects_unknown_records() {
    let bin = container(&dir_with_unknown(&spec_modules()));
    let err = read_project(&bin, &Limits::default()).unwrap_err();
    assert!(
        matches!(err, Error::DirUnexpectedRecord { found: 0x0777, .. }),
        "{err:?}"
    );
}

#[test]
fn tolerant_mode_preserves_unknown_records_as_opaque_blobs() {
    let bin = container(&dir_with_unknown(&spec_modules()));
    let p = read_project_with(&bin, &Limits::default(), ReadMode::Tolerant).unwrap();
    assert_eq!(p.name, "VBAProject");
    assert_eq!(p.modules.len(), 3);
    assert_eq!(p.references.len(), 4);
    let blob = p.preserved.get(keys::DIR_UNKNOWN).expect("project slot");
    assert_eq!(unknown_ids(blob), [0x0777, 0x0778, 0x0779, 0x07A2]);
    assert!(
        p.modules[0]
            .preserved
            .get(keys::MODULE_DIR_UNKNOWN)
            .is_none()
    );
    let m1 = p.modules[1]
        .preserved
        .get(keys::MODULE_DIR_UNKNOWN)
        .unwrap();
    assert_eq!(&m1[10..], b"in-module-1");
    let m2 = p.modules[2]
        .preserved
        .get(keys::MODULE_DIR_UNKNOWN)
        .unwrap();
    assert_eq!(&m2[10..], b"end-of-module-2");
}

#[test]
fn untouched_projects_write_the_original_dir_stream() {
    let dir = dir_with_unknown(&spec_modules());
    let bin = container(&dir);
    let p = read_project_with(&bin, &Limits::default(), ReadMode::Tolerant).unwrap();
    let out = write_project(&p, &WriteOptions::default()).unwrap();
    assert_eq!(dir_of(&out), dir);
}

#[test]
fn rewritten_dir_streams_keep_unknown_records_in_place() {
    let dir = dir_with_unknown(&spec_modules());
    let bin = container(&dir);
    let mut p = read_project_with(&bin, &Limits::default(), ReadMode::Tolerant).unwrap();
    p.modules[1].doc_string = "Changed".into();
    p.description = "New description".into();
    let out = write_project(&p, &WriteOptions::default()).unwrap();

    let mut modules = spec_modules();
    modules[1].doc = "Changed";
    let mut expected = dir_with_unknown(&modules);
    let old = pair(0x0005, 0x0040, "Example VBA Project Description");
    let new = pair(0x0005, 0x0040, "New description");
    let at = expected.windows(old.len()).position(|w| w == old).unwrap();
    expected.splice(at..at + old.len(), new);
    assert_eq!(dir_of(&out), expected);

    let again = read_project_with(&out, &Limits::default(), ReadMode::Tolerant).unwrap();
    assert_eq!(
        again.preserved.get(keys::DIR_UNKNOWN),
        p.preserved.get(keys::DIR_UNKNOWN)
    );
    assert_eq!(again.modules[1].doc_string, "Changed");
}

#[test]
fn removing_a_preserved_slot_removes_those_records() {
    let bin = container(&dir_with_unknown(&spec_modules()));
    let mut p = read_project_with(&bin, &Limits::default(), ReadMode::Tolerant).unwrap();
    p.preserved.remove(keys::DIR_UNKNOWN);
    let out = write_project(&p, &WriteOptions::default()).unwrap();
    assert!(read_project(&out, &Limits::default()).is_err());
    let q = read_project_with(&out, &Limits::default(), ReadMode::Tolerant).unwrap();
    assert!(q.preserved.get(keys::DIR_UNKNOWN).is_none());
    assert!(
        q.modules[1]
            .preserved
            .get(keys::MODULE_DIR_UNKNOWN)
            .is_some()
    );
}

#[test]
fn tolerant_mode_equals_strict_on_conforming_input() {
    let bin = sample_project_bin();
    let a = read_project(&bin, &Limits::default()).unwrap();
    let b = read_project_with(&bin, &Limits::default(), ReadMode::Tolerant).unwrap();
    assert_eq!(a, b);
}

#[test]
fn tolerant_mode_still_rejects_truncation_and_misplaced_known_records() {
    let mut dir = project_info();
    dir.extend(u16le(0x0777));
    dir.extend(u32le(0xFFFF_FFF0));
    let bin = container(&dir);
    assert!(read_project_with(&bin, &Limits::default(), ReadMode::Tolerant).is_err());

    let mut dir = project_info();
    dir.extend(rec(0x0013, &u16le(0xFFFF)));
    let bin = container(&dir);
    assert!(matches!(
        read_project_with(&bin, &Limits::default(), ReadMode::Tolerant),
        Err(Error::DirUnexpectedRecord { .. })
    ));
}
