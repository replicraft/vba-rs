//! End-to-end tests on hand-built synthetic bytes (no real project needed).

mod common;

use common::synth::{self, CacheSpec};
use vba_pcode::*;

fn sample_spec() -> CacheSpec {
    let mut indirect = synth::function_record(synth::user_name(0).map_into());
    indirect.resize(80, 0);
    let mut spec = CacheSpec {
        indirect,
        ..CacheSpec::default()
    };
    let mut l0 = Vec::new();
    l0.extend_from_slice(&synth::word(205, 4)); // OPTION_STMT Explicit
    let mut l1 = Vec::new();
    l1.extend_from_slice(&synth::word(150, 6)); // PROC_HEADER Public Function
    l1.extend_from_slice(&0u32.to_le_bytes());
    let mut l2 = Vec::new();
    l2.extend_from_slice(&synth::word(182, 0)); // CONST_TEXT
    l2.extend_from_slice(&2u16.to_le_bytes());
    l2.extend_from_slice(b"hi");
    l2.extend_from_slice(&synth::word(32, 0)); // READ_SIMPLE
    l2.extend_from_slice(&synth::user_name(1));
    l2.extend_from_slice(&synth::word(39, 8)); // WRITE_SIMPLE with $ suffix
    l2.extend_from_slice(&synth::user_name(0));
    let mut l4 = Vec::new();
    l4.extend_from_slice(&synth::word(224, 0)); // COMMENT_QUOTE
    l4.extend_from_slice(&0u16.to_le_bytes());
    l4.extend_from_slice(&3u16.to_le_bytes());
    l4.extend_from_slice(b"abc");
    l4.push(0);
    let l5 = synth::word(105, 0).to_vec(); // PROC_END_FUNCTION
    spec.lines = vec![l0, l1, l2, Vec::new(), l4, l5];
    spec
}

trait MapInto {
    fn map_into(self) -> u16;
}
impl MapInto for [u8; 2] {
    fn map_into(self) -> u16 {
        u16::from_le_bytes(self)
    }
}

#[test]
fn full_pipeline_on_synthetic_bytes() {
    let project = synth::build_project_stream(0x97, &["Hello", "Count"]);
    let pc = ProjectCache::parse(&project, 1).unwrap();
    assert_eq!(pc.layout.verification(), Verification::Verified);
    assert_eq!(pc.idents.names.len(), 2);
    assert_eq!(pc.idents.get(1).as_deref(), Some("Count"));
    assert_eq!(pc.idents.trailing_bytes(), 0);

    let bytes = synth::build_cache(&sample_spec());
    let cache = ModuleCache::parse(&bytes, &pc.layout).unwrap();
    assert_eq!(cache.lines.lines.len(), 6);
    assert_eq!(cache.references.as_ref().unwrap().entries, vec!["*\\Rtest"]);
    assert!(cache.members.is_none());
    let chk = cache.code_area_check();
    assert_eq!((chk.holes, chk.overlaps), (0, 0));
    // The regions tile the cache and concatenate back to it.
    let mut pos = 0;
    let mut rebuilt = Vec::new();
    for r in &cache.regions {
        assert_eq!(r.span.start, pos);
        pos = r.span.end;
        rebuilt.extend_from_slice(&bytes[r.span.clone()]);
    }
    assert_eq!(rebuilt, bytes);
    let cov = cache.coverage();
    assert_eq!(cov.total, bytes.len());
    assert_eq!(cov.unattributed, 0);

    let d = Disassembly::of_module(&cache, &pc.idents);
    let expected = "\
; vba-pcode disassembly v1
; layout Vba7 x86 version=0x0097 verified
; lines 6
L0000 +0000 OPTION_STMT mod=4
L0001 +0000 PROC_HEADER mod=6 proc{Public Function Hello() As String}
L0002 +0000 CONST_TEXT \"hi\"
L0002 +0006 READ_SIMPLE Count
L0002 +000A WRITE_SIMPLE mod=8 Hello$
L0004 +0000 COMMENT_QUOTE col=0 'abc
L0005 +0000 PROC_END_FUNCTION
";
    assert_eq!(d.to_string(), expected);
}

#[test]
fn member_table_is_found_when_present() {
    let project = synth::build_project_stream(0x97, &["A"]);
    let pc = ProjectCache::parse(&project, 1).unwrap();
    let mut spec = sample_spec();
    spec.members = Some(3);
    let bytes = synth::build_cache(&spec);
    let cache = ModuleCache::parse(&bytes, &pc.layout).unwrap();
    assert_eq!(cache.members.as_ref().unwrap().records.len(), 3);
    assert!(
        cache
            .regions
            .iter()
            .any(|r| r.kind == RegionKind::MemberTable)
    );
}

#[test]
fn declaration_record_and_table_bounds() {
    let mut spec = sample_spec();
    // One declaration record: header of 20 bytes then "Entry\0" padded to 4.
    let mut d = vec![0u8; 20];
    d[2..4].copy_from_slice(&0x0210u16.to_le_bytes());
    d[4..6].copy_from_slice(&20u16.to_le_bytes());
    d.extend_from_slice(b"Entry\0\0\0");
    let rec = DeclarationRecord::parse(&d, 0).unwrap();
    assert_eq!(rec.library, 0x0210);
    assert_eq!(rec.entry_point.as_deref(), Some(&b"Entry"[..]));
    spec.decl = d;
    let bytes = synth::build_cache(&spec);
    let layout = Layout::select(0x97, 1).unwrap();
    let cache = ModuleCache::parse(&bytes, &layout).unwrap();
    assert_eq!(cache.tables.declarations.len(), 28);
    // A declared length past the end of the data is rejected.
    let mut bad = bytes.clone();
    bad[0x3F..0x43].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        ModuleCache::parse(&bad, &layout),
        Err(PcodeError::OutOfBounds { .. })
    ));
}

#[test]
fn explicit_errors_for_unsupported_layouts() {
    let bytes = synth::build_cache(&sample_spec());
    // Big-endian marker: word at offset 2 above 0xFF.
    let mut be = bytes.clone();
    be[2] = 0x01;
    be[3] = 0x02;
    let layout = Layout::select(0x97, 1).unwrap();
    assert!(matches!(
        ModuleCache::parse(&be, &layout),
        Err(PcodeError::UnsupportedLayout(UnsupportedLayout::BigEndian))
    ));
    assert!(matches!(
        Layout::select(0x97, 2),
        Err(PcodeError::UnsupportedLayout(UnsupportedLayout::BigEndian))
    ));
    assert_eq!(Layout::select(0xFFFF, 1), Err(PcodeError::CacheNotUsable));
    // Missing line marker.
    let mut nm = bytes.clone();
    let t = u32::from_le_bytes([nm[0x19], nm[0x1A], nm[0x1B], nm[0x1C]]) as usize + 0x3C;
    nm[t] = 0;
    assert!(matches!(
        ModuleCache::parse(&nm, &layout),
        Err(PcodeError::BadLineMarker { .. })
    ));
}

#[test]
fn project_stream_errors() {
    assert!(matches!(
        version_word(&[0, 0, 0, 0]),
        Err(PcodeError::BadSignature { .. })
    ));
    assert!(matches!(
        version_word(&[0xCC]),
        Err(PcodeError::Truncated { .. })
    ));
    let good = synth::build_project_stream(0x97, &["X"]);
    // Every truncation fails cleanly.
    let layout = Layout::select(0x97, 1).unwrap();
    for n in 0..good.len() {
        assert!(parse_identifiers(&good[..n], &layout).is_err(), "len {n}");
    }
    assert!(parse_identifiers(&good, &layout).is_ok());
}

#[test]
fn x64_context_instruction_has_two_dwords() {
    let l = Layout::select(0xD7, 3).unwrap();
    // PROC_CONTEXT is canonical 90 = stored 90 on x64.
    let mut b = synth::word(90, 0).to_vec();
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&2u32.to_le_bytes());
    let d = decode_line(&b, &l);
    assert!(d.is_complete());
    assert_eq!(d.instructions[0].operands, Operands::Ctx(1, Some(2)));
    let x86 = Layout::select(0x97, 1).unwrap();
    assert_eq!(
        decode_line(&b[..6], &x86).instructions[0].operands,
        Operands::Ctx(1, None)
    );
}
