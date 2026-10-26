//! Property tests over the whole path: container, `dir` stream, modules.
//!
//! Hostile or damaged input must produce an `Err`, never a panic, and the
//! configured limits must hold.

mod common;

use common::packages::*;
use common::*;
use proptest::prelude::*;
use vba_format::codepage;
use vba_format::{
    Error, Limits, ReadMode, WriteOptions, decompress, extract_project, keys, read_document,
    read_project, read_project_with, write_project,
};
use vba_project::{Module, ModuleKind, VbaProject};

/// Valid documents of every supported kind, to be damaged.
fn corpus() -> &'static [Vec<u8>] {
    static CORPUS: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();
    CORPUS.get_or_init(|| {
        vec![
            sample().to_vec(),
            docm(),
            legacy_document("Macros", &[("WordDocument", vec![1; 20])]),
            legacy_document("_VBA_PROJECT_CUR", &[("Workbook", vec![2; 20])]),
            presentation(false, 1, 0xE391_C05F),
            presentation(true, 1, 0xE391_C05F),
        ]
    })
}

fn exercise(bytes: &[u8], limits: &Limits) {
    let _ = extract_project(bytes, limits);
    let _ = read_document(bytes, limits, ReadMode::Strict);
    let _ = read_document(bytes, limits, ReadMode::Tolerant);
    let _ = read_project(bytes, limits);
}

fn small_limits() -> Limits {
    Limits {
        max_container_bytes: 1 << 20,
        max_stream_bytes: 1 << 19,
        max_decompressed_bytes: 1 << 19,
        max_entries: 64,
        max_archive_entries: 64,
        ..Limits::default()
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Flipping bytes anywhere in a valid document never panics.
    #[test]
    fn damaged_documents_never_panic(
        which in 0usize..6,
        edits in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..24),
        cut in prop::option::of(any::<prop::sample::Index>()),
    ) {
        let mut bytes = corpus()[which].clone();
        for (at, b) in &edits {
            let i = at.index(bytes.len());
            bytes[i] = *b;
        }
        if let Some(c) = cut {
            bytes.truncate(c.index(bytes.len() + 1));
        }
        exercise(&bytes, &small_limits());
    }

    /// Every prefix-length truncation of every corpus document is handled.
    #[test]
    fn truncated_documents_never_panic(which in 0usize..6, frac in 0.0f64..1.0) {
        let full = &corpus()[which];
        let len = ((full.len() as f64) * frac) as usize;
        exercise(&full[..len], &small_limits());
    }

    /// Arbitrary bytes, with and without the magic numbers of the supported
    /// containers.
    #[test]
    fn arbitrary_bytes_never_panic(
        magic in 0u8..3,
        body in prop::collection::vec(any::<u8>(), 0..2000),
    ) {
        let mut bytes = match magic {
            0 => Vec::new(),
            1 => b"PK\x03\x04".to_vec(),
            _ => vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
        };
        bytes.extend(body);
        exercise(&bytes, &small_limits());
    }

    /// Whatever the limits, a successful read respects the input limit and
    /// the extracted project respects the container limit.
    #[test]
    fn limits_are_respected(which in 0usize..6, max_in in 0u64..20_000, max_entries in 1usize..80) {
        let limits = Limits {
            max_container_bytes: max_in,
            max_entries,
            max_archive_entries: max_entries,
            ..Limits::default()
        };
        let bytes = &corpus()[which];
        if let Ok(ex) = extract_project(bytes, &limits) {
            prop_assert!(bytes.len() as u64 <= max_in);
            prop_assert!(ex.bytes.len() as u64 <= max_in);
        }
        if read_project(bytes, &limits).is_ok() {
            prop_assert!(bytes.len() as u64 <= max_in);
        }
    }

    /// A highly compressible entry is refused when it expands beyond what
    /// the ratio limit allows, whatever its size.
    #[test]
    fn zip_bombs_hit_the_ratio_limit(mib in 2usize..8, ratio in 1usize..400) {
        let bomb = vec![0u8; mib << 20];
        let pkg = package("word/document.xml", "vbaProject.bin", "word/vbaProject.bin", &bomb, false);
        let limits = Limits {
            max_archive_ratio: ratio,
            ..Limits::default()
        };
        let err = extract_project(&pkg, &limits).unwrap_err();
        prop_assert!(matches!(err, Error::ArchiveLimit(_)), "{:?}", err);
    }
}

// ------------------------------------------- code pages through the writer

/// Text that `code_page` can hold: decoded from random bytes.
fn page_text(code_page: u16, bytes: &[u8]) -> String {
    codepage::decode(code_page, bytes)
        .unwrap()
        .chars()
        .filter(|c| *c != '\u{FFFD}' && *c != '\r' && *c != '\n' && *c != '\0' && !c.is_control())
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Fresh projects in the OEM code pages round-trip their source text.
    #[test]
    fn oem_code_page_sources_round_trip(
        page_ix in 0usize..15,
        lines in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..40), 0..20),
    ) {
        const PAGES: [u16; 15] =
            [437, 737, 775, 850, 852, 855, 857, 858, 860, 861, 862, 863, 864, 865, 869];
        let page = PAGES[page_ix];
        let source: String = lines
            .iter()
            .map(|l| format!("{}\r\n", page_text(page, l)))
            .collect();
        let mut p = VbaProject::new("Proj");
        p.code_page = page;
        p.add_module(Module::new("Mod1", ModuleKind::Standard, source.clone())).unwrap();
        let bin = write_project(&p, &WriteOptions::default()).unwrap();
        let back = read_project(&bin, &Limits::default()).unwrap();
        prop_assert_eq!(back.code_page, page);
        prop_assert_eq!(&back.module("Mod1").unwrap().source, &source);
        // Writing again changes nothing observable.
        let again = write_project(&back, &WriteOptions::default()).unwrap();
        prop_assert_eq!(dump_cfb(&again).0, dump_cfb(&bin).0);
    }
}

// ----------------------------------------------- unknown dir records

/// A `dir` stream with the given unknown records inserted at three
/// project-level boundaries.
fn dir_with(unknown: &[(u16, Vec<u8>)], spots: &[usize]) -> Vec<u8> {
    let info = project_info();
    let modules = spec_modules();
    let mut v = Vec::new();
    let emit = |v: &mut Vec<u8>, spot: usize| {
        for ((id, data), s) in unknown.iter().zip(spots) {
            if *s == spot {
                v.extend(rec(*id, data));
            }
        }
    };
    v.extend(&info[..10]);
    emit(&mut v, 0);
    v.extend(&info[10..]);
    v.extend(spec_references());
    emit(&mut v, 1);
    v.extend(rec(0x000F, &u16le(modules.len() as u16)));
    v.extend(rec(0x0013, &u16le(0xFFFF)));
    for m in &modules {
        v.extend(module_record(m));
    }
    emit(&mut v, 2);
    v.extend(u16le(0x0010));
    v.extend(u32le(0));
    v
}

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

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Unknown records at any project-level boundary are an error when
    /// strict and survive an unrelated edit when tolerant.
    #[test]
    fn unknown_records_survive_in_tolerant_mode(
        records in prop::collection::vec(
            (0x0100u16..0x7000, prop::collection::vec(any::<u8>(), 0..40), 0usize..3),
            1..8,
        ),
    ) {
        let mut spots: Vec<usize> = records.iter().map(|r| r.2).collect();
        spots.sort();
        let unknown: Vec<(u16, Vec<u8>)> =
            records.iter().map(|r| (r.0, r.1.clone())).collect();
        let dir = dir_with(&unknown, &spots);
        let bin = container(&dir);

        prop_assert!(read_project(&bin, &Limits::default()).is_err());
        let mut p = read_project_with(&bin, &Limits::default(), ReadMode::Tolerant).unwrap();
        prop_assert!(p.preserved.contains(keys::DIR_UNKNOWN));

        // Untouched: the original dir stream comes back.
        let out = write_project(&p, &WriteOptions::default()).unwrap();
        let (streams, _) = dump_cfb(&out);
        let (_, d) = streams.iter().find(|(n, _)| n == "/VBA/dir").unwrap();
        prop_assert_eq!(decompress(d, 1 << 20).unwrap(), dir.clone());

        // A forced rewrite keeps every record, at the same anchors.
        p.description = "Changed".into();
        let again = write_project(&p, &WriteOptions::default()).unwrap();
        let (streams, _) = dump_cfb(&again);
        let (_, d) = streams.iter().find(|(n, _)| n == "/VBA/dir").unwrap();
        prop_assert_ne!(decompress(d, 1 << 20).unwrap(), dir);
        let q = read_project_with(&again, &Limits::default(), ReadMode::Tolerant).unwrap();
        prop_assert_eq!(q.description.as_str(), "Changed");
        prop_assert_eq!(
            q.preserved.get(keys::DIR_UNKNOWN),
            p.preserved.get(keys::DIR_UNKNOWN)
        );
    }
}
