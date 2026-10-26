//! Extracting projects from host documents and replacing them in packages.

mod common;

use std::io::{Cursor, Read, Write};

use common::packages::*;
use common::*;
use vba_format::container::ProjectLocation;
use vba_format::{
    Error, Limits, ReadMode, WriteOptions, extract_project, locate_ooxml_project, read_document,
    read_project, replace_ooxml_project, write_project,
};
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

#[test]
fn project_is_found_through_the_relationships() {
    let pkg = docm();
    let limits = Limits::default();
    assert_eq!(
        locate_ooxml_project(&pkg, &limits).unwrap(),
        "word/vbaProject.bin"
    );
    let ex = extract_project(&pkg, &limits).unwrap();
    assert_eq!(
        ex.location,
        ProjectLocation::OoxmlPart("word/vbaProject.bin".into())
    );
    assert_eq!(ex.bytes, sample());
    let (project, loc) = read_document(&pkg, &limits, ReadMode::Strict).unwrap();
    assert_eq!(loc, ex.location);
    assert_eq!(project.modules.len(), 5);
}

#[test]
fn file_names_are_not_guessed() {
    // The referenced part has an unusual name and place; a decoy with the
    // conventional name is not referenced and must be ignored.
    let pkg = package(
        "word/document.xml",
        "macros/Project42.dat",
        "word/macros/Project42.dat",
        sample(),
        true,
    );
    let limits = Limits::default();
    assert_eq!(
        locate_ooxml_project(&pkg, &limits).unwrap(),
        "word/macros/Project42.dat"
    );
    assert_eq!(extract_project(&pkg, &limits).unwrap().bytes, sample());
}

#[test]
fn targets_are_resolved_like_opc_part_names() {
    let limits = Limits::default();
    // Absolute target.
    let p = package(
        "xl/workbook.xml",
        "/xl/vba/p.bin",
        "xl/vba/p.bin",
        sample(),
        false,
    );
    assert_eq!(locate_ooxml_project(&p, &limits).unwrap(), "xl/vba/p.bin");
    // Parent-directory target.
    let p = package(
        "xl/sub/workbook.xml",
        "../vba/p.bin",
        "xl/vba/p.bin",
        sample(),
        false,
    );
    assert_eq!(locate_ooxml_project(&p, &limits).unwrap(), "xl/vba/p.bin");
    // Case-insensitive part names and percent-encoding.
    let p = package(
        "xl/workbook.xml",
        "VBA%20Dir/P.BIN",
        "xl/vba dir/p.bin",
        sample(),
        false,
    );
    assert_eq!(
        locate_ooxml_project(&p, &limits).unwrap(),
        "xl/VBA Dir/P.BIN"
    );
    assert!(extract_project(&p, &limits).is_ok());
    // A target that climbs out of the package is rejected.
    let p = package("a.xml", "../../x.bin", "x.bin", sample(), false);
    assert!(matches!(
        locate_ooxml_project(&p, &limits),
        Err(Error::Package(_))
    ));
}

#[test]
fn packages_without_a_usable_project_are_reported() {
    let limits = Limits::default();
    // No macros.
    let docx = zip_of(&[
        (
            "_rels/.rels",
            rels(&[(OFFICE_DOCUMENT, "word/document.xml", "")]),
        ),
        ("word/document.xml", "<doc/>".into()),
    ]);
    assert!(matches!(
        extract_project(&docx, &limits),
        Err(Error::NoVbaProject(_))
    ));
    // No officeDocument relationship.
    let odd = zip_of(&[(
        "_rels/.rels",
        rels(&[("http://example.com/x", "a.xml", "")]),
    )]);
    assert!(matches!(
        extract_project(&odd, &limits),
        Err(Error::Package(_))
    ));
    // No relationships part at all.
    let bare = zip_of(&[("a.txt", "hi".into())]);
    assert!(matches!(
        extract_project(&bare, &limits),
        Err(Error::Package(_))
    ));
    // The relationship points at a part that is not there.
    let dangling = zip_of(&[
        (
            "_rels/.rels",
            rels(&[(OFFICE_DOCUMENT, "word/document.xml", "")]),
        ),
        ("word/document.xml", "<doc/>".into()),
        (
            "word/_rels/document.xml.rels",
            rels(&[(VBA, "vbaProject.bin", "")]),
        ),
    ]);
    assert!(matches!(
        extract_project(&dangling, &limits),
        Err(Error::Package(_))
    ));
    // External vbaProject targets do not count.
    let external = zip_of(&[
        (
            "_rels/.rels",
            rels(&[(OFFICE_DOCUMENT, "word/document.xml", "")]),
        ),
        ("word/document.xml", "<doc/>".into()),
        (
            "word/_rels/document.xml.rels",
            rels(&[(VBA, "vbaProject.bin", r#" TargetMode="External""#)]),
        ),
    ]);
    assert!(matches!(
        extract_project(&external, &limits),
        Err(Error::NoVbaProject(_))
    ));
    // Two vbaProject relationships are ambiguous.
    let two = zip_of(&[
        (
            "_rels/.rels",
            rels(&[(OFFICE_DOCUMENT, "word/document.xml", "")]),
        ),
        ("word/document.xml", "<doc/>".into()),
        (
            "word/_rels/document.xml.rels",
            rels(&[(VBA, "a.bin", ""), (VBA, "b.bin", "")]),
        ),
        ("word/a.bin", "x".into()),
        ("word/b.bin", "y".into()),
    ]);
    assert!(matches!(
        extract_project(&two, &limits),
        Err(Error::Package(_))
    ));
    // Malformed XML.
    let bad = zip_of(&[("_rels/.rels", "<Relationships><Relationship Type=".into())]);
    assert!(matches!(
        extract_project(&bad, &limits),
        Err(Error::Package(_))
    ));
}

// ------------------------------------------------------------ replacement

/// Everything the zip format records about an entry, plus its raw
/// (compressed) bytes.
#[derive(Debug, PartialEq, Eq)]
struct RawEntry {
    name: Vec<u8>,
    method: CompressionMethod,
    crc: u32,
    compressed: u64,
    size: u64,
    modified: Option<DateTime>,
    attributes: u32,
    comment: String,
    extra: Option<Vec<u8>>,
    raw: Vec<u8>,
}

fn raw_entries(bytes: &[u8]) -> (Vec<RawEntry>, Vec<u8>) {
    let mut a = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let comment = a.comment().to_vec();
    let mut out = Vec::new();
    for i in 0..a.len() {
        let mut f = a.by_index_raw(i).unwrap();
        let mut raw = Vec::new();
        f.read_to_end(&mut raw).unwrap();
        out.push(RawEntry {
            name: f.name_raw().to_vec(),
            method: f.compression(),
            crc: f.crc32(),
            compressed: f.compressed_size(),
            size: f.size(),
            modified: f.last_modified(),
            attributes: f.external_attributes(),
            comment: f.comment().to_owned(),
            extra: f.extra_data(),
            raw,
        });
    }
    (out, comment)
}

#[test]
fn replacing_the_project_leaves_every_other_entry_identical() {
    let pkg = docm();
    let limits = Limits::default();

    // The new project: the sample with one module edited.
    let mut project = read_project(sample(), &limits).unwrap();
    project
        .module_mut("Module1")
        .unwrap()
        .source
        .push_str("' replaced\r\n");
    let new_bin = write_project(&project, &WriteOptions::default()).unwrap();

    let out = replace_ooxml_project(&pkg, &new_bin, &limits).unwrap();
    assert_ne!(out, pkg);

    let (before, comment_before) = raw_entries(&pkg);
    let (after, comment_after) = raw_entries(&out);
    assert_eq!(comment_before, comment_after, "archive comment");
    assert_eq!(before.len(), after.len());
    let mut compared = 0;
    for (b, a) in before.iter().zip(&after) {
        // Order and names are the same for all entries.
        assert_eq!(b.name, a.name);
        if b.name == b"word/vbaProject.bin" {
            continue;
        }
        // Everything else, metadata and compressed bytes, is identical.
        assert_eq!(b, a, "{}", String::from_utf8_lossy(&b.name));
        compared += 1;
    }
    assert_eq!(compared, before.len() - 1);

    // The replaced entry keeps its name, method, time and attributes.
    let (b, a) = (
        before
            .iter()
            .find(|e| e.name == b"word/vbaProject.bin")
            .unwrap(),
        after
            .iter()
            .find(|e| e.name == b"word/vbaProject.bin")
            .unwrap(),
    );
    assert_eq!(
        (b.method, b.modified, b.attributes),
        (a.method, a.modified, a.attributes)
    );

    // And the package now yields the new project, found the same way.
    let ex = extract_project(&out, &limits).unwrap();
    assert_eq!(ex.bytes, new_bin);
    let reread = read_project(&ex.bytes, &limits).unwrap();
    assert!(
        reread
            .module("Module1")
            .unwrap()
            .source
            .ends_with("' replaced\r\n")
    );
}

#[test]
fn replacement_preserves_the_decompressed_content_of_other_entries() {
    let pkg = docm();
    let out = replace_ooxml_project(&pkg, sample(), &Limits::default()).unwrap();
    let (mut a, mut b) = (
        ZipArchive::new(Cursor::new(&pkg[..])).unwrap(),
        ZipArchive::new(Cursor::new(&out[..])).unwrap(),
    );
    for i in 0..a.len() {
        let (mut x, mut y) = (a.by_index(i).unwrap(), b.by_index(i).unwrap());
        let (mut dx, mut dy) = (Vec::new(), Vec::new());
        x.read_to_end(&mut dx).unwrap();
        y.read_to_end(&mut dy).unwrap();
        assert_eq!(dx, dy);
        assert_eq!(x.name_raw(), y.name_raw());
    }
}

#[test]
fn replacement_requires_an_existing_project_and_a_compound_file() {
    let limits = Limits::default();
    let docx = zip_of(&[
        (
            "_rels/.rels",
            rels(&[(OFFICE_DOCUMENT, "word/document.xml", "")]),
        ),
        ("word/document.xml", "<doc/>".into()),
    ]);
    assert!(matches!(
        replace_ooxml_project(&docx, sample(), &limits),
        Err(Error::NoVbaProject(_))
    ));
    assert!(matches!(
        replace_ooxml_project(&docm(), b"not a cfb", &limits),
        Err(Error::Package(_))
    ));
}

// ----------------------------------------------------------------- limits

#[test]
fn zip_bombs_are_rejected() {
    // A 40 MiB entry of zeros deflates to about 40 KiB.
    let bomb = vec![0u8; 40 << 20];
    let pkg = package(
        "word/document.xml",
        "vbaProject.bin",
        "word/vbaProject.bin",
        &bomb,
        false,
    );
    assert!(pkg.len() < 200_000, "fixture is small: {}", pkg.len());

    // Over the ratio limit.
    let err = extract_project(&pkg, &Limits::default()).unwrap_err();
    assert!(matches!(err, Error::ArchiveLimit(_)), "{err:?}");

    // Over the size limit, with the ratio limit out of the way.
    let limits = Limits {
        max_container_bytes: 1 << 20,
        max_archive_ratio: usize::MAX / 2,
        ..Limits::default()
    };
    // (the package itself is small enough; the entry is not)
    let err = extract_project(&pkg, &limits).unwrap_err();
    assert!(matches!(err, Error::ArchiveLimit(_)), "{err:?}");

    // Replacement never inflates the old project part, so it is unaffected.
    assert!(replace_ooxml_project(&pkg, sample(), &Limits::default()).is_ok());
}

#[test]
fn archive_entry_count_is_limited() {
    let limits = Limits {
        max_archive_entries: 3,
        ..Limits::default()
    };
    assert!(matches!(
        extract_project(&docm(), &limits),
        Err(Error::ArchiveLimit(_))
    ));
}

#[test]
fn oversized_input_is_rejected_up_front() {
    let limits = Limits {
        max_container_bytes: 100,
        ..Limits::default()
    };
    assert!(matches!(
        extract_project(&docm(), &limits),
        Err(Error::InputTooLarge { .. })
    ));
}

#[test]
fn duplicate_entry_names_are_rejected() {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    // The zip writer refuses duplicates, so patch one name afterwards.
    for name in ["_rels/.rels", "_rels/.relX"] {
        w.start_file(name, opts(CompressionMethod::Stored, 2))
            .unwrap();
        w.write_all(b"x").unwrap();
    }
    let mut bytes = w.finish().unwrap().into_inner();
    let mut n = 0;
    while let Some(at) = bytes.windows(11).position(|w| w == b"_rels/.relX") {
        bytes[at + 10] = b's';
        n += 1;
    }
    assert_eq!(n, 2, "local header and central directory");
    assert!(matches!(
        extract_project(&bytes, &Limits::default()),
        Err(Error::Package(_))
    ));
}

// ------------------------------------------------------- legacy documents

#[test]
fn word_documents_are_read_from_the_macros_storage() {
    let doc = legacy_document(
        "Macros",
        &[("WordDocument", vec![1, 2, 3]), ("1Table", vec![4])],
    );
    let limits = Limits::default();
    let (project, loc) = read_document(&doc, &limits, ReadMode::Strict).unwrap();
    assert_eq!(loc, ProjectLocation::WordMacros);
    let expected = read_project(sample(), &limits).unwrap();
    for (k, v) in expected.preserved.iter() {
        assert_eq!(project.preserved.get(k), Some(v), "{k}");
    }
    assert_eq!(project.preserved.len(), expected.preserved.len());
    assert_eq!(project, expected);
}

#[test]
fn excel_workbooks_are_read_from_the_vba_project_cur_storage() {
    let doc = legacy_document("_VBA_PROJECT_CUR", &[("Workbook", vec![9; 50])]);
    let limits = Limits::default();
    let (project, loc) = read_document(&doc, &limits, ReadMode::Strict).unwrap();
    assert_eq!(loc, ProjectLocation::ExcelVbaProjectCur);
    assert_eq!(project, read_project(sample(), &limits).unwrap());
    assert_eq!(project.module("Module1").unwrap().source, SRC_MODULE1);
}

#[test]
fn legacy_documents_without_a_project_are_reported() {
    let limits = Limits::default();
    let word = build_cfb(&[("WordDocument".into(), vec![0; 10])], &[]);
    assert!(matches!(
        extract_project(&word, &limits),
        Err(Error::NoVbaProject(_))
    ));
    let excel = build_cfb(&[("Workbook".into(), vec![0; 10])], &[]);
    assert!(matches!(
        extract_project(&excel, &limits),
        Err(Error::NoVbaProject(_))
    ));
}

#[test]
fn a_raw_storage_is_returned_as_is() {
    let bin = sample().to_vec();
    let ex = extract_project(&bin, &Limits::default()).unwrap();
    assert_eq!(ex.location, ProjectLocation::RawStorage);
    assert_eq!(ex.bytes, bin);
}

#[test]
fn encrypted_and_unrecognized_inputs_are_unsupported() {
    let limits = Limits::default();
    let enc = build_cfb(
        &[
            ("EncryptedPackage".into(), vec![0; 16]),
            ("EncryptionInfo".into(), vec![0; 16]),
        ],
        &[],
    );
    assert!(matches!(
        extract_project(&enc, &limits),
        Err(Error::UnsupportedContainer(_))
    ));
    assert!(matches!(
        extract_project(b"hello world, not a document", &limits),
        Err(Error::UnsupportedContainer(_))
    ));
    let other = build_cfb(&[("Something".into(), vec![0; 4])], &[]);
    assert!(matches!(
        extract_project(&other, &limits),
        Err(Error::UnsupportedContainer(_))
    ));
    // A dir stream directly in _VBA_PROJECT_CUR is a layout we cannot verify.
    let odd = build_cfb(
        &[("_VBA_PROJECT_CUR/dir".into(), vec![0; 4])],
        &[StorageSpec {
            path: "_VBA_PROJECT_CUR",
            clsid: [0; 16],
            state_bits: 0,
        }],
    );
    assert!(matches!(
        extract_project(&odd, &limits),
        Err(Error::UnsupportedContainer(_))
    ));
}

// ------------------------------------------------------------- PowerPoint

#[test]
fn powerpoint_projects_are_found_through_the_persist_directory() {
    let limits = Limits::default();
    for compressed in [false, true] {
        let ppt = presentation(compressed, 1, 0xE391_C05F);
        let ex = extract_project(&ppt, &limits).unwrap();
        assert_eq!(
            ex.location,
            ProjectLocation::PowerPointVbaProjectStg { compressed }
        );
        assert_eq!(ex.bytes, sample(), "newest edit wins");
        let (project, _) = read_document(&ppt, &limits, ReadMode::Strict).unwrap();
        assert_eq!(project.modules.len(), 5);
    }
}

#[test]
fn powerpoint_problems_are_reported() {
    let limits = Limits::default();
    assert!(matches!(
        extract_project(&presentation(false, 0, 0xE391_C05F), &limits),
        Err(Error::NoVbaProject(_))
    ));
    assert!(matches!(
        extract_project(&presentation(false, 1, 0xF3D1_C4DF), &limits),
        Err(Error::UnsupportedContainer(_))
    ));
    // Truncating the document stream breaks the offsets.
    let ppt = presentation(false, 1, 0xE391_C05F);
    let (mut streams, _) = dump_cfb(&ppt);
    for (p, d) in &mut streams {
        if p == "/PowerPoint Document" {
            d.truncate(40);
        }
    }
    let streams: Vec<_> = streams
        .into_iter()
        .map(|(p, d)| (p.trim_start_matches('/').to_owned(), d))
        .collect();
    let broken = build_cfb(&streams, &[]);
    assert!(matches!(
        extract_project(&broken, &limits),
        Err(Error::Package(_))
    ));
}

#[test]
fn powerpoint_decompression_is_bounded() {
    // The declared size is honest but over the limit.
    let limits = Limits {
        max_container_bytes: 1000,
        ..Limits::default()
    };
    let ppt = presentation(true, 1, 0xE391_C05F);
    assert!(extract_project(&ppt, &Limits::default()).is_ok());
    let err = extract_project(&ppt, &limits).unwrap_err();
    assert!(
        matches!(err, Error::InputTooLarge { .. } | Error::Package(_)),
        "{err:?}"
    );
}

#[test]
fn replacing_with_identical_bytes_reproduces_a_simple_package() {
    // For a package laid out the way the zip writer lays out entries (no
    // data descriptors, no gaps), replacing the project by its own bytes
    // yields the same file. Packages from other producers can differ in
    // layout while their entries stay identical (see the module docs).
    let pkg = docm();
    let out = replace_ooxml_project(&pkg, sample(), &Limits::default()).unwrap();
    assert_eq!(out, pkg);
}
