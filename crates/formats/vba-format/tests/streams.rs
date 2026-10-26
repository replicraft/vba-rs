//! PROJECTwm, PROJECTlk, protection state and digital signature streams.

mod common;

use common::*;
use vba_format::signature::{self, SignatureKind};
use vba_format::{
    Error, LicenseKeys, Limits, ProjectWm, WriteOptions, crypt, keys, license_keys,
    module_name_map, read_project, write_project, write_project_with_report,
};

/// Rebuilds a container, letting `edit` change or add streams.
fn rebuild(bin: &[u8], edit: impl Fn(&mut Vec<(String, Vec<u8>)>)) -> Vec<u8> {
    let (streams, storages) = dump_cfb(bin);
    let mut streams: Vec<(String, Vec<u8>)> = streams
        .into_iter()
        .map(|(p, d)| (p.trim_start_matches('/').to_owned(), d))
        .collect();
    edit(&mut streams);
    let storages: Vec<StorageSpec> = storages
        .into_iter()
        .filter(|(p, _, _)| p != "/")
        .map(|(p, clsid, state_bits)| StorageSpec {
            path: Box::leak(p.trim_start_matches('/').to_owned().into_boxed_str()),
            clsid,
            state_bits,
        })
        .collect();
    build_cfb(&streams, &storages)
}

fn set_stream(streams: &mut Vec<(String, Vec<u8>)>, name: &str, data: Vec<u8>) {
    match streams.iter_mut().find(|(p, _)| p == name) {
        Some((_, d)) => *d = data,
        None => streams.push((name.to_owned(), data)),
    }
}

// ------------------------------------------------------------- PROJECTwm

#[test]
fn project_wm_of_the_sample_is_typed_and_round_trips() {
    let p = read_project(&sample_project_bin(), &Limits::default()).unwrap();
    let wm = module_name_map(&p).unwrap().expect("sample has PROJECTwm");
    let names: Vec<_> = wm.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        ["ThisWorkbook", "Sheet1", "UserForm1", "Module1", "Class1"]
    );
    assert!(wm.entries.iter().all(|e| e.name == e.name_unicode));
    let raw = p
        .preserved
        .get(&format!("{}PROJECTwm", keys::STREAM_PREFIX))
        .unwrap();
    assert_eq!(wm.to_bytes(), raw);
}

#[test]
fn project_wm_keeps_mbcs_and_unicode_names_apart() {
    // "Modül" in code page 1252 is 4D 6F 64 FC 6C.
    let mut raw = vec![0x4D, 0x6F, 0x64, 0xFC, 0x6C, 0];
    raw.extend(utf16("Modül"));
    raw.extend([0, 0, 0, 0]);
    let wm = ProjectWm::parse(&raw, 1252).unwrap();
    assert_eq!(wm.entries.len(), 1);
    assert_eq!(wm.entries[0].name_mbcs, [0x4D, 0x6F, 0x64, 0xFC, 0x6C]);
    assert_eq!(wm.entries[0].name, "Modül");
    assert_eq!(wm.entries[0].name_unicode, "Modül");
    assert_eq!(wm.to_bytes(), raw);
    // The same bytes under an OEM page decode differently (page 437: 0xFC).
    assert_eq!(
        ProjectWm::parse(&raw, 437).unwrap().entries[0].name,
        "Modⁿl"
    );
}

#[test]
fn project_wm_rejects_malformed_streams() {
    for raw in [
        &b""[..],
        &b"\0"[..],
        &b"A\0"[..],                      // no terminator
        &[b'A', 0, b'A', 0, 0, 0][..],    // Unicode name lacks its terminator
        &[0, 0, 0, 0][..],                // empty MBCS name
        &[b'A', 0, 0, 0, 0, 0][..],       // empty Unicode name
        &[b'A', b'B', 0, b'A', 0, 0][..], // odd-length Unicode name
    ] {
        assert!(
            matches!(ProjectWm::parse(raw, 1252), Err(Error::AuxStream { .. })),
            "{raw:?}"
        );
    }
    assert_eq!(ProjectWm::parse(&[0, 0], 1252).unwrap().entries.len(), 0);
}

// ------------------------------------------------------------- PROJECTlk

fn license_stream() -> Vec<u8> {
    let mut v = vec![1, 0];
    v.extend(u32le(2));
    // CLSID {00020430-0000-0000-C000-000000000046}
    v.extend([
        0x30, 0x04, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x46,
    ]);
    v.extend(lp(b"KEY-1"));
    v.extend(u32le(1));
    v.extend([0x11; 16]);
    v.extend(lp(b""));
    v.extend(u32le(0));
    v
}

#[test]
fn project_lk_is_typed_and_round_trips() {
    let raw = license_stream();
    let lk = LicenseKeys::parse(&raw).unwrap();
    assert_eq!(lk.records.len(), 2);
    assert_eq!(
        lk.records[0].class_id,
        "{00020430-0000-0000-C000-000000000046}"
    );
    assert_eq!(lk.records[0].license_key, b"KEY-1");
    assert!(lk.records[0].license_required);
    assert_eq!(lk.records[1].license_key, b"");
    assert!(!lk.records[1].license_required);
    assert!(lk.trailing.is_empty());
    assert_eq!(lk.to_bytes().unwrap(), raw);
}

#[test]
fn project_lk_rejects_bad_version_and_truncation() {
    let mut raw = license_stream();
    raw[0] = 2;
    assert!(LicenseKeys::parse(&raw).is_err());
    let good = license_stream();
    for len in 0..good.len() {
        assert!(LicenseKeys::parse(&good[..len]).is_err(), "prefix {len}");
    }
    // A count that promises far more records than there are bytes.
    let mut huge = vec![1, 0];
    huge.extend(u32le(u32::MAX));
    assert!(LicenseKeys::parse(&huge).is_err());
}

#[test]
fn project_lk_is_found_in_a_project_and_preserved_on_write() {
    let bin = rebuild(&sample_project_bin(), |s| {
        set_stream(s, "PROJECTlk", license_stream())
    });
    let mut p = read_project(&bin, &Limits::default()).unwrap();
    assert_eq!(license_keys(&p).unwrap().unwrap().records.len(), 2);
    p.modules[3].source.push_str("' edit\r\n");
    let out = write_project(&p, &WriteOptions::default()).unwrap();
    let q = read_project(&out, &Limits::default()).unwrap();
    assert_eq!(license_keys(&q).unwrap(), license_keys(&p).unwrap());
    // A project without the stream has none.
    let none = read_project(&sample_project_bin(), &Limits::default()).unwrap();
    assert_eq!(license_keys(&none).unwrap(), None);
}

// ------------------------------------------------------------ protection

const ID: &str = "{917DED54-440B-4FD1-A5C1-74ACF261E600}";

fn protected_project(cmg: u32, password: bool, visible: bool) -> Vec<u8> {
    let cmg_hex = crypt::hex_encode(&crypt::encrypt(&cmg.to_le_bytes(), 0x07, ID));
    let dpb: &[u8] = if password { &[0x42; 29] } else { &[0] };
    let dpb_hex = crypt::hex_encode(&crypt::encrypt(dpb, 0x0E, ID));
    let gc_hex = crypt::hex_encode(&crypt::encrypt(&[if visible { 0xFF } else { 0 }], 0x15, ID));
    rebuild(&sample_project_bin(), |s| {
        let (_, project) = s.iter_mut().find(|(p, _)| p == "PROJECT").unwrap();
        let text = String::from_utf8(project.clone()).unwrap();
        let text = text
            .replace("0705D8E3D8EDDBF1DBF1DBF1DBF1", &cmg_hex)
            .replace("0E0CD1ECDFF4E7F5E7F5E7", &dpb_hex)
            .replace("1517CAF1D6F9D7F9D706", &gc_hex);
        *project = text.into_bytes();
    })
}

#[test]
fn spec_example_project_is_not_locked() {
    let p = read_project(&sample_project_bin(), &Limits::default()).unwrap();
    assert!(!p.protection.locked_for_viewing());
    assert_eq!(p.protection.notice(), None);
}

#[test]
fn each_protection_bit_is_reported_as_locked_for_viewing() {
    for (bits, user, host, vbe) in [
        (1, true, false, false),
        (2, false, true, false),
        (4, false, false, true),
        (7, true, true, true),
    ] {
        let p = read_project(&protected_project(bits, false, true), &Limits::default()).unwrap();
        assert_eq!(
            (
                p.protection.user_protected,
                p.protection.host_protected,
                p.protection.vbe_protected
            ),
            (user, host, vbe),
            "bits {bits}"
        );
        assert!(p.protection.locked_for_viewing());
        assert!(!p.protection.has_password);
        assert!(
            p.protection
                .notice()
                .unwrap()
                .contains("locked for viewing")
        );
    }
}

#[test]
fn password_and_hidden_state_are_reported() {
    let p = read_project(&protected_project(1, true, false), &Limits::default()).unwrap();
    assert!(p.protection.has_password);
    assert!(p.protection.is_hidden());
    assert!(
        p.protection
            .notice()
            .unwrap()
            .contains("password protected")
    );
}

#[test]
fn locked_projects_are_still_fully_readable_and_writable() {
    // The flag is not security: the source is stored in the clear.
    let bin = protected_project(7, true, false);
    let mut p = read_project(&bin, &Limits::default()).unwrap();
    assert!(p.protection.locked_for_viewing());
    assert_eq!(p.module("Module1").unwrap().source, SRC_MODULE1);
    p.module_mut("Module1").unwrap().source.push_str("' x\r\n");
    let out = write_project(&p, &WriteOptions::default()).unwrap();
    let q = read_project(&out, &Limits::default()).unwrap();
    assert!(q.protection.locked_for_viewing());
    assert!(q.module("Module1").unwrap().source.ends_with("' x\r\n"));
}

// ------------------------------------------------------------ signatures

fn sig_blob() -> Vec<u8> {
    // DigSigBlob per MS-OSHARED 2.3.2.2: cb, serializedPointer = 8, then
    // DigSigInfoSerialized (nine u32 fields) and its buffers.
    let signature = b"\x30\x82\x01\x00pretend-pkcs7";
    let cert = b"pretend-cert-store";
    let sig_off = 8 + 36;
    let cert_off = sig_off + signature.len();
    let name_off = cert_off + cert.len();
    let mut info = Vec::new();
    for v in [
        signature.len(),
        sig_off,
        cert.len(),
        cert_off,
        0,
        name_off,
        0,
        0,
        name_off + 2,
    ] {
        info.extend(u32le(v as u32));
    }
    info.extend(signature);
    info.extend(cert);
    info.extend([0, 0, 0, 0]);
    while info.len() % 4 != 0 {
        info.push(0);
    }
    let mut out = u32le(info.len() as u32).to_vec();
    out.extend(u32le(8));
    out.extend(info);
    out
}

fn signed_project() -> Vec<u8> {
    rebuild(&sample_project_bin(), |s| {
        set_stream(s, "\u{5}DigitalSignature", sig_blob());
        set_stream(s, "\u{5}DigitalSignatureEx", sig_blob());
    })
}

#[test]
fn signature_streams_are_detected_and_parsed_not_verified() {
    let p = read_project(&signed_project(), &Limits::default()).unwrap();
    assert!(signature::is_signed(&p));
    let sigs = signature::signatures(&p);
    assert_eq!(sigs.len(), 2);
    let kinds: Vec<_> = sigs.iter().map(|s| s.kind).collect();
    assert!(kinds.contains(&SignatureKind::Legacy) && kinds.contains(&SignatureKind::Ex));
    for s in &sigs {
        let blob = s.blob.as_ref().expect("blob parses");
        assert_eq!(blob.signature, b"\x30\x82\x01\x00pretend-pkcs7");
        assert_eq!(blob.cert_store, b"pretend-cert-store");
    }
    let unsigned = read_project(&sample_project_bin(), &Limits::default()).unwrap();
    assert!(!signature::is_signed(&unsigned));
    assert!(signature::signatures(&unsigned).is_empty());
}

#[test]
fn an_unparsable_signature_stream_is_reported_but_kept() {
    let bin = rebuild(&sample_project_bin(), |s| {
        set_stream(s, "\u{5}DigitalSignatureAgile", b"junk".to_vec())
    });
    let p = read_project(&bin, &Limits::default()).unwrap();
    let sigs = signature::signatures(&p);
    assert_eq!(sigs.len(), 1);
    assert!(sigs[0].blob.is_err());
    let out = write_project(&p, &WriteOptions::default()).unwrap();
    assert!(
        dump_cfb(&out)
            .0
            .iter()
            .any(|(path, d)| path.ends_with("DigitalSignatureAgile") && d == b"junk")
    );
}

#[test]
fn untouched_signed_projects_stay_valid() {
    let p = read_project(&signed_project(), &Limits::default()).unwrap();
    let out = write_project_with_report(&p, &WriteOptions::default()).unwrap();
    assert!(out.report.changed_streams.is_empty());
    assert_eq!(out.report.signature_streams.len(), 2);
    assert!(!out.report.signatures_invalidated);
}

#[test]
fn edits_to_signed_projects_report_invalidation_and_keep_the_signature_bytes() {
    let mut p = read_project(&signed_project(), &Limits::default()).unwrap();
    p.module_mut("Module1")
        .unwrap()
        .source
        .push_str("' edit\r\n");
    let out = write_project_with_report(&p, &WriteOptions::default()).unwrap();
    assert!(out.report.signatures_invalidated);
    assert!(
        out.report
            .changed_streams
            .contains(&"VBA/Module1".to_owned())
    );
    assert!(
        !out.report
            .changed_streams
            .contains(&"VBA/Class1".to_owned())
    );
    // The signature streams themselves are byte-identical.
    let (streams, _) = dump_cfb(&out.bytes);
    for name in ["DigitalSignature", "DigitalSignatureEx"] {
        let (_, d) = streams.iter().find(|(p, _)| p.ends_with(name)).unwrap();
        assert_eq!(d, &sig_blob());
    }
    // Unsigned projects never report invalidation.
    let mut q = read_project(&sample_project_bin(), &Limits::default()).unwrap();
    q.module_mut("Module1")
        .unwrap()
        .source
        .push_str("' edit\r\n");
    let out = write_project_with_report(&q, &WriteOptions::default()).unwrap();
    assert!(!out.report.signatures_invalidated);
    assert!(!out.report.changed_streams.is_empty());
}

#[test]
fn metadata_only_edits_also_report_invalidation() {
    let mut p = read_project(&signed_project(), &Limits::default()).unwrap();
    p.module_mut("Module1").unwrap().doc_string = "changed".into();
    let out = write_project_with_report(&p, &WriteOptions::default()).unwrap();
    assert!(out.report.signatures_invalidated);
    assert!(out.report.changed_streams.contains(&"VBA/dir".to_owned()));
}

#[test]
fn signature_parts_are_read_from_their_own_compound_file() {
    let part = build_cfb(&[("\u{5}DigitalSignature".to_owned(), sig_blob())], &[]);
    let sigs = signature::read_signature_part(&part, &Limits::default()).unwrap();
    assert_eq!(sigs.len(), 1);
    assert_eq!(sigs[0].kind, SignatureKind::Legacy);
    assert!(sigs[0].blob.is_ok());
}
