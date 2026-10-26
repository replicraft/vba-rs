//! Compiled-only (stripped-source) projects: classification, byte-exact
//! preservation through every writer path, and refusal to invent source.
//!
//! The fixtures imitate a stripped project: every module stream holds only a
//! performance cache (`MODULEOFFSET` equals the stream length), no compressed
//! source container follows it, and `_VBA_PROJECT` carries a real version and
//! cache instead of the `0xFFFF` "no cache" marker.

mod common;

use common::packages::package;
use common::*;
use proptest::prelude::*;
use std::collections::BTreeMap;
use vba_format::{
    Error, Limits, ReadMode, StaleCache, WriteOptions, extract_project, read_project,
    read_project_with, replace_ooxml_project, write_project, write_project_with_report,
};
use vba_project::{ModelError, ProjectSourceAvailability, SourceAvailability, VbaProject};

/// What a fixture module stream holds.
#[derive(Clone, Debug)]
enum Body {
    /// Cache of this many bytes, no source container.
    CompiledOnly(usize),
    /// Neither cache nor source: an empty stream.
    Nothing,
    /// Cache and a container holding the standard sample source.
    Source(usize),
}

const SRC: &str = "Sub A()\r\nEnd Sub\r\n";

/// Builds a project with the three specification modules.
fn project_bin(bodies: [Body; 3], source: &str, vba_project_cache: &[u8]) -> Vec<u8> {
    let mut modules = spec_modules();
    let mut streams = Vec::new();
    for (i, (m, body)) in modules.iter_mut().zip(&bodies).enumerate() {
        let (cache, has_src) = match body {
            Body::CompiledOnly(n) => (filler(*n, i as u8), false),
            Body::Nothing => (Vec::new(), false),
            Body::Source(n) => (filler(*n, i as u8), true),
        };
        m.offset = cache.len() as u32;
        let stream = if has_src {
            module_stream(&cache, source.as_bytes())
        } else {
            cache
        };
        streams.push((format!("VBA/{}", m.name), stream));
    }
    let dir = vba_format::compress(&dir_stream(&spec_references(), &modules));
    // Version 0x0069, not the 0xFFFF "no cache" marker.
    let mut vp = vec![0xCC, 0x61, 0x69, 0x00, 0x00, 0x01, 0x00];
    vp.extend(vba_project_cache);
    streams.push(("VBA/dir".to_owned(), dir));
    streams.push(("VBA/_VBA_PROJECT".to_owned(), vp));
    streams.push(("PROJECT".to_owned(), project_stream("")));
    streams.push(("PROJECTwm".to_owned(), b"opaque\0".to_vec()));
    build_cfb(&streams, &[])
}

fn all_compiled() -> Vec<u8> {
    project_bin(
        [
            Body::CompiledOnly(40),
            Body::CompiledOnly(64),
            Body::CompiledOnly(7),
        ],
        SRC,
        &filler(33, 9),
    )
}

fn mixed() -> Vec<u8> {
    project_bin(
        [
            Body::Source(12),
            Body::CompiledOnly(64),
            Body::CompiledOnly(7),
        ],
        SRC,
        &filler(33, 9),
    )
}

fn read(b: &[u8]) -> VbaProject {
    read_project(b, &Limits::default()).expect("fixture reads")
}

fn classes(p: &VbaProject) -> Vec<SourceAvailability> {
    p.modules.iter().map(|m| m.source_availability()).collect()
}

fn streams(b: &[u8]) -> BTreeMap<String, Vec<u8>> {
    dump_cfb(b).0.into_iter().collect()
}

#[test]
fn classification_is_derived_from_offset_and_container() {
    use SourceAvailability::*;
    let p = read(&all_compiled());
    assert_eq!(classes(&p), [CompiledOnly; 3]);
    assert_eq!(
        p.source_availability(),
        ProjectSourceAvailability::CompiledOnly
    );
    assert_eq!(p.modules[0].performance_cache_len(), 40);
    assert!(p.modules.iter().all(|m| !m.source_is_ambiguous()));

    let p = read(&mixed());
    assert_eq!(classes(&p), [Present, CompiledOnly, CompiledOnly]);
    assert_eq!(p.source_availability(), ProjectSourceAvailability::Mixed);
    let names: Vec<_> = p.modules_without_source().map(|m| &*m.name).collect();
    assert_eq!(names, ["Sheet1", "UserForm1"]);

    let p = read(&project_bin(
        [Body::Source(0), Body::Source(5), Body::Source(0)],
        SRC,
        &[],
    ));
    assert_eq!(
        p.source_availability(),
        ProjectSourceAvailability::FullSource
    );
}

#[test]
fn empty_module_with_source_is_not_compiled_only() {
    use SourceAvailability::*;
    // An empty class module still stores a container.
    let p = read(&project_bin(
        [Body::Source(0), Body::Source(9), Body::Nothing],
        "",
        &[],
    ));
    assert_eq!(classes(&p), [Empty, Empty, AbsentNoCache]);
    // Not ambiguous: an empty container and no cache.
    assert!(!p.modules[0].source_is_ambiguous());
    // Ambiguous: empty container with a cache, and nothing at all.
    assert!(p.modules[1].source_is_ambiguous());
    assert!(p.modules[2].source_is_ambiguous());
    // Only the module without any container counts as lacking source.
    assert_eq!(p.source_availability(), ProjectSourceAvailability::Mixed);
    let lacking: Vec<_> = p.modules_without_source().map(|m| &*m.name).collect();
    assert_eq!(lacking, ["UserForm1"]);
}

#[test]
fn every_writer_path_preserves_a_compiled_only_project() {
    for bin in [all_compiled(), mixed()] {
        let before = dump_cfb(&bin);
        let p = read(&bin);
        for stale in [StaleCache::Invalidate, StaleCache::Keep] {
            let o = WriteOptions { stale_cache: stale };
            assert_eq!(dump_cfb(&write_project(&p, &o).unwrap()), before);
            let out = write_project_with_report(&p, &o).unwrap();
            assert_eq!(dump_cfb(&out.bytes), before);
            assert!(out.report.changed_streams.is_empty());
        }
        // Tolerant mode.
        let t = read_project_with(&bin, &Limits::default(), ReadMode::Tolerant).unwrap();
        assert_eq!(t, p);
        assert_eq!(
            dump_cfb(&write_project(&t, &WriteOptions::default()).unwrap()),
            before
        );

        // OOXML replacement.
        let pkg = package(
            "word/document.xml",
            "vbaProject.bin",
            "word/vbaProject.bin",
            &bin,
            false,
        );
        let rewritten = write_project(&p, &WriteOptions::default()).unwrap();
        let out = replace_ooxml_project(&pkg, &rewritten, &Limits::default()).unwrap();
        let extracted = extract_project(&out, &Limits::default()).unwrap();
        assert_eq!(dump_cfb(&extracted.bytes), before);
        assert_eq!(read(&extracted.bytes), p);
    }
}

#[test]
fn version_fields_and_vba_project_stream_are_untouched() {
    let bin = all_compiled();
    let p = read(&bin);
    let orig = streams(&bin);
    let out = streams(&write_project(&p, &WriteOptions::default()).unwrap());
    for k in ["/VBA/_VBA_PROJECT", "/VBA/dir", "/PROJECT", "/PROJECTwm"] {
        assert_eq!(out[k], orig[k], "{k}");
    }
    assert_eq!(&out["/VBA/_VBA_PROJECT"][2..4], [0x69, 0x00]);
    // Project metadata edits rewrite `dir` but leave versions and caches.
    let mut q = p.clone();
    q.description = "changed".into();
    let bytes = write_project(&q, &WriteOptions::default()).unwrap();
    assert_eq!(read(&bytes).version, p.version);
    let after = streams(&bytes);
    assert_eq!(after["/VBA/_VBA_PROJECT"], orig["/VBA/_VBA_PROJECT"]);
    for m in ["ThisWorkbook", "Sheet1", "UserForm1"] {
        let k = format!("/VBA/{m}");
        assert_eq!(after[&k], orig[&k]);
    }
}

fn unsupported(r: Result<Vec<u8>, Error>, module: &str) {
    match r {
        Err(Error::UnsupportedEdit(m)) => {
            assert!(m.contains(&format!("{module:?}")), "{m}");
        }
        other => panic!("expected UnsupportedEdit naming {module}, got {other:?}"),
    }
}

#[test]
fn source_cannot_be_invented_for_an_absent_module() {
    let p = read(&all_compiled());
    // The model-level setter refuses.
    let mut q = p.clone();
    assert_eq!(
        q.set_module_source("Sheet1", SRC),
        Err(ModelError::CompiledOnlyModule("Sheet1".into()))
    );
    // Setting the fields directly is refused by the writer.
    for stale in [StaleCache::Invalidate, StaleCache::Keep] {
        let o = WriteOptions { stale_cache: stale };
        let mut q = p.clone();
        let m = q.module_mut("Sheet1").unwrap();
        m.source = SRC.into();
        m.has_source = true;
        unsupported(write_project(&q, &o), "Sheet1");
        // Text without the flag is not silently dropped either.
        let mut q = p.clone();
        q.module_mut("UserForm1").unwrap().source = "x".into();
        unsupported(write_project(&q, &o), "UserForm1");
        // An empty source is still an invented container.
        let mut q = p.clone();
        q.module_mut("ThisWorkbook").unwrap().has_source = true;
        unsupported(write_project(&q, &o), "ThisWorkbook");
    }
}

#[test]
fn a_module_without_cache_and_source_cannot_gain_source_either() {
    let bin = project_bin([Body::Nothing, Body::Source(0), Body::Nothing], SRC, &[]);
    let mut p = read(&bin);
    let m = p.module_mut("ThisWorkbook").unwrap();
    m.source = "x".into();
    m.has_source = true;
    unsupported(write_project(&p, &WriteOptions::default()), "ThisWorkbook");
}

#[test]
fn editing_a_present_module_of_a_mixed_project() {
    let p = read(&mixed());
    let before = streams(&mixed());
    let mut q = p.clone();
    q.set_module_source("ThisWorkbook", "Sub B()\r\nEnd Sub\r\n")
        .unwrap();
    // Invalidation would force a recompile the stripped modules cannot survive.
    unsupported(write_project(&q, &WriteOptions::default()), "Sheet1");
    // Keeping the caches is an explicit choice and leaves absent modules alone.
    let o = WriteOptions {
        stale_cache: StaleCache::Keep,
    };
    let out = write_project(&q, &o).unwrap();
    let after = streams(&out);
    for k in ["/VBA/Sheet1", "/VBA/UserForm1", "/VBA/_VBA_PROJECT"] {
        assert_eq!(after[k], before[k], "{k}");
    }
    let back = read(&out);
    assert_eq!(
        back.module("ThisWorkbook").unwrap().source,
        "Sub B()\r\nEnd Sub\r\n"
    );
    assert_eq!(back.source_availability(), ProjectSourceAvailability::Mixed);
}

fn body() -> impl Strategy<Value = Body> {
    prop_oneof![
        (1usize..300).prop_map(Body::CompiledOnly),
        Just(Body::Nothing),
        (0usize..300).prop_map(Body::Source),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Whatever mix of stripped, empty and full modules a project has, an
    /// unedited read/write cycle reproduces every stream, in either read
    /// mode, with either cache policy.
    #[test]
    fn unedited_projects_round_trip_byte_for_byte(
        b in prop::array::uniform3(body()),
        vp_cache in prop::collection::vec(any::<u8>(), 0..200),
        keep in any::<bool>(),
        tolerant in any::<bool>(),
    ) {
        let bin = project_bin(b.clone(), SRC, &vp_cache);
        let mode = if tolerant { ReadMode::Tolerant } else { ReadMode::Strict };
        let p = read_project_with(&bin, &Limits::default(), mode).unwrap();
        let o = WriteOptions {
            stale_cache: if keep { StaleCache::Keep } else { StaleCache::Invalidate },
        };
        let out = write_project_with_report(&p, &o).unwrap();
        prop_assert_eq!(dump_cfb(&out.bytes), dump_cfb(&bin));
        prop_assert!(out.report.changed_streams.is_empty());
        for (m, body) in p.modules.iter().zip(&b) {
            let expect = match body {
                Body::CompiledOnly(_) => SourceAvailability::CompiledOnly,
                Body::Nothing => SourceAvailability::AbsentNoCache,
                Body::Source(_) => SourceAvailability::Present,
            };
            prop_assert_eq!(m.source_availability(), expect);
        }
    }
}
