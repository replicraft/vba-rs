//! Test support: hand-assembled MS-OVBA structures.
//!
//! Everything here builds bytes directly from the layouts in the
//! specification, independently of the crate's own `dir` serializer, so the
//! reader is tested against separately constructed input. The `dir` stream
//! and PROJECT text reproduce the worked examples of MS-OVBA §3.1.
//!
//! This file is included both by the integration tests and by unit tests
//! inside the crate.
#![allow(dead_code)]

use std::io::{Cursor, Write};

pub fn u16le(v: u16) -> [u8; 2] {
    v.to_le_bytes()
}

pub fn u32le(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

/// `id`, size, payload.
pub fn rec(id: u16, payload: &[u8]) -> Vec<u8> {
    let mut v = u16le(id).to_vec();
    v.extend(u32le(payload.len() as u32));
    v.extend(payload);
    v
}

/// Length-prefixed bytes.
pub fn lp(bytes: &[u8]) -> Vec<u8> {
    let mut v = u32le(bytes.len() as u32).to_vec();
    v.extend(bytes);
    v
}

pub fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// A record with an MBCS half and a Unicode half (ASCII text only).
pub fn pair(id: u16, reserved: u16, text: &str) -> Vec<u8> {
    let mut v = u16le(id).to_vec();
    v.extend(lp(text.as_bytes()));
    v.extend(u16le(reserved));
    v.extend(lp(&utf16(text)));
    v
}

/// PROJECTINFORMATION exactly as in MS-OVBA §3.1.2.1.
pub fn project_info() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend(rec(0x0001, &u32le(1))); // SYSKIND: 32-bit Windows
    v.extend(rec(0x0002, &u32le(0x409))); // LCID
    v.extend(rec(0x0014, &u32le(0x409))); // LCIDINVOKE
    v.extend(rec(0x0003, &u16le(0x04E4))); // CODEPAGE 1252
    v.extend(rec(0x0004, b"VBAProject"));
    v.extend(pair(0x0005, 0x0040, "Example VBA Project Description"));
    v.extend(u16le(0x0006));
    v.extend(lp(b"c:\\example path\\example.hlp"));
    v.extend(u16le(0x003D));
    v.extend(lp(b"c:\\example path\\example.hlp"));
    v.extend(rec(0x0007, &u32le(1))); // HELPCONTEXT
    v.extend(rec(0x0008, &u32le(0))); // LIBFLAGS
    v.extend(u16le(0x0009)); // VERSION
    v.extend(u32le(4));
    v.extend(u32le(0x49B5196B));
    v.extend(u16le(6));
    v.extend(pair(0x000C, 0x003C, "abc = 1")); // CONSTANTS
    v
}

fn reference_name(name: &str) -> Vec<u8> {
    pair(0x0016, 0x003E, name)
}

const STDOLE: &str = "*\\G{00020430-0000-0000-C000-000000000046}#2.0#0#C:\\Windows\\system32\\stdole2.tlb#OLE Automation";
const OFFICE: &str = "*\\G{2DF8D04C-5BFA-101B-BDE5-00AA0044DE52}#2.0#0#C:\\Program Files\\Common Files\\Microsoft Shared\\OFFICE12\\MSO.DLL#Microsoft Office 12.0 Object Library";
pub const MSFORMS_EXT: &str = "*\\G{896C2D83-5466-46ED-8FAE-4C3E4F85E710}#2.0#0#C:\\Users\\jsmith\\AppData\\Local\\Temp\\VBE\\MSForms.exd#Microsoft Forms 2.0 Object Library";
pub const MSFORMS_TWIDDLED: &str = "*\\G{00000000-0000-0000-0000-000000000000}#0.0#0##";
pub const ORIGINAL_TYPE_LIB: [u8; 16] = [
    0xE1, 0x2E, 0x45, 0x0D, 0x8F, 0xE0, 0x1A, 0x10, 0x85, 0x2E, 0x02, 0x60, 0x8C, 0x4D, 0x0B, 0xB4,
];

pub fn reference_registered(name: &str, libid: &str) -> Vec<u8> {
    let mut v = reference_name(name);
    let mut body = lp(libid.as_bytes());
    body.extend(u32le(0));
    body.extend(u16le(0));
    v.extend(rec(0x000D, &body));
    v
}

pub fn reference_project(name: &str, abs: &str, rel: &str, major: u32, minor: u16) -> Vec<u8> {
    let mut v = reference_name(name);
    let mut body = lp(abs.as_bytes());
    body.extend(lp(rel.as_bytes()));
    body.extend(u32le(major));
    body.extend(u16le(minor));
    v.extend(rec(0x000E, &body));
    v
}

/// REFERENCECONTROL, optionally preceded by a REFERENCEORIGINAL.
pub fn reference_control(name: &str, original: Option<&str>) -> Vec<u8> {
    let mut v = reference_name(name);
    if let Some(o) = original {
        v.extend(u16le(0x0033));
        v.extend(lp(o.as_bytes()));
    }
    let mut first = lp(MSFORMS_TWIDDLED.as_bytes());
    first.extend(u32le(0));
    first.extend(u16le(0));
    v.extend(rec(0x002F, &first));
    v.extend(reference_name(name));
    let mut ext = lp(MSFORMS_EXT.as_bytes());
    ext.extend(u32le(0));
    ext.extend(u16le(0));
    ext.extend(ORIGINAL_TYPE_LIB);
    ext.extend(u32le(1));
    v.extend(rec(0x0030, &ext));
    v
}

/// The four references of MS-OVBA §3.1.2.2.
pub fn spec_references() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend(reference_registered("stdole", STDOLE));
    v.extend(reference_registered("Office", OFFICE));
    v.extend(reference_project(
        "VBAProject1",
        "*\\CC:\\Example Path\\Example-ReferencedProject.xls",
        "*\\CExample-ReferencedProject.xls",
        0x49A95F46,
        0x000D,
    ));
    v.extend(reference_control("MSForms", None));
    v
}

/// What a MODULE record should contain.
#[derive(Clone)]
pub struct ModSpec {
    pub name: &'static str,
    pub doc: &'static str,
    pub offset: u32,
    pub help: u32,
    /// MODULETYPE 0x0021 (procedural) rather than 0x0022.
    pub procedural: bool,
    pub read_only: bool,
    pub private: bool,
}

pub fn module_record(m: &ModSpec) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend(rec(0x0019, m.name.as_bytes()));
    v.extend(rec(0x0047, &utf16(m.name)));
    v.extend(pair(0x001A, 0x0032, m.name));
    v.extend(pair(0x001C, 0x0048, m.doc));
    v.extend(rec(0x0031, &u32le(m.offset)));
    v.extend(rec(0x001E, &u32le(m.help)));
    v.extend(rec(0x002C, &u16le(0xFFFF)));
    v.extend(u16le(if m.procedural { 0x0021 } else { 0x0022 }));
    v.extend(u32le(0));
    if m.read_only {
        v.extend(u16le(0x0025));
        v.extend(u32le(0));
    }
    if m.private {
        v.extend(u16le(0x0028));
        v.extend(u32le(0));
    }
    v.extend(u16le(0x002B));
    v.extend(u32le(0));
    v
}

/// The three modules of MS-OVBA §3.1.2.3.
pub fn spec_modules() -> Vec<ModSpec> {
    vec![
        ModSpec {
            name: "ThisWorkbook",
            doc: "",
            offset: 0,
            help: 0,
            procedural: false,
            read_only: false,
            private: false,
        },
        ModSpec {
            name: "Sheet1",
            doc: "Example Module Description",
            offset: 0,
            help: 2,
            procedural: false,
            read_only: false,
            private: false,
        },
        ModSpec {
            name: "UserForm1",
            doc: "",
            offset: 0,
            help: 0,
            procedural: false,
            read_only: false,
            private: true,
        },
    ]
}

/// A complete decompressed `dir` stream.
pub fn dir_stream(references: &[u8], modules: &[ModSpec]) -> Vec<u8> {
    let mut v = project_info();
    v.extend(references);
    v.extend(rec(0x000F, &u16le(modules.len() as u16)));
    v.extend(rec(0x0013, &u16le(0xFFFF)));
    for m in modules {
        v.extend(module_record(m));
    }
    v.extend(u16le(0x0010));
    v.extend(u32le(0));
    v
}

/// The `dir` stream of the specification's worked example.
pub fn spec_dir_stream() -> Vec<u8> {
    dir_stream(&spec_references(), &spec_modules())
}

/// MS-OVBA §3.1.6 PROJECT stream, with extra module lines inserted after the
/// document modules.
pub fn project_stream(extra_module_lines: &str) -> Vec<u8> {
    let text = "ID=\"{917DED54-440B-4FD1-A5C1-74ACF261E600}\"\r\nDocument=ThisWorkbook/&H00000000\r\nDocument=Sheet1/&H00000000\r\n".to_owned()
        + extra_module_lines
        + "Package={AC9F2F90-E877-11CE-9F68-00AA00574A4F}\r\nBaseClass=UserForm1\r\nHelpFile=\"c:\\example path\\example.hlp\"\r\nName=\"VBAProject\"\r\nHelpContextID=\"1\"\r\nDescription=\"Example VBA Project Description\"\r\nVersionCompatible32=\"393222000\"\r\nCMG=\"0705D8E3D8EDDBF1DBF1DBF1DBF1\"\r\nDPB=\"0E0CD1ECDFF4E7F5E7F5E7\"\r\nGC=\"1517CAF1D6F9D7F9D706\"\r\n\r\n[Host Extender Info]\r\n&H00000001={3832D640-CF90-11CF-8E43-00A0C911005A};VBE;&H00000000\r\n\r\n[Workspace]\r\nThisWorkbook=23, 23, 911, 280, \r\nSheet1=69, 69, 724, 317, C\r\nUserForm1=0, 0, 0, 0, C, 46, 46, 701, 294, Z";
    text.into_bytes()
}

/// A module stream: performance cache followed by compressed source.
pub fn module_stream(cache: &[u8], source: &[u8]) -> Vec<u8> {
    let mut v = cache.to_vec();
    v.extend(vba_format::compress(source));
    v
}

/// Deterministic filler bytes.
pub fn filler(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

/// A storage to create in a test container.
pub struct StorageSpec {
    pub path: &'static str,
    pub clsid: [u8; 16],
    pub state_bits: u32,
}

/// Builds a compound file from streams and storages.
pub fn build_cfb(streams: &[(String, Vec<u8>)], storages: &[StorageSpec]) -> Vec<u8> {
    let mut cf = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    for s in storages {
        let p = format!("/{}", s.path);
        cf.create_storage_all(&p).unwrap();
        cf.set_storage_clsid(&p, uuid::Uuid::from_bytes(s.clsid))
            .unwrap();
        cf.set_state_bits(&p, s.state_bits).unwrap();
    }
    for (path, data) in streams {
        let full = format!("/{path}");
        if let Some((parent, _)) = full.rsplit_once('/')
            && !parent.is_empty()
            && !cf.is_storage(parent)
        {
            cf.create_storage_all(parent).unwrap();
        }
        let mut s = cf.create_stream(&full).unwrap();
        s.write_all(data).unwrap();
    }
    cf.flush().unwrap();
    cf.into_inner().into_inner()
}

/// Sources of the sample project's modules.
pub const SRC_THISWORKBOOK: &str = "Attribute VB_Name = \"ThisWorkbook\"\r\nAttribute VB_Base = \"0{00020819-0000-0000-C000-000000000046}\"\r\nAttribute VB_PredeclaredId = True\r\nAttribute VB_Exposed = True\r\nAttribute VB_TemplateDerived = False\r\nAttribute VB_Customizable = True\r\n";
pub const SRC_SHEET1: &str = "Attribute VB_Name = \"Sheet1\"\r\nAttribute VB_Base = \"0{00020820-0000-0000-C000-000000000046}\"\r\n#If abc = 1 Then\r\nSub Hello()\r\nEnd Sub\r\n#End If\r\n";
pub const SRC_USERFORM1: &str = "Attribute VB_Name = \"UserForm1\"\r\nAttribute VB_Base = \"0{C62A69F0-16DC-11CE-9E98-00AA00574A4F}\"\r\n";
pub const SRC_MODULE1: &str = "Attribute VB_Name = \"Module1\"\r\nOption Explicit\r\nPublic Sub Greet()\r\n    MsgBox \"hi\"\r\nEnd Sub\r\n";
pub const SRC_CLASS1: &str = "Attribute VB_Name = \"Class1\"\r\nAttribute VB_Creatable = False\r\nOption Explicit\r\nPublic Value As Long\r\n";

pub const CACHE_MODULE1: usize = 37;
pub const CACHE_CLASS1: usize = 11;
pub const CACHE_VBA_PROJECT: usize = 25;

/// A realistic sample `vbaProject.bin`: the specification's three modules plus
/// a standard module and a class module with performance caches, unknown
/// streams, a designer storage and SRP streams.
pub fn sample_project_bin() -> Vec<u8> {
    let mut modules = spec_modules();
    modules.push(ModSpec {
        name: "Module1",
        doc: "",
        offset: CACHE_MODULE1 as u32,
        help: 0,
        procedural: true,
        read_only: false,
        private: false,
    });
    modules.push(ModSpec {
        name: "Class1",
        doc: "",
        offset: CACHE_CLASS1 as u32,
        help: 0,
        procedural: false,
        read_only: true,
        private: false,
    });
    let dir = vba_format::compress(&dir_stream(&spec_references(), &modules));

    let mut vba_project = vec![0xCC, 0x61, 0xFF, 0xFF, 0x00, 0x01, 0x00];
    vba_project.extend(filler(CACHE_VBA_PROJECT, 3));

    let mut projectwm = Vec::new();
    for m in &modules {
        projectwm.extend(m.name.as_bytes());
        projectwm.push(0);
        projectwm.extend(utf16(m.name));
        projectwm.extend([0, 0]);
    }
    projectwm.extend([0, 0]);

    let streams: Vec<(String, Vec<u8>)> = vec![
        ("VBA/dir".into(), dir),
        ("VBA/_VBA_PROJECT".into(), vba_project),
        ("VBA/ThisWorkbook".into(), module_stream(&[], SRC_THISWORKBOOK.as_bytes())),
        ("VBA/Sheet1".into(), module_stream(&[], SRC_SHEET1.as_bytes())),
        ("VBA/UserForm1".into(), module_stream(&[], SRC_USERFORM1.as_bytes())),
        (
            "VBA/Module1".into(),
            module_stream(&filler(CACHE_MODULE1, 9), SRC_MODULE1.as_bytes()),
        ),
        (
            "VBA/Class1".into(),
            module_stream(&filler(CACHE_CLASS1, 21), SRC_CLASS1.as_bytes()),
        ),
        ("VBA/__SRP_0".into(), filler(300, 1)),
        ("VBA/__SRP_1".into(), filler(40, 2)),
        (
            "PROJECT".into(),
            project_stream("Module=Module1\r\nClass=Class1\r\n"),
        ),
        ("PROJECTwm".into(), projectwm),
        ("UserForm1/\u{3}VBFrame".into(), b"VERSION 5.00\r\nBegin {C62A69F0-16DC-11CE-9E98-00AA00574A4F} UserForm1\r\n   Caption = \"UserForm1\"\r\nEnd\r\n".to_vec()),
        ("UserForm1/f".into(), filler(120, 5)),
        ("UserForm1/o".into(), filler(77, 6)),
        ("Unknown/blob".into(), filler(5000, 7)),
        ("XTRA".into(), filler(10, 8)),
    ];
    let storages = [
        StorageSpec {
            path: "VBA",
            clsid: [0; 16],
            state_bits: 0,
        },
        StorageSpec {
            path: "UserForm1",
            clsid: [
                0xC0, 0x69, 0x2A, 0xC6, 0xDC, 0x16, 0xCE, 0x11, 0x9E, 0x98, 0x00, 0xAA, 0x00, 0x57,
                0x4A, 0x4F,
            ],
            state_bits: 0,
        },
        StorageSpec {
            path: "Unknown",
            clsid: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            state_bits: 0x55,
        },
    ];
    build_cfb(&streams, &storages)
}

/// Reads every stream and storage of a compound file as `(path, bytes)` and
/// `(path, clsid, state_bits)`.
pub type Dump = (Vec<(String, Vec<u8>)>, Vec<(String, [u8; 16], u32)>);

pub fn dump_cfb(bytes: &[u8]) -> Dump {
    use std::io::Read;
    let mut cf = cfb::CompoundFile::open(Cursor::new(bytes)).unwrap();
    let entries: Vec<_> = cf
        .walk()
        .map(|e| {
            (
                e.path().to_string_lossy().into_owned(),
                e.is_stream(),
                *e.clsid().as_bytes(),
                e.state_bits(),
            )
        })
        .collect();
    let mut streams = Vec::new();
    let mut storages = Vec::new();
    for (path, is_stream, clsid, bits) in entries {
        if is_stream {
            let mut buf = Vec::new();
            cf.open_stream(&path)
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
            streams.push((path, buf));
        } else {
            storages.push((path, clsid, bits));
        }
    }
    streams.sort();
    storages.sort();
    (streams, storages)
}
