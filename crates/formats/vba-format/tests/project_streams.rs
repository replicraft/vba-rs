//! Reading and writing a project as a set of named streams, without a
//! container: the seam an embedding application uses.

mod common;

use common::*;
use proptest::prelude::*;
use vba_format::{
    Error, Limits, ProjectStreams, ReadMode, StaleCache, WriteOptions, keys, read_project,
    read_project_from_streams, write_project_streams, write_project_with_report,
};
use vba_project::{SysKind, VbaProject};

fn streams_of(bin: &[u8]) -> ProjectStreams {
    dump_cfb(bin).0.into_iter().collect()
}

fn read_set(s: &ProjectStreams) -> Result<VbaProject, Error> {
    read_project_from_streams(s, ReadMode::Strict, &Limits::default())
}

/// A copy of the sample whose module streams hold only a performance cache.
fn compiled_only_streams() -> ProjectStreams {
    let mut s = streams_of(&sample_project_bin());
    // Module1 and Class1 have caches of known length; strip the source.
    let m1 = s.get("VBA/Module1").unwrap()[..CACHE_MODULE1].to_vec();
    s.insert("VBA/Module1", m1);
    let c1 = s.get("VBA/Class1").unwrap()[..CACHE_CLASS1].to_vec();
    s.insert("VBA/Class1", c1);
    // ThisWorkbook, Sheet1 and UserForm1 have no cache: empty them.
    for n in ["ThisWorkbook", "Sheet1", "UserForm1"] {
        s.insert(format!("VBA/{n}"), Vec::new());
    }
    s
}

fn signed_streams() -> ProjectStreams {
    let mut s = streams_of(&sample_project_bin());
    s.insert("\u{5}DigitalSignature", vec![9; 40]);
    s
}

fn tolerant_streams() -> ProjectStreams {
    // The sample with a version word other than the no-cache marker.
    let mut s = streams_of(&sample_project_bin());
    let mut vp = s.get("VBA/_VBA_PROJECT").unwrap().to_vec();
    vp[2] = 0x97;
    vp[3] = 0x00;
    s.insert("VBA/_VBA_PROJECT", vp);
    s
}

fn fixtures() -> Vec<(&'static str, ProjectStreams)> {
    vec![
        ("sample", streams_of(&sample_project_bin())),
        ("compiled-only", compiled_only_streams()),
        ("signed", signed_streams()),
        ("versioned", tolerant_streams()),
    ]
}

fn container_free(mut p: VbaProject) -> VbaProject {
    p.preserved.remove(keys::CFB_VERSION);
    p.preserved.remove_prefixed(keys::STORAGE_PREFIX);
    p
}

fn assert_diff_matches_report(before: &ProjectStreams, after: &ProjectStreams, changed: &[String]) {
    let mut actual: Vec<String> = before
        .paths()
        .chain(after.paths())
        .filter(|p| before.get(p) != after.get(p))
        .map(str::to_owned)
        .collect();
    actual.sort();
    actual.dedup();
    assert_eq!(
        actual, changed,
        "report must list exactly the changed streams"
    );
}

fn edit(p: &mut VbaProject) {
    p.set_module_source("Module1", "Sub Changed()\r\nEnd Sub\r\n")
        .unwrap();
}

#[test]
fn stream_set_read_equals_compound_file_read_for_every_fixture() {
    for (name, set) in fixtures() {
        let cfb = build_cfb(
            &set.iter()
                .map(|(p, b)| (p.to_owned(), b.to_vec()))
                .collect::<Vec<_>>(),
            &[],
        );
        let via_cfb = container_free(read_project(&cfb, &Limits::default()).unwrap());
        let via_set = read_set(&set).unwrap();
        assert_eq!(via_cfb, via_set, "{name}");
    }
}

#[test]
fn stream_set_round_trip_equals_compound_file_round_trip() {
    for (name, set) in fixtures() {
        let storages = [StorageSpec {
            path: "VBA",
            clsid: [0; 16],
            state_bits: 0,
        }];
        let cfb = build_cfb(
            &set.iter()
                .map(|(p, b)| (p.to_owned(), b.to_vec()))
                .collect::<Vec<_>>(),
            &storages,
        );
        for do_edit in [false, true] {
            if do_edit && name == "compiled-only" {
                continue;
            }
            for stale in [StaleCache::Invalidate, StaleCache::Keep] {
                let opts = WriteOptions { stale_cache: stale };
                let mut a = read_project(&cfb, &Limits::default()).unwrap();
                let mut b = read_set(&set).unwrap();
                if do_edit {
                    edit(&mut a);
                    edit(&mut b);
                }
                let by_cfb = write_project_with_report(&a, &opts);
                let by_set = write_project_streams(&b, &opts);
                match (by_cfb, by_set) {
                    (Ok(c), Ok((s, report))) => {
                        assert_eq!(streams_of(&c.bytes), s, "{name} edit={do_edit} {stale:?}");
                        assert_eq!(c.report, report, "{name}");
                    }
                    (Err(Error::UnsupportedEdit(x)), Err(Error::UnsupportedEdit(y))) => {
                        assert_eq!(x, y, "{name}");
                    }
                    (x, y) => panic!("{name}: diverged: {:?} vs {:?}", x.is_ok(), y.is_ok()),
                }
            }
        }
    }
}

#[test]
fn untouched_project_writes_byte_identical_streams() {
    for (name, set) in fixtures() {
        let p = read_set(&set).unwrap();
        let (out, report) = write_project_streams(&p, &WriteOptions::default()).unwrap();
        assert_eq!(out, set, "{name}");
        assert!(report.changed_streams.is_empty(), "{name}");
    }
}

#[test]
fn an_edit_touches_only_the_streams_the_report_lists() {
    let set = streams_of(&sample_project_bin());
    let mut p = read_set(&set).unwrap();
    edit(&mut p);
    let (out, report) = write_project_streams(&p, &WriteOptions::default()).unwrap();
    assert_eq!(
        report.changed_streams,
        [
            "VBA/Module1",
            "VBA/_VBA_PROJECT",
            "VBA/__SRP_0",
            "VBA/__SRP_1",
            "VBA/dir"
        ]
    );
    assert_diff_matches_report(&set, &out, &report.changed_streams);
    // Dropped caches are gone from the set; the embedder must delete them.
    assert!(!out.contains("VBA/__SRP_0") && !out.contains("VBA/__SRP_1"));
    for name in ["VBA/Class1", "PROJECT", "PROJECTwm", "Unknown/blob", "XTRA"] {
        assert_eq!(out.get(name), set.get(name), "{name}");
    }
}

#[test]
fn keeping_caches_changes_only_the_module_stream() {
    let set = streams_of(&sample_project_bin());
    let mut p = read_set(&set).unwrap();
    edit(&mut p);
    let opts = WriteOptions {
        stale_cache: StaleCache::Keep,
    };
    let (out, report) = write_project_streams(&p, &opts).unwrap();
    assert_diff_matches_report(&set, &out, &report.changed_streams);
    assert_eq!(report.changed_streams, ["VBA/Module1"]);
}

#[test]
fn metadata_edit_changes_dir_and_project_only() {
    let set = streams_of(&sample_project_bin());
    let mut p = read_set(&set).unwrap();
    p.description = "Changed".into();
    let (out, report) = write_project_streams(&p, &WriteOptions::default()).unwrap();
    assert_diff_matches_report(&set, &out, &report.changed_streams);
    assert_eq!(report.changed_streams, ["PROJECT", "VBA/dir"]);
}

#[test]
fn signed_edits_report_invalidation() {
    let set = signed_streams();
    let mut p = read_set(&set).unwrap();
    let (_, clean) = write_project_streams(&p, &WriteOptions::default()).unwrap();
    assert!(!clean.signatures_invalidated);
    assert_eq!(clean.signature_streams, ["\u{5}DigitalSignature"]);
    edit(&mut p);
    let (out, report) = write_project_streams(&p, &WriteOptions::default()).unwrap();
    assert!(report.signatures_invalidated);
    assert_eq!(
        out.get("\u{5}DigitalSignature"),
        set.get("\u{5}DigitalSignature")
    );
}

#[test]
fn compiled_only_rules_apply_to_stream_sets() {
    let set = compiled_only_streams();
    let p = read_set(&set).unwrap();
    assert!(p.project_version().compiled_only);

    // Untouched: byte-identical.
    let (out, report) = write_project_streams(&p, &WriteOptions::default()).unwrap();
    assert_eq!(out, set);
    assert!(report.changed_streams.is_empty());

    // Supplying source for a stripped module is refused.
    let mut q = p.clone();
    let m = q.modules.iter_mut().find(|m| m.name == "Module1").unwrap();
    m.source = "Sub A()\r\nEnd Sub\r\n".into();
    m.has_source = true;
    let err = write_project_streams(&q, &WriteOptions::default()).unwrap_err();
    assert!(matches!(err, Error::UnsupportedEdit(_)), "{err}");
}

#[test]
fn lookup_ignores_case_and_spelling_is_written_back() {
    let set = streams_of(&sample_project_bin());
    let lower: ProjectStreams = set
        .iter()
        .map(|(p, b)| (p.to_ascii_lowercase(), b.to_vec()))
        .collect();
    let p = read_set(&lower).unwrap();
    assert_eq!(p.modules.len(), 5);
    let (out, report) = write_project_streams(&p, &WriteOptions::default()).unwrap();
    assert_eq!(out, lower);
    assert!(report.changed_streams.is_empty());

    let mut q = p;
    edit(&mut q);
    let (out, _) = write_project_streams(&q, &WriteOptions::default()).unwrap();
    assert!(out.entry("VBA/Module1").unwrap().0 == "vba/module1");
    assert!(out.entry("VBA/dir").unwrap().0 == "vba/dir");
}

#[test]
fn project_version_is_exposed_from_stream_sets() {
    let v = read_set(&tolerant_streams()).unwrap().project_version();
    assert_eq!(v.version_word, Some(0x0097));
    assert_eq!(v.sys_kind, SysKind::Win32);
    assert_eq!(v.profile_hint(), Some("vba7.0-excel-win32"));
    let v = read_set(&streams_of(&sample_project_bin()))
        .unwrap()
        .project_version();
    assert_eq!(v.version_word, Some(0xFFFF));
    assert_eq!(v.profile_hint(), None);
    assert!(!v.compiled_only);
    // The version word survives a rewrite of the module.
    let mut p = read_set(&tolerant_streams()).unwrap();
    edit(&mut p);
    let opts = WriteOptions {
        stale_cache: StaleCache::Keep,
    };
    let (out, _) = write_project_streams(&p, &opts).unwrap();
    assert_eq!(
        read_set(&out).unwrap().project_version().version_word,
        Some(0x0097)
    );
}

#[test]
fn fresh_projects_write_a_complete_readable_set() {
    let mut p = VbaProject::new("Fresh");
    p.add_module(vba_project::Module::new(
        "M",
        vba_project::ModuleKind::Standard,
        "Sub A()\r\nEnd Sub\r\n",
    ))
    .unwrap();
    let (set, report) = write_project_streams(&p, &WriteOptions::default()).unwrap();
    assert_eq!(
        set.paths().collect::<Vec<_>>(),
        ["PROJECT", "VBA/dir", "VBA/M", "VBA/_VBA_PROJECT"]
    );
    assert_eq!(report.changed_streams.len(), 4);
    let back = read_set(&set).unwrap();
    assert_eq!(back.module("M").unwrap().source, "Sub A()\r\nEnd Sub\r\n");
}

// ------------------------------------------------------- hostile input

#[test]
fn missing_streams_are_typed_errors() {
    let full = streams_of(&sample_project_bin());
    for missing in ["VBA/dir", "VBA/_VBA_PROJECT", "PROJECT", "VBA/Module1"] {
        let mut s = full.clone();
        s.remove(missing);
        match read_set(&s) {
            Err(Error::MissingStream { path }) => {
                assert!(path.eq_ignore_ascii_case(missing), "{path} vs {missing}");
            }
            other => panic!("{missing}: {other:?}"),
        }
    }
    assert!(matches!(
        read_set(&ProjectStreams::new()),
        Err(Error::MissingStream { .. })
    ));
}

#[test]
fn malformed_streams_are_typed_errors() {
    let full = streams_of(&sample_project_bin());
    let with = |path: &str, bytes: Vec<u8>| {
        let mut s = full.clone();
        s.insert(path, bytes);
        s
    };
    assert!(matches!(
        read_set(&with("VBA/_VBA_PROJECT", vec![0xCC, 0x61])),
        Err(Error::VbaProjectTooShort { len: 2 })
    ));
    assert!(matches!(
        read_set(&with("VBA/dir", vec![0xFF; 9])),
        Err(Error::Decompress { .. })
    ));
    assert!(matches!(
        read_set(&with("VBA/dir", Vec::new())),
        Err(Error::Decompress { .. })
    ));
    // A module stream shorter than its MODULEOFFSET.
    assert!(matches!(
        read_set(&with("VBA/Module1", vec![1, 2, 3])),
        Err(Error::ModuleOffsetOutOfRange { .. })
    ));
    // Corrupt compressed source.
    let mut bad = vec![0u8; CACHE_MODULE1];
    bad.extend([0x01, 0x05, 0xB0, 0xFF, 0xFF]);
    assert!(matches!(
        read_set(&with("VBA/Module1", bad)),
        Err(Error::Decompress { .. })
    ));
}

#[test]
fn limits_apply_to_stream_sets() {
    let set = streams_of(&sample_project_bin());
    let small = Limits {
        max_stream_bytes: 100,
        ..Limits::default()
    };
    assert!(matches!(
        read_project_from_streams(&set, ReadMode::Strict, &small),
        Err(Error::StreamTooLarge { .. })
    ));
    let few = Limits {
        max_entries: 3,
        ..Limits::default()
    };
    assert!(matches!(
        read_project_from_streams(&set, ReadMode::Strict, &few),
        Err(Error::TooManyEntries { limit: 3 })
    ));
    let total = Limits {
        max_container_bytes: 1000,
        ..Limits::default()
    };
    assert!(matches!(
        read_project_from_streams(&set, ReadMode::Strict, &total),
        Err(Error::InputTooLarge { .. })
    ));
    let bomb = Limits {
        max_decompressed_bytes: 4,
        ..Limits::default()
    };
    assert!(matches!(
        read_project_from_streams(&set, ReadMode::Strict, &bomb),
        Err(Error::Decompress { .. })
    ));
}

// ------------------------------------------------------------ properties

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Whatever source is set, the stream set reads back, the report matches
    /// the byte-level difference, and unlisted streams are untouched.
    #[test]
    fn edits_round_trip_and_reports_are_exact(
        body in "[ -~]{0,200}",
        keep in any::<bool>(),
        which in 0usize..2,
    ) {
        let set = streams_of(&sample_project_bin());
        let mut p = read_set(&set).unwrap();
        let name = ["Module1", "Class1"][which];
        let text = format!("Sub X()\r\n' {body}\r\nEnd Sub\r\n");
        p.set_module_source(name, &text).unwrap();
        let opts = WriteOptions {
            stale_cache: if keep { StaleCache::Keep } else { StaleCache::Invalidate },
        };
        let (out, report) = write_project_streams(&p, &opts).unwrap();
        assert_diff_matches_report(&set, &out, &report.changed_streams);
        let back = read_set(&out).unwrap();
        prop_assert_eq!(&back.module(name).unwrap().source, &text);
        // Idempotent: writing what was read changes nothing further.
        let (again, rep2) = write_project_streams(&back, &opts).unwrap();
        prop_assert_eq!(again, out);
        prop_assert!(rep2.changed_streams.is_empty());
    }

    /// Arbitrary corruption of any stream never panics.
    #[test]
    fn corrupted_stream_sets_never_panic(
        pick in any::<prop::sample::Index>(),
        edits in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..8),
        truncate in proptest::option::of(any::<prop::sample::Index>()),
        tolerant in any::<bool>(),
    ) {
        let set = streams_of(&sample_project_bin());
        let paths: Vec<String> = set.paths().map(str::to_owned).collect();
        let path = &paths[pick.index(paths.len())];
        let mut bytes = set.get(path).unwrap().to_vec();
        for (i, b) in edits {
            if !bytes.is_empty() {
                let at = i.index(bytes.len());
                bytes[at] = b;
            }
        }
        if let Some(t) = truncate {
            bytes.truncate(t.index(bytes.len() + 1));
        }
        let mut s = set.clone();
        s.insert(path, bytes);
        let mode = if tolerant { ReadMode::Tolerant } else { ReadMode::Strict };
        if let Ok(p) = read_project_from_streams(&s, mode, &Limits::default()) {
            let _ = write_project_streams(&p, &WriteOptions::default());
        }
    }
}
