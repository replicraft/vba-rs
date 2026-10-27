//! Synthetic checks for the layout findings of the second round of local
//! material: identifier numbering with empty entries, the identifier offset
//! by version word, the byte offset of qualified type references, fixed-length
//! string descriptors and the extended module header. Bytes are built here;
//! nothing comes from any real project.

mod common;

use common::synth::{self, CacheSpec};
use vba_pcode::*;

/// A project stream whose identifier list is `alpha`, an entry with an empty
/// name and a type byte of 0x80 (six extra bytes), and `gamma`.
fn stream_with_empty_entry(version: u16) -> Vec<u8> {
    let mut v = synth::build_project_stream(version, &[]);
    // The three counts sit right before the four bytes that end the prefix.
    let at = v.len() - 4 - 6;
    v[at..at + 2].copy_from_slice(&3u16.to_le_bytes());
    v[at + 2..at + 4].copy_from_slice(&3u16.to_le_bytes());
    for (len, typ, text) in [(5u8, 0u8, "alpha"), (0, 0x80, ""), (5, 0, "gamma")] {
        v.push(len);
        v.push(typ);
        if typ & 0x80 != 0 {
            v.extend_from_slice(&[0u8; 6]);
        }
        v.extend_from_slice(text.as_bytes());
        v.extend_from_slice(&[0u8; 4]);
    }
    v
}

#[test]
fn an_entry_with_an_empty_name_takes_an_index() {
    let stream = stream_with_empty_entry(0x5E);
    let layout = Layout::select(0x5E, 1).unwrap();
    let t = parse_identifiers(&stream, &layout).unwrap();
    assert_eq!(t.announced, 3);
    assert_eq!(
        t.names,
        vec![b"alpha".to_vec(), Vec::new(), b"gamma".to_vec()]
    );
}

#[test]
fn identifier_offset_follows_the_version_word() {
    let names: Vec<Vec<u8>> = (0..12).map(|i| format!("n{i}").into_bytes()).collect();
    let idents = IdentTable {
        names,
        announced: 12,
        dummies: 0,
        end: 0,
        stream_len: 0,
    };
    // `j = 5`: the word is `(0x100 + j) << 1`.
    let word = ((0x100u16 + 5) << 1).to_le_bytes();
    let w = u16::from_le_bytes(word);
    for (version, sys, want) in [
        (0x5E, 1, "n5"),
        (0x6D, 1, "n5"),
        (0x88, 1, "n5"),
        (0x8E, 1, "n1"),
        (0x97, 1, "n1"),
        (0xA3, 1, "n1"),
    ] {
        let l = Layout::select(version, sys).unwrap();
        let name = Resolver::new(&idents, l).resolve(w);
        let got = match &name {
            Name::User(s) => s.as_str(),
            other => panic!("{version:#x}: {other:?}"),
        };
        assert_eq!(got, want, "{version:#x}");
    }
    // On x64 the offset is 7.
    let w9 = (0x100u16 + 9) << 1;
    for version in [0x9A, 0xB5, 0xDF] {
        let l64 = Layout::select(version, 3).unwrap();
        assert_eq!(
            Resolver::new(&idents, l64).resolve(w9),
            Name::User("n2".into()),
            "{version:#x}"
        );
    }
}

#[test]
fn qualified_type_offset_is_in_bytes_on_x86() {
    let project = synth::build_project_stream(0x97, &["Lib", "First", "Second", "Var"]);
    let pc = ProjectCache::parse(&project, 1).unwrap();
    // A descriptor of entry 1 (word 4, scaled by the pointer size) and a
    // variable record `v As <entry 1>`.
    let mut ind = vec![0u8; 8];
    let mut d = vec![0u8; 8];
    d[0..2].copy_from_slice(&0x001Du16.to_le_bytes());
    d[2..4].copy_from_slice(&4u16.to_le_bytes());
    ind.extend(d);
    let mut var = vec![0u8; 24];
    var[0] = 0x20;
    var[2..4].copy_from_slice(&synth::user_name(3));
    var[12..16].copy_from_slice(&8u32.to_le_bytes());
    ind.extend(var);
    // Entry 1 is qualified and names the second record of the list: its
    // byte offset 8, not 8 divided by the pointer size.
    let mut e1 = [0u8; 10];
    e1[2..4].copy_from_slice(&0x3C01u16.to_le_bytes());
    e1[6..8].copy_from_slice(&8u16.to_le_bytes());
    let mut r0 = [0u8; 8];
    r0[0..2].copy_from_slice(&2u16.to_le_bytes());
    r0[2..4].copy_from_slice(&synth::user_name(0));
    r0[4..6].copy_from_slice(&synth::user_name(1));
    let mut r1 = r0;
    r1[4..6].copy_from_slice(&synth::user_name(2));
    let mut l0 = synth::word(242, 1).to_vec(); // VAR_DECL
    l0.extend_from_slice(&16u32.to_le_bytes());
    let spec = CacheSpec {
        indirect: ind,
        objects: vec![[0; 10], e1],
        type_refs: vec![r0, r1],
        lines: vec![l0],
        ..CacheSpec::default()
    };
    let bytes = synth::build_cache(&spec);
    let cache = ModuleCache::parse(&bytes, &pc.layout).unwrap();
    let dis = Disassembler::new(&cache, &pc.idents);
    let Operand::Variable(v) = &dis.line(0).instructions[0].operands[0] else {
        panic!("variable expected");
    };
    assert_eq!(v.ty.to_string(), "Lib.Second");
}

#[test]
fn fixed_length_string_descriptor() {
    let project = synth::build_project_stream(0x97, &["A", "B", "C", "Var"]);
    let pc = ProjectCache::parse(&project, 1).unwrap();
    for (flags, len, pos_bit, want) in [
        (0x0020u16, 64u16, 0u32, "String * 64"),
        (0x1020, 28, 0, "String * 28"),
        // Length held by an expression in front of the declaration.
        (0x0020, 0, 0, "String * ?"),
        // A stored position with bit 0 set.
        (0x0020, 512, 1, "String * 512"),
    ] {
        let mut ind = vec![0u8; 8];
        let mut d = vec![0u8; 8];
        d[0..2].copy_from_slice(&flags.to_le_bytes());
        d[2..4].copy_from_slice(&len.to_le_bytes());
        ind.extend(d);
        let mut var = vec![0u8; 24];
        var[0] = 0x20;
        var[2..4].copy_from_slice(&synth::user_name(3));
        var[12..16].copy_from_slice(&(8 | pos_bit).to_le_bytes());
        ind.extend(var);
        let mut l0 = synth::word(242, 1).to_vec();
        l0.extend_from_slice(&16u32.to_le_bytes());
        let spec = CacheSpec {
            indirect: ind,
            lines: vec![l0],
            ..CacheSpec::default()
        };
        let bytes = synth::build_cache(&spec);
        let cache = ModuleCache::parse(&bytes, &pc.layout).unwrap();
        let dis = Disassembler::new(&cache, &pc.idents);
        let Operand::Variable(v) = &dis.line(0).instructions[0].operands[0] else {
            panic!("variable expected");
        };
        assert_eq!(v.ty.to_string(), want);
    }
}

#[test]
fn extended_header_moves_the_declaration_table() {
    let project = synth::build_project_stream(0xDF, &["A"]);
    let pc = ProjectCache::parse(&project, 3).unwrap();
    assert_eq!(pc.layout.verification(), Verification::Verified);
    let plain = CacheSpec {
        x64: true,
        lines: vec![synth::word(150, 6).to_vec()],
        ..CacheSpec::default()
    };
    let ext = CacheSpec {
        ext: true,
        ..plain.clone()
    };
    let a = synth::build_cache(&plain);
    let b = synth::build_cache(&ext);
    // The platform word is 3 in one and 0x13 in the other: only the second
    // is the extended header, whatever the low nibble.
    assert_eq!(u16::from_le_bytes([a[2], a[3]]), 3);
    assert_eq!(u16::from_le_bytes([b[2], b[3]]), 0x13);
    let ca = ModuleCache::parse(&a, &pc.layout).unwrap();
    let cb = ModuleCache::parse(&b, &pc.layout).unwrap();
    assert_eq!(ca.tables.declarations_len_at, 0x3F);
    assert_eq!(cb.tables.declarations_len_at, 0x43);
    assert_eq!(cb.lines.lines.len(), 1);
    // A low nibble that disagrees with the project is refused without the
    // extension bit and accepted with it.
    let mut c = b.clone();
    c[2] = 0x11;
    assert!(ModuleCache::parse(&c, &pc.layout).is_ok());
    let mut d = a.clone();
    d[2] = 1;
    assert!(matches!(
        ModuleCache::parse(&d, &pc.layout),
        Err(PcodeError::UnsupportedLayout(
            UnsupportedLayout::PlatformMismatch { .. }
        ))
    ));
}

#[test]
fn verified_layout_list_covers_the_new_version_words() {
    for (v, sys) in [
        (0x5E, 1),
        (0x6D, 1),
        (0x8E, 1),
        (0x91, 1),
        (0xA3, 1),
        (0xDF, 3),
    ] {
        let l = Layout::select(v, sys).unwrap();
        assert_eq!(l.verification(), Verification::Verified, "{v:#x}");
    }
    // Combinations not seen stay unverified.
    assert_eq!(
        Layout::select(0x5E, 3).err().map(|_| ()),
        Some(()),
        "VBA 5 on 64-bit Windows is refused"
    );
    assert_eq!(
        Layout::select(0xDF, 1).unwrap().verification(),
        Verification::SpecUnverified
    );
}
