//! Reading and writing whole `vbaProject.bin` storages.

mod common;

use common::*;
use std::collections::BTreeMap;
use vba_format::{Error, Limits, StaleCache, WriteOptions, keys, read_project, write_project};
use vba_project::{Module, ModuleKind, Reference, SysKind, VbaProject};

fn read(bytes: &[u8]) -> VbaProject {
    read_project(bytes, &Limits::default()).expect("fixture reads")
}

fn write(p: &VbaProject) -> Vec<u8> {
    write_project(p, &WriteOptions::default()).expect("project writes")
}

fn streams(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    dump_cfb(bytes).0.into_iter().collect()
}

#[test]
fn reads_project_information() {
    let p = read(&sample_project_bin());
    assert_eq!(p.name, "VBAProject");
    assert_eq!(p.description, "Example VBA Project Description");
    assert_eq!(p.code_page, 1252);
    assert_eq!(p.sys_kind, SysKind::Win32);
    assert_eq!((p.lcid, p.lcid_invoke), (0x409, 0x409));
    assert_eq!(
        p.help_file.as_deref(),
        Some("c:\\example path\\example.hlp")
    );
    assert_eq!(p.help_context, 1);
    assert_eq!(p.constants, "abc = 1");
    assert_eq!((p.version.major, p.version.minor), (0x49B5196B, 6));
    assert_eq!(
        p.project_id.as_deref(),
        Some("{917DED54-440B-4FD1-A5C1-74ACF261E600}")
    );
    assert!(!p.protection.is_locked());
    assert!(p.protection.visible);
    assert_eq!(p.host_extenders.len(), 1);
    assert_eq!(p.host_extenders[0].lib_name, "VBE");
    assert_eq!(p.workspace.len(), 3);
    assert_eq!(p.workspace[1].module, "Sheet1");
    assert_eq!(p.workspace[1].state, "69, 69, 724, 317, C");
}

#[test]
fn reads_references_of_every_kind() {
    let p = read(&sample_project_bin());
    let kinds: Vec<_> = p.references.iter().map(Reference::kind_name).collect();
    assert_eq!(kinds, ["registered", "registered", "project", "control"]);
    assert_eq!(p.references[0].name(), Some("stdole"));
    assert_eq!(p.references[3].name(), Some("MSForms"));
}

#[test]
fn reads_modules_with_kinds_flags_and_source() {
    let p = read(&sample_project_bin());
    let names: Vec<_> = p.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        ["ThisWorkbook", "Sheet1", "UserForm1", "Module1", "Class1"]
    );
    let kind = |n: &str| p.module(n).unwrap().kind;
    assert_eq!(kind("ThisWorkbook"), ModuleKind::Document);
    assert_eq!(kind("Sheet1"), ModuleKind::Document);
    assert_eq!(kind("UserForm1"), ModuleKind::Designer);
    assert_eq!(kind("Module1"), ModuleKind::Standard);
    assert_eq!(kind("Class1"), ModuleKind::Class);

    assert_eq!(p.module("Module1").unwrap().source, SRC_MODULE1);
    assert_eq!(p.module("Class1").unwrap().source, SRC_CLASS1);
    assert_eq!(p.module("Sheet1").unwrap().source, SRC_SHEET1);
    assert_eq!(
        p.module("Sheet1").unwrap().doc_string,
        "Example Module Description"
    );
    assert_eq!(p.module("Sheet1").unwrap().help_context, 2);
    assert!(p.module("UserForm1").unwrap().private);
    assert!(p.module("Class1").unwrap().read_only);
    assert!(p.modules.iter().all(|m| m.has_source));
}

#[test]
fn splits_the_performance_cache_at_moduleoffset() {
    let p = read(&sample_project_bin());
    let m = p.module("Module1").unwrap();
    assert_eq!(
        m.preserved.get(keys::MODULE_PERFORMANCE_CACHE).unwrap(),
        &filler(CACHE_MODULE1, 9)[..]
    );
    assert_eq!(
        m.preserved.get(keys::MODULE_COMPRESSED_SOURCE).unwrap(),
        &vba_format::compress(SRC_MODULE1.as_bytes())[..]
    );
    let m = p.module("ThisWorkbook").unwrap();
    assert!(
        m.preserved
            .get(keys::MODULE_PERFORMANCE_CACHE)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn unknown_data_is_preserved_in_the_model() {
    let p = read(&sample_project_bin());
    let pres = &p.preserved;
    assert_eq!(pres.get("ovba:stream:XTRA"), Some(&filler(10, 8)[..]));
    assert_eq!(pres.get("ovba:stream:Unknown/blob").unwrap().len(), 5000);
    assert!(pres.contains("ovba:stream:UserForm1/f"));
    assert!(pres.contains("ovba:stream:PROJECTwm"));
    assert!(pres.contains("ovba:stream:VBA/__SRP_0"));
    // Interpreted streams are not duplicated as generic streams.
    assert!(!pres.contains("ovba:stream:PROJECT"));
    assert!(!pres.contains("ovba:stream:VBA/dir"));
    assert!(!pres.contains("ovba:stream:VBA/Module1"));
    // Storage metadata.
    let unknown = pres.get("ovba:storage:Unknown").unwrap();
    assert_eq!(
        &unknown[..16],
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    );
    assert_eq!(&unknown[16..], &0x55u32.to_le_bytes());
}

#[test]
fn untouched_project_round_trips_every_stream_exactly() {
    let original = sample_project_bin();
    let rewritten = write(&read(&original));
    assert_eq!(dump_cfb(&rewritten), dump_cfb(&original));
}

#[test]
fn read_write_read_is_stable() {
    let first = read(&sample_project_bin());
    let second = read(&write(&first));
    assert_eq!(first, second);
    // And writing again changes nothing.
    assert_eq!(dump_cfb(&write(&second)), dump_cfb(&write(&first)));
}

#[test]
fn replacing_one_module_rewrites_only_what_it_must() {
    let original = sample_project_bin();
    let mut p = read(&original);
    let new_src = "Attribute VB_Name = \"Module1\"\r\nSub Changed()\r\nEnd Sub\r\n";
    p.set_module_source("Module1", new_src).unwrap();
    let out = write(&p);

    let (before, after) = (streams(&original), streams(&out));
    let changed: Vec<_> = before
        .keys()
        .chain(after.keys())
        .filter(|k| before.get(*k) != after.get(*k))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .cloned()
        .collect();
    // The module stream, the dir stream (its MODULEOFFSET), the version
    // stream whose cache is now invalid, and the dropped SRP caches.
    assert_eq!(
        changed,
        [
            "/VBA/Module1",
            "/VBA/_VBA_PROJECT",
            "/VBA/__SRP_0",
            "/VBA/__SRP_1",
            "/VBA/dir"
        ]
    );
    assert_eq!(
        after["/VBA/_VBA_PROJECT"],
        [0xCC, 0x61, 0xFF, 0xFF, 0x00, 0x01, 0x00]
    );
    assert_eq!(
        after["/VBA/Module1"],
        vba_format::compress(new_src.as_bytes())
    );
    // Untouched module streams keep their own caches.
    assert_eq!(before["/VBA/Class1"], after["/VBA/Class1"]);

    let back = read(&out);
    assert_eq!(back.module("Module1").unwrap().source, new_src);
    assert_eq!(back.module("Class1").unwrap().source, SRC_CLASS1);
    assert_eq!(back.references, read(&original).references);
    assert!(
        back.module("Module1")
            .unwrap()
            .preserved
            .get(keys::MODULE_PERFORMANCE_CACHE)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn keeping_caches_changes_only_the_edited_module_stream() {
    let original = sample_project_bin();
    let mut p = read(&original);
    p.set_module_source("Module1", "Attribute VB_Name = \"Module1\"\r\n")
        .unwrap();
    let out = write_project(
        &p,
        &WriteOptions {
            stale_cache: StaleCache::Keep,
        },
    )
    .unwrap();
    let (before, after) = (streams(&original), streams(&out));
    for (k, v) in &before {
        if k == "/VBA/Module1" {
            assert_ne!(v, &after[k]);
            assert_eq!(&after[k][..CACHE_MODULE1], &v[..CACHE_MODULE1]);
        } else {
            assert_eq!(v, &after[k], "{k}");
        }
    }
    assert_eq!(before.len(), after.len());
}

#[test]
fn setting_identical_source_is_not_an_edit() {
    let original = sample_project_bin();
    let mut p = read(&original);
    let same = p.module("Module1").unwrap().source.clone();
    p.set_module_source("Module1", same).unwrap();
    assert_eq!(dump_cfb(&write(&p)), dump_cfb(&original));
}

#[test]
fn metadata_edits_patch_the_project_stream_and_rewrite_dir() {
    let original = sample_project_bin();
    let mut p = read(&original);
    p.name = "Renamed".into();
    p.description = "New description".into();
    p.module_mut("Sheet1").unwrap().doc_string = "Changed".into();
    let out = write(&p);
    let (before, after) = (streams(&original), streams(&out));
    let differing: Vec<_> = before.keys().filter(|k| before[*k] != after[*k]).collect();
    assert_eq!(differing, ["/PROJECT", "/VBA/dir"]);
    let project_text = String::from_utf8(after["/PROJECT"].clone()).unwrap();
    assert!(project_text.contains("Name=\"Renamed\"\r\n"));
    assert!(project_text.contains("Description=\"New description\"\r\n"));
    let expected = String::from_utf8(before["/PROJECT"].clone())
        .unwrap()
        .replace("Name=\"VBAProject\"", "Name=\"Renamed\"")
        .replace("Example VBA Project Description", "New description");
    assert_eq!(project_text, expected);
    let back = read(&out);
    assert_eq!(back.name, "Renamed");
    assert_eq!(back.description, "New description");
    assert_eq!(back.module("Sheet1").unwrap().doc_string, "Changed");
    // Module caches are not touched by metadata edits.
    assert_eq!(before["/VBA/Module1"], after["/VBA/Module1"]);
    assert_eq!(before["/VBA/_VBA_PROJECT"], after["/VBA/_VBA_PROJECT"]);
}

#[test]
fn reference_edits_are_written() {
    let mut p = read(&sample_project_bin());
    p.references.remove(1);
    p.references.push(Reference::Registered {
        name: Some("Extra".into()),
        libid: "*\\G{00000000-0000-0000-0000-000000000001}#1.0#0#C:\\x.tlb#Extra".into(),
    });
    let back = read(&write(&p));
    assert_eq!(back.references, p.references);
}

#[test]
fn structural_edits_are_rejected() {
    let base = read(&sample_project_bin());

    let mut p = base.clone();
    p.modules[3].name = "Other".into();
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));

    let mut p = base.clone();
    p.add_module(Module::new("Extra", ModuleKind::Standard, ""))
        .unwrap();
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));

    let mut p = base.clone();
    p.remove_module("Class1").unwrap();
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));

    let mut p = base.clone();
    p.modules[3].kind = ModuleKind::Class;
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));

    let mut p = base.clone();
    p.protection.has_password = true;
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));

    let mut p = base.clone();
    p.workspace.clear();
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));

    let mut p = base;
    p.modules[3].has_source = false;
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));
}

#[test]
fn source_that_the_code_page_cannot_hold_is_an_error() {
    let mut p = read(&sample_project_bin());
    p.set_module_source("Module1", "' 日本語\r\n").unwrap();
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::Unencodable {
            code_page: 1252,
            ..
        })
    ));
}

#[test]
fn non_ascii_source_survives_in_the_project_code_page() {
    let mut p = read(&sample_project_bin());
    let src = "' Größe € ünï\r\n";
    p.set_module_source("Module1", src).unwrap();
    let back = read(&write(&p));
    assert_eq!(back.module("Module1").unwrap().source, src);
}

#[test]
fn bytes_that_do_not_decode_cleanly_are_kept_when_untouched() {
    // A comment containing a lone high byte invalid in UTF-8 but valid in
    // Windows-1252, plus a project in code page 65001 where it is invalid.
    let mut modules = spec_modules();
    modules.truncate(1);
    let dir = vba_format::compress(&dir_stream(&spec_references(), &modules));
    let bad = b"' bad \xFF byte\r\n";
    let streams = vec![
        ("VBA/dir".to_owned(), dir),
        (
            "VBA/_VBA_PROJECT".to_owned(),
            vec![0xCC, 0x61, 0xFF, 0xFF, 0, 1, 0],
        ),
        ("VBA/ThisWorkbook".to_owned(), module_stream(&[], bad)),
        ("PROJECT".to_owned(), project_stream("")),
    ];
    let bin = build_cfb(&streams, &[]);
    let p = read(&bin);
    assert!(p.module("ThisWorkbook").unwrap().source.contains('\u{FF}'));
    // Not edited, so the original bytes come back unchanged.
    assert_eq!(
        streams_of(&write(&p))["/VBA/ThisWorkbook"],
        streams_of(&bin)["/VBA/ThisWorkbook"]
    );
}

fn streams_of(b: &[u8]) -> BTreeMap<String, Vec<u8>> {
    streams(b)
}

#[test]
fn project_without_source_is_reported_and_not_editable() {
    let mut modules = spec_modules();
    modules.truncate(2);
    modules[1].offset = 16;
    let dir = vba_format::compress(&dir_stream(&spec_references(), &modules));
    let streams = vec![
        ("VBA/dir".to_owned(), dir),
        (
            "VBA/_VBA_PROJECT".to_owned(),
            vec![0xCC, 0x61, 0xFF, 0xFF, 0, 1, 0, 9, 9],
        ),
        ("VBA/ThisWorkbook".to_owned(), Vec::new()),
        // Only a performance cache, no compressed source after it.
        ("VBA/Sheet1".to_owned(), filler(16, 4)),
        ("PROJECT".to_owned(), project_stream("")),
    ];
    let bin = build_cfb(&streams, &[]);
    let mut p = read(&bin);
    assert!(
        p.modules
            .iter()
            .all(|m| !m.has_source && m.source.is_empty())
    );
    // Untouched: identical.
    assert_eq!(dump_cfb(&write(&p)), dump_cfb(&bin));
    // Source cannot be supplied for a module whose stored source is absent.
    let m = p.module_mut("Sheet1").unwrap();
    m.source = "Sub A()\r\nEnd Sub\r\n".into();
    m.has_source = true;
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(m)) if m.contains("Sheet1")
    ));
}

#[test]
fn protection_state_is_detected() {
    let id = "{917DED54-440B-4FD1-A5C1-74ACF261E600}";
    let hex = |b: Vec<u8>| vba_format::crypt::hex_encode(&b);
    let cmg = hex(vba_format::crypt::encrypt(&5u32.to_le_bytes(), 0x07, id));
    let dpb = hex(vba_format::crypt::encrypt(&[0x11; 29], 0x05, id));
    let gc = hex(vba_format::crypt::encrypt(&[0], 0x07, id));
    let text = String::from_utf8(project_stream("")).unwrap();
    let text = text
        .replace("0705D8E3D8EDDBF1DBF1DBF1DBF1", &cmg)
        .replace("0E0CD1ECDFF4E7F5E7F5E7", &dpb)
        .replace("1517CAF1D6F9D7F9D706", &gc);
    let mut modules = spec_modules();
    modules.truncate(1);
    let streams = vec![
        (
            "VBA/dir".to_owned(),
            vba_format::compress(&dir_stream(&spec_references(), &modules)),
        ),
        (
            "VBA/_VBA_PROJECT".to_owned(),
            vec![0xCC, 0x61, 0xFF, 0xFF, 0, 1, 0],
        ),
        (
            "VBA/ThisWorkbook".to_owned(),
            module_stream(&[], b"Option Explicit\r\n"),
        ),
        ("PROJECT".to_owned(), text.into_bytes()),
    ];
    let p = read(&build_cfb(&streams, &[]));
    assert!(p.protection.user_protected);
    assert!(!p.protection.host_protected);
    assert!(p.protection.vbe_protected);
    assert!(p.protection.has_password);
    assert!(!p.protection.visible);
    assert!(p.protection.is_locked());
    // Source is still readable: the flags are declarations, not encryption.
    assert_eq!(
        p.module("ThisWorkbook").unwrap().source,
        "Option Explicit\r\n"
    );
}

// ----------------------------------------------------------- new projects

fn fresh() -> VbaProject {
    let mut p = VbaProject::new("Fresh");
    p.description = "A fresh project".into();
    p.constants = "DEBUG_MODE = 1".into();
    p.help_context = 5;
    p.references.push(Reference::Registered {
        name: Some("stdole".into()),
        libid: "*\\G{00020430-0000-0000-C000-000000000046}#2.0#0#C:\\Windows\\system32\\stdole2.tlb#OLE Automation".into(),
    });
    p.add_module(Module::new(
        "ThisDocument",
        ModuleKind::Document,
        "Option Explicit\r\n",
    ))
    .unwrap();
    p.add_module(Module::new("Util", ModuleKind::Standard, SRC_MODULE1))
        .unwrap();
    let mut c = Module::new("Thing", ModuleKind::Class, SRC_CLASS1);
    c.doc_string = "A thing".into();
    c.private = true;
    p.add_module(c).unwrap();
    p
}

#[test]
fn new_project_round_trips_through_bytes() {
    let p = fresh();
    let back = read(&write(&p));
    assert_eq!(back.name, "Fresh");
    assert_eq!(back.description, p.description);
    assert_eq!(back.constants, p.constants);
    assert_eq!(back.help_context, 5);
    assert_eq!(back.references, p.references);
    assert_eq!(back.code_page, 1252);
    assert_eq!(back.modules.len(), 3);
    for (a, b) in p.modules.iter().zip(&back.modules) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.source, b.source);
        assert_eq!(a.doc_string, b.doc_string);
        assert_eq!(a.private, b.private);
    }
    assert!(!back.protection.is_locked());
    // Writing the re-read project reproduces the same streams.
    assert_eq!(dump_cfb(&write(&back)), dump_cfb(&write(&p)));
}

#[test]
fn new_project_output_is_deterministic() {
    assert_eq!(write(&fresh()), write(&fresh()));
}

#[test]
fn new_project_can_use_another_code_page() {
    let mut p = fresh();
    p.code_page = 932;
    p.set_module_source("Util", "' 日本語のコメント\r\nSub A()\r\nEnd Sub\r\n")
        .unwrap();
    p.description = "説明".into();
    let back = read(&write(&p));
    assert_eq!(back.code_page, 932);
    assert_eq!(
        back.module("Util").unwrap().source,
        p.module("Util").unwrap().source
    );
    assert_eq!(back.description, "説明");
}

#[test]
fn new_project_rejects_what_it_cannot_write() {
    let mut p = fresh();
    p.add_module(Module::new("Form", ModuleKind::Designer, ""))
        .unwrap();
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));

    let mut p = fresh();
    p.modules
        .push(Module::new("util", ModuleKind::Standard, ""));
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));

    let mut p = fresh();
    p.modules[0].name = "a/b".into();
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));

    let mut p = fresh();
    p.protection.has_password = true;
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedEdit(_))
    ));

    let mut p = fresh();
    p.code_page = 10006;
    assert!(matches!(
        write_project(&p, &WriteOptions::default()),
        Err(Error::UnsupportedCodePage(10006))
    ));
}

#[test]
fn new_project_with_preserved_streams_keeps_them() {
    let mut p = fresh();
    p.preserved.insert("ovba:stream:Custom/data", vec![1, 2, 3]);
    let back = read(&write(&p));
    assert_eq!(
        back.preserved.get("ovba:stream:Custom/data"),
        Some(&[1, 2, 3][..])
    );
}

#[test]
fn new_project_can_be_edited_after_reading() {
    let back = read(&write(&fresh()));
    let mut p = back;
    p.set_module_source("Util", "Option Explicit\r\n").unwrap();
    let again = read(&write(&p));
    assert_eq!(again.module("Util").unwrap().source, "Option Explicit\r\n");
    assert_eq!(again.module("Thing").unwrap().source, SRC_CLASS1);
}

// -------------------------------------------------------- hostile input

#[test]
fn rejects_non_compound_files() {
    for input in [
        &b""[..],
        b"hello",
        &[0u8; 512][..],
        &[0xD0, 0xCF, 0x11, 0xE0][..],
    ] {
        assert!(matches!(
            read_project(input, &Limits::default()),
            Err(Error::Container(_))
        ));
    }
}

#[test]
fn reports_missing_required_streams() {
    let full: Vec<(String, Vec<u8>)> = {
        let mut m = spec_modules();
        m.truncate(1);
        vec![
            (
                "VBA/dir".into(),
                vba_format::compress(&dir_stream(&spec_references(), &m)),
            ),
            (
                "VBA/_VBA_PROJECT".into(),
                vec![0xCC, 0x61, 0xFF, 0xFF, 0, 1, 0],
            ),
            ("VBA/ThisWorkbook".into(), module_stream(&[], b"x")),
            ("PROJECT".into(), project_stream("")),
        ]
    };
    for missing in ["VBA/dir", "VBA/_VBA_PROJECT", "VBA/ThisWorkbook", "PROJECT"] {
        let streams: Vec<_> = full.iter().filter(|(p, _)| p != missing).cloned().collect();
        let err = read_project(&build_cfb(&streams, &[]), &Limits::default()).unwrap_err();
        assert!(
            matches!(err, Error::MissingStream { .. }),
            "{missing}: {err}"
        );
    }
    let err = read_project(
        &build_cfb(&[("PROJECT".into(), vec![1])], &[]),
        &Limits::default(),
    )
    .unwrap_err();
    assert!(matches!(err, Error::MissingStream { .. }));
}

#[test]
fn rejects_bad_streams() {
    let mut m = spec_modules();
    m.truncate(1);
    let good_dir = vba_format::compress(&dir_stream(&spec_references(), &m));
    let base = |dir: Vec<u8>, vp: Vec<u8>, module: Vec<u8>| {
        build_cfb(
            &[
                ("VBA/dir".to_owned(), dir),
                ("VBA/_VBA_PROJECT".to_owned(), vp),
                ("VBA/ThisWorkbook".to_owned(), module),
                ("PROJECT".to_owned(), project_stream("")),
            ],
            &[],
        )
    };
    let vp = vec![0xCC, 0x61, 0xFF, 0xFF, 0, 1, 0];
    let ok = module_stream(&[], b"x");

    // dir that is not a compressed container
    let e = read_project(
        &base(vec![1, 2, 3], vp.clone(), ok.clone()),
        &Limits::default(),
    )
    .unwrap_err();
    assert!(matches!(e, Error::Decompress { .. }), "{e}");
    // dir that is compressed garbage records
    let e = read_project(
        &base(vba_format::compress(&[1, 2, 3, 4]), vp.clone(), ok.clone()),
        &Limits::default(),
    )
    .unwrap_err();
    assert!(
        matches!(
            e,
            Error::DirUnexpectedRecord { .. } | Error::DirTruncated { .. }
        ),
        "{e}"
    );
    // _VBA_PROJECT too short
    let e = read_project(
        &base(good_dir.clone(), vec![1, 2, 3], ok.clone()),
        &Limits::default(),
    )
    .unwrap_err();
    assert!(matches!(e, Error::VbaProjectTooShort { len: 3 }), "{e}");
    // module source that is not a compressed container
    let e = read_project(
        &base(good_dir.clone(), vp.clone(), vec![0x55, 1, 2]),
        &Limits::default(),
    )
    .unwrap_err();
    assert!(matches!(e, Error::Decompress { .. }), "{e}");

    // MODULEOFFSET beyond the stream
    let mut m2 = spec_modules();
    m2.truncate(1);
    m2[0].offset = 1000;
    let dir = vba_format::compress(&dir_stream(&spec_references(), &m2));
    let e = read_project(&base(dir, vp, ok), &Limits::default()).unwrap_err();
    assert!(
        matches!(e, Error::ModuleOffsetOutOfRange { offset: 1000, .. }),
        "{e}"
    );
}

#[test]
fn enforces_resource_limits() {
    let bin = sample_project_bin();
    let tight = |f: &dyn Fn(&mut Limits)| {
        let mut l = Limits::default();
        f(&mut l);
        read_project(&bin, &l).unwrap_err()
    };
    assert!(matches!(
        tight(&|l| l.max_container_bytes = 100),
        Error::InputTooLarge { .. }
    ));
    assert!(matches!(
        tight(&|l| l.max_entries = 3),
        Error::TooManyEntries { .. }
    ));
    assert!(matches!(
        tight(&|l| l.max_stream_bytes = 1000),
        Error::StreamTooLarge { .. }
    ));
    assert!(matches!(
        tight(&|l| l.max_decompressed_bytes = 50),
        Error::Decompress { .. }
    ));
    assert!(matches!(
        tight(&|l| l.max_compression_ratio = 1),
        Error::Decompress { .. }
    ));
}

#[test]
fn decompression_bombs_are_stopped_by_the_ratio_limit() {
    // 1.2 MB of 'a' compresses to about 2 KB (ratio ~585): valid data that a
    // stricter ratio limit still rejects.
    let bomb = vba_format::compress(&vec![b'a'; 300 * 4096]);
    assert!(bomb.len() < 3000);
    let mut m = spec_modules();
    m.truncate(1);
    let streams = vec![
        (
            "VBA/dir".to_owned(),
            vba_format::compress(&dir_stream(&spec_references(), &m)),
        ),
        (
            "VBA/_VBA_PROJECT".to_owned(),
            vec![0xCC, 0x61, 0xFF, 0xFF, 0, 1, 0],
        ),
        ("VBA/ThisWorkbook".to_owned(), bomb),
        ("PROJECT".to_owned(), project_stream("")),
    ];
    let bin = build_cfb(&streams, &[]);
    assert!(read_project(&bin, &Limits::default()).is_ok());
    let strict = Limits {
        max_compression_ratio: 100,
        ..Limits::default()
    };
    assert!(matches!(
        read_project(&bin, &strict),
        Err(Error::Decompress { .. })
    ));
}

#[test]
fn truncated_files_never_panic() {
    let bin = sample_project_bin();
    let mut len = 0;
    while len < bin.len() {
        let _ = read_project(&bin[..len], &Limits::default());
        len += if len < 2048 { 7 } else { 211 };
    }
}

#[test]
fn corrupted_files_never_panic() {
    let bin = sample_project_bin();
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    for _ in 0..1500 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut m = bin.clone();
        for k in 0..=(seed % 4) {
            let pos = ((seed >> (9 * k)) as usize) % m.len();
            m[pos] ^= (seed >> 33) as u8 | 1;
        }
        if let Ok(p) = read_project(&m, &Limits::default()) {
            // Anything that reads must also write without panicking.
            let _ = write_project(&p, &WriteOptions::default());
        }
    }
}

#[test]
fn corrupted_stream_contents_never_panic() {
    // Corrupt inside the streams (not the container structure), so the
    // OVBA-level parsing is what gets exercised.
    let (streams_in, storages) = dump_cfb(&sample_project_bin());
    let mut seed = 0xDEAD_BEEF_CAFE_F00Du64;
    for _ in 0..800 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut streams: Vec<(String, Vec<u8>)> = streams_in
            .iter()
            .map(|(p, d)| (p.trim_start_matches('/').to_owned(), d.clone()))
            .collect();
        let which = (seed as usize) % 5;
        let name = [
            "VBA/dir",
            "PROJECT",
            "VBA/Module1",
            "VBA/_VBA_PROJECT",
            "VBA/Sheet1",
        ][which];
        let s = streams.iter_mut().find(|(p, _)| p == name).unwrap();
        for k in 0..=(seed % 3) {
            let pos = ((seed >> (11 * k)) as usize) % s.1.len();
            s.1[pos] = (seed >> 40) as u8;
        }
        let specs: Vec<StorageSpec> = storages
            .iter()
            .filter(|(p, _, _)| p != "/")
            .map(|(p, c, b)| StorageSpec {
                path: Box::leak(p.trim_start_matches('/').to_owned().into_boxed_str()),
                clsid: *c,
                state_bits: *b,
            })
            .collect();
        let bin = build_cfb(&streams, &specs);
        if let Ok(p) = read_project(&bin, &Limits::default()) {
            let _ = write_project(&p, &WriteOptions::default());
        }
    }
}
