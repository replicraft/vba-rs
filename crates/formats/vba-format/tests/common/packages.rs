//! Hand-built OOXML packages, legacy documents and PowerPoint streams.
#![allow(dead_code)]

use super::*;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

/// One shared sample: container timestamps differ between builds.
pub fn sample() -> &'static [u8] {
    static SAMPLE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    SAMPLE.get_or_init(sample_project_bin)
}

pub const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="bin" ContentType="application/vnd.ms-office.vbaProject"/></Types>"#;

pub fn rels(entries: &[(&str, &str, &str)]) -> String {
    let mut s = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (i, (ty, target, extra)) in entries.iter().enumerate() {
        s.push_str(&format!(
            r#"<Relationship Id="rId{i}" Type="{ty}" Target="{target}"{extra}/>"#
        ));
    }
    s.push_str("</Relationships>");
    s
}

pub const OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
pub const VBA: &str = "http://schemas.microsoft.com/office/2006/relationships/vbaProject";

pub fn opts(method: CompressionMethod, second: u8) -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(method)
        .last_modified_time(DateTime::from_date_and_time(2021, 5, 17, 12, 30, second).unwrap())
        .unix_permissions(0o644)
}

/// A package whose VBA project lives at `vba_part`, referenced from the main
/// part `main` with relationship target `target`.
pub fn package(main: &str, target: &str, vba_part: &str, project: &[u8], decoy: bool) -> Vec<u8> {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    w.set_raw_comment(b"package comment".to_vec().into_boxed_slice())
        .unwrap();
    let put =
        |w: &mut ZipWriter<Cursor<Vec<u8>>>, name: &str, data: &[u8], o: SimpleFileOptions| {
            w.start_file(name, o).unwrap();
            w.write_all(data).unwrap();
        };
    put(
        &mut w,
        "[Content_Types].xml",
        CONTENT_TYPES.as_bytes(),
        opts(CompressionMethod::Deflated, 2),
    );
    put(
        &mut w,
        "_rels/.rels",
        rels(&[(OFFICE_DOCUMENT, main, "")]).as_bytes(),
        opts(CompressionMethod::Deflated, 4),
    );
    put(&mut w, main, b"<doc/>", opts(CompressionMethod::Stored, 6));
    let rels_name = match main.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{main}.rels"),
    };
    put(
        &mut w,
        &rels_name,
        rels(&[
            ("http://example.com/other", "other.xml", ""),
            (
                "http://example.com/ext",
                "http://example.com/x.bin",
                r#" TargetMode="External""#,
            ),
            (VBA, target, ""),
        ])
        .as_bytes(),
        opts(CompressionMethod::Deflated, 8),
    );
    w.add_directory("docProps/", opts(CompressionMethod::Stored, 10))
        .unwrap();
    put(
        &mut w,
        "docProps/app.xml",
        &vec![b'a'; 4000],
        opts(CompressionMethod::Deflated, 12),
    );
    put(
        &mut w,
        vba_part,
        project,
        opts(CompressionMethod::Deflated, 14),
    );
    if decoy {
        put(
            &mut w,
            "word/vbaProject.bin",
            b"not the project; only the relationship counts",
            opts(CompressionMethod::Stored, 16),
        );
    }
    put(
        &mut w,
        "word/\u{e9}t\u{e9}.xml",
        b"unicode name",
        opts(CompressionMethod::Stored, 18),
    );
    w.finish().unwrap().into_inner()
}

pub fn docm() -> Vec<u8> {
    package(
        "word/document.xml",
        "vbaProject.bin",
        "word/vbaProject.bin",
        sample(),
        false,
    )
}

pub fn zip_of(entries: &[(&str, String)]) -> Vec<u8> {
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        w.start_file(*name, opts(CompressionMethod::Deflated, 2))
            .unwrap();
        w.write_all(data.as_bytes()).unwrap();
    }
    w.finish().unwrap().into_inner()
}

/// The streams of the sample project under `prefix` (empty for the root).
pub fn project_streams(prefix: &str) -> Dump {
    let (streams, storages) = dump_cfb(sample());
    let streams = streams
        .into_iter()
        .map(|(p, d)| (format!("{prefix}{}", p.trim_start_matches('/')), d))
        .collect();
    let storages = storages
        .into_iter()
        .filter(|(p, _, _)| p != "/")
        .map(|(p, c, b)| (format!("{prefix}{}", p.trim_start_matches('/')), c, b))
        .collect();
    (streams, storages)
}

pub fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

pub fn legacy_document(root_storage: &str, host_streams: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let prefix = format!("{root_storage}/");
    let (mut streams, storages) = project_streams(&prefix);
    for (n, d) in host_streams {
        streams.push(((*n).to_owned(), d.clone()));
    }
    let mut specs: Vec<StorageSpec> = vec![StorageSpec {
        path: leak(root_storage.to_owned()),
        clsid: [0; 16],
        state_bits: 0,
    }];
    specs.extend(
        storages
            .into_iter()
            .map(|(p, clsid, state_bits)| StorageSpec {
                path: leak(p),
                clsid,
                state_bits,
            }),
    );
    build_cfb(&streams, &specs)
}

pub fn ppt_rec(ver: u8, instance: u16, ty: u16, body: &[u8]) -> Vec<u8> {
    let mut v = (u16::from(ver) | (instance << 4)).to_le_bytes().to_vec();
    v.extend(ty.to_le_bytes());
    v.extend((body.len() as u32).to_le_bytes());
    v.extend(body);
    v
}

pub fn document_container(vba_persist_id: u32, has_macros: u32) -> Vec<u8> {
    let atom = ppt_rec(
        2,
        0,
        0x0400,
        &[
            u32le(vba_persist_id).as_slice(),
            &u32le(has_macros),
            &u32le(2),
        ]
        .concat(),
    );
    let vba_info = ppt_rec(0xF, 1, 0x03FF, &atom);
    let other = ppt_rec(0, 0, 0x0FC9, &[0; 4]);
    let list = ppt_rec(0xF, 0, 0x07D0, &[other, vba_info].concat());
    let document_atom = ppt_rec(1, 0, 0x03E9, &[0; 40]);
    ppt_rec(0xF, 0, 0x03E8, &[document_atom, list].concat())
}

pub fn persist_dir(entries: &[(u32, usize)]) -> Vec<u8> {
    let mut body = Vec::new();
    for &(id, off) in entries {
        body.extend(((1u32 << 20) | id).to_le_bytes());
        body.extend((off as u32).to_le_bytes());
    }
    ppt_rec(0, 0, 0x1772, &body)
}

pub fn user_edit(last_edit: usize, dir: usize, doc_id: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend(u32le(0)); // lastSlideIdRef
    body.extend([0, 0, 0, 3]); // version, minor, major
    body.extend(u32le(last_edit as u32));
    body.extend(u32le(dir as u32));
    body.extend(u32le(doc_id));
    body.extend(u32le(3)); // persistIdSeed
    body.extend([0, 0, 0, 0]);
    ppt_rec(0, 0, 0x0FF5, &body)
}

pub fn current_user(edit: usize, token: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend(u32le(0x14));
    body.extend(u32le(token));
    body.extend(u32le(edit as u32));
    body.extend([0, 0]); // lenUserName
    body.extend(0x03F4u16.to_le_bytes());
    body.extend([3, 0, 0, 0]);
    body.extend(u32le(8)); // relVersion
    ppt_rec(0, 0, 0x0FF6, &body)
}

pub fn vba_stg(compressed: bool, storage: &[u8]) -> Vec<u8> {
    if compressed {
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(storage).unwrap();
        let z = enc.finish().unwrap();
        let mut body = u32le(storage.len() as u32).to_vec();
        body.extend(z);
        ppt_rec(0, 1, 0x1011, &body)
    } else {
        ppt_rec(0, 0, 0x1011, storage)
    }
}

/// Two user edits. The first writes the document and an outdated VBA
/// storage; the second replaces only the VBA storage.
pub fn presentation(compressed: bool, has_macros: u32, token: u32) -> Vec<u8> {
    let old_stg = vba_stg(false, b"outdated storage");
    let doc = document_container(2, has_macros);
    let mut stream = Vec::new();
    let old_off = stream.len();
    stream.extend(&old_stg);
    let doc_off = stream.len();
    stream.extend(&doc);
    let dir1_off = stream.len();
    stream.extend(persist_dir(&[(1, doc_off), (2, old_off)]));
    let edit1_off = stream.len();
    stream.extend(user_edit(0, dir1_off, 1));
    let new_off = stream.len();
    stream.extend(vba_stg(compressed, sample()));
    let dir2_off = stream.len();
    stream.extend(persist_dir(&[(2, new_off)]));
    let edit2_off = stream.len();
    stream.extend(user_edit(edit1_off, dir2_off, 1));
    build_cfb(
        &[
            ("Current User".into(), current_user(edit2_off, token)),
            ("PowerPoint Document".into(), stream),
        ],
        &[],
    )
}
