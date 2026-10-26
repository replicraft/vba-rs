//! A minimal MSFT type library writer used only by tests. It emits the
//! layout documented in `docs/formats/typelib.md` (not byte-identical to any
//! compiler's output: hash tables are zero, unknown header words are fixed).

use std::collections::HashMap;

pub use vba_typelib::{FuncKind, InvokeKind, TypeKind, VarKind};

/// A type reference in a spec.
#[derive(Debug, Clone, Copy)]
pub enum R {
    Local(usize),
    Import(usize),
}

/// A type description in a spec.
#[derive(Debug, Clone)]
pub enum Dt {
    Prim(u16),
    Ptr(Box<Dt>),
    Safe(Box<Dt>),
    CArray(Box<Dt>, Vec<(u32, i32)>),
    User(R),
}

#[derive(Debug, Clone)]
pub enum Cv {
    I4(i32),
    U4(u32),
    I2(i16),
    Bool(bool),
    Str(String),
    R8(f64),
    Nothing,
}

#[derive(Debug, Clone)]
pub struct ParamSpec {
    pub name: Option<String>,
    pub ty: Dt,
    pub flags: u32,
    pub default: Option<Cv>,
}

#[derive(Debug, Clone)]
pub struct FuncSpec {
    pub name: String,
    pub memid: i32,
    pub kind: FuncKind,
    pub invoke: InvokeKind,
    pub flags: u32,
    pub vtable_offset: u16,
    pub ret: Dt,
    pub params: Vec<ParamSpec>,
    pub optional: i16,
    pub help: Option<(u32, String)>,
}

#[derive(Debug, Clone)]
pub struct VarSpec {
    pub name: String,
    pub memid: i32,
    pub kind: VarKind,
    pub ty: Dt,
    pub flags: u32,
    pub offset: u32,
    pub value: Option<Cv>,
}

#[derive(Debug, Clone)]
pub struct TypeSpec {
    pub name: String,
    pub kind: TypeKind,
    pub guid: Option<[u8; 16]>,
    pub flags: u32,
    pub doc: Option<String>,
    pub alias: Option<Dt>,
    pub implements: Vec<(R, u32)>,
    pub funcs: Vec<FuncSpec>,
    pub vars: Vec<VarSpec>,
    pub dll: Option<String>,
    pub size: u32,
    pub vft: u16,
}

#[derive(Debug, Clone)]
pub struct ImportSpec {
    pub file: String,
    pub lib_guid: [u8; 16],
    pub type_guid: [u8; 16],
    pub kind: u8,
}

#[derive(Debug, Clone)]
pub struct LibSpec {
    pub name: String,
    pub guid: [u8; 16],
    pub lcid: u32,
    pub version: (u16, u16),
    pub flags: u32,
    pub sys_kind: u32,
    pub doc: Option<String>,
    pub imports: Vec<ImportSpec>,
    pub types: Vec<TypeSpec>,
}

impl TypeSpec {
    pub fn new(name: &str, kind: TypeKind) -> TypeSpec {
        TypeSpec {
            name: name.to_owned(),
            kind,
            guid: None,
            flags: 0,
            doc: None,
            alias: None,
            implements: Vec::new(),
            funcs: Vec::new(),
            vars: Vec::new(),
            dll: None,
            size: 0,
            vft: 0,
        }
    }
}

impl FuncSpec {
    pub fn new(name: &str, memid: i32, invoke: InvokeKind, ret: Dt) -> FuncSpec {
        FuncSpec {
            name: name.to_owned(),
            memid,
            kind: FuncKind::Dispatch,
            invoke,
            flags: 0,
            vtable_offset: 0,
            ret,
            params: Vec::new(),
            optional: 0,
            help: None,
        }
    }

    pub fn param(mut self, name: &str, ty: Dt, flags: u32) -> FuncSpec {
        self.params.push(ParamSpec {
            name: Some(name.to_owned()),
            ty,
            flags,
            default: None,
        });
        self
    }
}

impl LibSpec {
    pub fn new(name: &str) -> LibSpec {
        LibSpec {
            name: name.to_owned(),
            guid: [7; 16],
            lcid: 0x409,
            version: (1, 0),
            flags: 0,
            sys_kind: 1,
            doc: None,
            imports: Vec::new(),
            types: Vec::new(),
        }
    }
}

const TI_SIZE: usize = 0x64;

fn pad4(v: &mut Vec<u8>, fill: u8) {
    while !v.len().is_multiple_of(4) {
        v.push(fill);
    }
}

fn put32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn put16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}

#[derive(Default)]
struct Pools {
    names: Vec<u8>,
    name_map: HashMap<String, u32>,
    strings: Vec<u8>,
    string_map: HashMap<String, u32>,
    guids: Vec<u8>,
    guid_map: HashMap<[u8; 16], u32>,
    typdesc: Vec<u8>,
    arraydesc: Vec<u8>,
    custdata: Vec<u8>,
    impinfo: Vec<u8>,
    impfiles: Vec<u8>,
    reftab: Vec<u8>,
}

impl Pools {
    fn name(&mut self, s: &str, href: u32) -> u32 {
        if let Some(o) = self.name_map.get(s) {
            return *o;
        }
        let off = self.names.len() as u32;
        put32(&mut self.names, href);
        put32(&mut self.names, 0xffff_ffff);
        put32(&mut self.names, s.len() as u32 | 0x1234_0000);
        self.names.extend_from_slice(s.as_bytes());
        pad4(&mut self.names, 0x57);
        self.name_map.insert(s.to_owned(), off);
        off
    }

    fn string(&mut self, s: &str) -> u32 {
        if let Some(o) = self.string_map.get(s) {
            return *o;
        }
        let off = self.strings.len() as u32;
        put16(&mut self.strings, s.len() as u16);
        self.strings.extend_from_slice(s.as_bytes());
        pad4(&mut self.strings, 0x57);
        while (self.strings.len() as u32 - off) < 8 {
            self.strings.push(0x57);
        }
        self.string_map.insert(s.to_owned(), off);
        off
    }

    fn guid(&mut self, g: [u8; 16]) -> u32 {
        if let Some(o) = self.guid_map.get(&g) {
            return *o;
        }
        let off = self.guids.len() as u32;
        self.guids.extend_from_slice(&g);
        put32(&mut self.guids, 0xffff_ffff);
        put32(&mut self.guids, 0xffff_ffff);
        self.guid_map.insert(g, off);
        off
    }

    fn href(r: R) -> u32 {
        match r {
            R::Local(i) => (i * TI_SIZE) as u32,
            R::Import(i) => (i * 12) as u32 | 1,
        }
    }

    fn dt(&mut self, d: &Dt) -> u32 {
        match d {
            Dt::Prim(v) => 0x8000_0000 | (u32::from(*v) << 16) | u32::from(*v),
            Dt::Ptr(inner) => self.composite(26, inner),
            Dt::Safe(inner) => self.composite(27, inner),
            Dt::CArray(elem, bounds) => {
                let e = self.dt(elem);
                let aoff = self.arraydesc.len() as u32;
                put32(&mut self.arraydesc, e);
                put16(&mut self.arraydesc, bounds.len() as u16);
                put16(&mut self.arraydesc, 0);
                for (n, lb) in bounds {
                    put32(&mut self.arraydesc, *n);
                    put32(&mut self.arraydesc, *lb as u32);
                }
                self.entry(28, 0x7ffe, aoff)
            }
            Dt::User(r) => self.entry(29, 0x7fff, Pools::href(*r)),
        }
    }

    fn composite(&mut self, vt: u16, inner: &Dt) -> u32 {
        let c = self.dt(inner);
        self.entry(vt, 0x7fff, c)
    }

    fn entry(&mut self, vt: u16, tag: u16, data: u32) -> u32 {
        let off = self.typdesc.len() as u32;
        put16(&mut self.typdesc, vt);
        put16(&mut self.typdesc, tag);
        put32(&mut self.typdesc, data);
        off
    }

    fn constant(&mut self, c: &Cv) -> u32 {
        let imm = |vt: u32, raw: u32| 0x8000_0000 | (vt << 26) | (raw & 0x03ff_ffff);
        match c {
            Cv::I4(v) if (-(1 << 25)..(1 << 25)).contains(v) => imm(3, *v as u32),
            Cv::I2(v) => imm(2, *v as i32 as u32),
            Cv::Bool(b) => imm(11, u32::from(*b)),
            Cv::Nothing => imm(9, 0),
            Cv::U4(v) if *v < (1 << 25) => imm(19, *v),
            other => {
                let off = self.custdata.len() as u32;
                match other {
                    Cv::I4(v) => {
                        put16(&mut self.custdata, 3);
                        put32(&mut self.custdata, *v as u32);
                    }
                    Cv::U4(v) => {
                        put16(&mut self.custdata, 19);
                        put32(&mut self.custdata, *v);
                    }
                    Cv::Str(s) => {
                        put16(&mut self.custdata, 8);
                        put32(&mut self.custdata, s.len() as u32);
                        self.custdata.extend_from_slice(s.as_bytes());
                    }
                    Cv::R8(x) => {
                        put16(&mut self.custdata, 5);
                        self.custdata.extend_from_slice(&x.to_bits().to_le_bytes());
                    }
                    _ => unreachable!(),
                }
                pad4(&mut self.custdata, 0x57);
                off
            }
        }
    }
}

/// Builds the bytes of an MSFT type library.
pub fn build(spec: &LibSpec) -> Vec<u8> {
    let mut p = Pools::default();
    let ntypes = spec.types.len();
    // The library name entry carries hreftype -1.
    p.name(&spec.name, 0xffff_ffff);
    let lib_guid = p.guid(spec.guid);
    let lib_doc = spec.doc.as_ref().map_or(0xffff_ffff, |d| p.string(d));
    for imp in &spec.imports {
        let file_off = p.impfiles.len() as u32;
        let g = p.guid(imp.lib_guid);
        put32(&mut p.impfiles, g);
        put32(&mut p.impfiles, 0);
        put32(&mut p.impfiles, 2);
        put16(&mut p.impfiles, ((imp.file.len() as u16) << 2) | 1);
        p.impfiles.extend_from_slice(imp.file.as_bytes());
        pad4(&mut p.impfiles, 0x57);
        let tg = p.guid(imp.type_guid);
        put32(&mut p.impinfo, 0x0001_0000 | (u32::from(imp.kind) << 24));
        put32(&mut p.impinfo, file_off);
        put32(&mut p.impinfo, tg);
    }

    let mut records: Vec<u8> = Vec::new();
    let mut members: Vec<u8> = Vec::new();
    let mut member_offsets: Vec<u32> = Vec::new();
    for (idx, t) in spec.types.iter().enumerate() {
        // Member block.
        let mem_off = if t.funcs.is_empty() && t.vars.is_empty() {
            0xffff_ffffu32
        } else {
            member_offsets.push(members.len() as u32);
            members.len() as u32
        };
        if mem_off != 0xffff_ffff {
            let mut recs = Vec::new();
            let mut ids = Vec::new();
            let mut names = Vec::new();
            let mut offs = Vec::new();
            for (i, f) in t.funcs.iter().enumerate() {
                offs.push(recs.len() as u32);
                let has_def = f.params.iter().any(|a| a.default.is_some());
                let help_words = usize::from(f.help.is_some()) * 2;
                let extra = help_words + if has_def { f.params.len() } else { 0 };
                let rs = 24 + extra * 4 + 12 * f.params.len();
                put16(&mut recs, rs as u16);
                put16(&mut recs, i as u16);
                let ret = p.dt(&f.ret);
                put32(&mut recs, ret);
                put32(&mut recs, f.flags);
                put16(&mut recs, f.vtable_offset);
                put16(&mut recs, 0x40);
                let fk = (match f.kind {
                    FuncKind::Virtual => 0,
                    FuncKind::PureVirtual => 1,
                    FuncKind::NonVirtual => 2,
                    FuncKind::Static => 3,
                    FuncKind::Dispatch => 4,
                }) | (match f.invoke {
                    InvokeKind::Func => 1,
                    InvokeKind::PropertyGet => 2,
                    InvokeKind::PropertyPut => 4,
                    InvokeKind::PropertyPutRef => 8,
                } << 3)
                    | (4 << 8)
                    | ((i as u32) << 16);
                put32(&mut recs, fk);
                put16(&mut recs, f.params.len() as u16);
                put16(&mut recs, f.optional as u16);
                if let Some((ctx, doc)) = &f.help {
                    put32(&mut recs, *ctx);
                    let d = p.string(doc);
                    put32(&mut recs, d);
                }
                if has_def {
                    for a in &f.params {
                        let w = a.default.as_ref().map_or(0xffff_ffff, |c| p.constant(c));
                        put32(&mut recs, w);
                    }
                }
                for a in &f.params {
                    let d = p.dt(&a.ty);
                    put32(&mut recs, d);
                    let n = a.name.as_ref().map_or(0xffff_ffff, |n| p.name(n, 0));
                    put32(&mut recs, n);
                    put32(&mut recs, a.flags);
                }
                ids.push(f.memid as u32);
                names.push(p.name(&f.name, (idx * TI_SIZE) as u32));
            }
            for (i, v) in t.vars.iter().enumerate() {
                offs.push(recs.len() as u32);
                put16(&mut recs, 20);
                put16(&mut recs, (t.funcs.len() + i) as u16);
                let d = p.dt(&v.ty);
                put32(&mut recs, d);
                put32(&mut recs, v.flags);
                let k = match v.kind {
                    VarKind::PerInstance => 0,
                    VarKind::Static => 1,
                    VarKind::Const => 2,
                    VarKind::Dispatch => 3,
                };
                put32(&mut recs, k | (0x24 << 16));
                let w = match &v.value {
                    Some(c) => p.constant(c),
                    None => v.offset,
                };
                put32(&mut recs, w);
                ids.push(v.memid as u32);
                names.push(p.name(&v.name, (idx * TI_SIZE) as u32));
            }
            put32(&mut members, recs.len() as u32);
            members.extend_from_slice(&recs);
            for x in ids.iter().chain(names.iter()).chain(offs.iter()) {
                put32(&mut members, *x);
            }
        }
        // dt1 and implemented types.
        let mut dt1 = 0xffff_ffffu32;
        match t.kind {
            TypeKind::Alias => dt1 = p.dt(t.alias.as_ref().expect("alias target")),
            TypeKind::Module => {
                if let Some(d) = &t.dll {
                    dt1 = p.string(d);
                }
            }
            TypeKind::CoClass => {
                if !t.implements.is_empty() {
                    dt1 = p.reftab.len() as u32;
                    for (i, (r, fl)) in t.implements.iter().enumerate() {
                        put32(&mut p.reftab, Pools::href(*r));
                        put32(&mut p.reftab, *fl);
                        put32(&mut p.reftab, 0xffff_ffff);
                        let next = if i + 1 < t.implements.len() {
                            p.reftab.len() as u32 + 4
                        } else {
                            0xffff_ffff
                        };
                        put32(&mut p.reftab, next);
                    }
                }
            }
            _ => {
                if let Some((r, _)) = t.implements.first() {
                    dt1 = Pools::href(*r);
                }
            }
        }
        let guid_off = t.guid.map_or(0xffff_ffff, |g| p.guid(g));
        let name_off = p.name(&t.name, (idx * TI_SIZE) as u32);
        let doc_off = t.doc.as_ref().map_or(0xffff_ffff, |d| p.string(d));
        let kind = match t.kind {
            TypeKind::Enum => 0u32,
            TypeKind::Record => 1,
            TypeKind::Module => 2,
            TypeKind::Interface => 3,
            TypeKind::Dispatch => 4,
            TypeKind::CoClass => 5,
            TypeKind::Alias => 6,
            TypeKind::Union => 7,
        };
        let mut w = [0u32; 25];
        w[0] = kind | (4 << 11) | ((idx as u32) << 16);
        w[1] = mem_off; // patched to an absolute offset below
        w[4] = 3;
        w[6] = t.funcs.len() as u32 | ((t.vars.len() as u32) << 16);
        w[11] = guid_off;
        w[12] = t.flags;
        w[13] = name_off;
        w[14] = 0;
        w[15] = doc_off;
        w[17] = 0;
        w[18] = 0xffff_ffff;
        w[19] = t.implements.len() as u32 | (u32::from(t.vft) << 16);
        w[20] = t.size;
        w[21] = dt1;
        w[24] = 0xffff_ffff;
        for x in w {
            put32(&mut records, x);
        }
    }

    // Assemble.
    let mut head: Vec<u8> = Vec::new();
    head.extend_from_slice(b"MSFT");
    put32(&mut head, 0x0001_0002);
    put32(&mut head, lib_guid);
    put32(&mut head, spec.lcid);
    put32(&mut head, 0);
    put32(&mut head, 0x40 | spec.sys_kind);
    put32(
        &mut head,
        u32::from(spec.version.0) | (u32::from(spec.version.1) << 16),
    );
    put32(&mut head, spec.flags);
    put32(&mut head, ntypes as u32);
    put32(&mut head, lib_doc);
    for v in [0u32, 0, 0, 0, 0, 0xffff_ffff, 0x18, 0x20, 0x80, 1] {
        put32(&mut head, v);
    }
    put32(&mut head, spec.imports.len() as u32);
    assert_eq!(head.len(), 0x54);
    for i in 0..ntypes {
        put32(&mut head, (i * TI_SIZE) as u32);
    }
    let seg_dir_len = 15 * 16;
    let base = head.len() + seg_dir_len;
    let guid_hash = vec![0xffu8; 128];
    let name_hash = vec![0xffu8; 512];
    let segs: [&Vec<u8>; 13] = [
        &records,
        &p.impinfo,
        &p.impfiles,
        &p.reftab,
        &guid_hash,
        &p.guids,
        &name_hash,
        &p.names,
        &p.strings,
        &p.typdesc,
        &p.arraydesc,
        &p.custdata,
        &Vec::new(),
    ];
    let mut offsets = Vec::new();
    let mut cur = base;
    for s in segs {
        offsets.push(cur);
        cur += s.len();
    }
    let members_base = cur;
    // Patch member offsets (absolute file offsets).
    let ti_count = ntypes;
    let mut rec_patched = records.clone();
    for i in 0..ti_count {
        let o = i * TI_SIZE + 4;
        let v = u32::from_le_bytes(rec_patched[o..o + 4].try_into().unwrap());
        if v != 0xffff_ffff {
            rec_patched[o..o + 4].copy_from_slice(&((members_base as u32) + v).to_le_bytes());
        }
    }
    let mut out = head;
    for (i, s) in segs.iter().enumerate() {
        let (off, len) = if s.is_empty() && i != 0 {
            (0xffff_ffffu32, 0u32)
        } else {
            (offsets[i] as u32, s.len() as u32)
        };
        put32(&mut out, off);
        put32(&mut out, len);
        put32(&mut out, 0xffff_ffff);
        put32(&mut out, 0xf);
    }
    for _ in 0..2 {
        put32(&mut out, 0xffff_ffff);
        put32(&mut out, 0);
        put32(&mut out, 0xffff_ffff);
        put32(&mut out, 0xf);
    }
    out.extend_from_slice(&rec_patched);
    for s in &segs[1..] {
        out.extend_from_slice(s);
    }
    out.extend_from_slice(&members);
    out
}
