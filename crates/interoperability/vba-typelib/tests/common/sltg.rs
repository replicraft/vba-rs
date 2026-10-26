//! A writer for synthetic SLTG libraries that follows `docs/formats/typelib.md`.
#![allow(dead_code, missing_docs)]

pub const KIND_ENUM: u16 = 0;
pub const KIND_RECORD: u16 = 1;
pub const KIND_MODULE: u16 = 2;
pub const KIND_INTERFACE: u16 = 3;
pub const KIND_DISPATCH: u16 = 4;
pub const KIND_COCLASS: u16 = 5;
pub const KIND_UNION: u16 = 7;

/// A type description to encode.
#[derive(Clone, Debug)]
pub enum Ty {
    Prim(u16),
    Ptr(Box<Ty>),
    SafeArray(Box<Ty>),
    /// A type reference: index into the owning type's `refs`.
    User(usize),
    CArray(Vec<(u32, i32)>, Box<Ty>),
}

pub fn ptr(t: Ty) -> Ty {
    Ty::Ptr(Box::new(t))
}

pub struct SParam {
    pub name: Option<String>,
    pub ty: Ty,
    /// Flag bits OR-ed into the first type word (0x4000 out, 0x2000 lcid, ...).
    pub bits: u16,
}

pub struct SFunc {
    pub name: String,
    /// 1 func, 2 get, 4 put, 8 putref.
    pub invoke: u8,
    pub dispid: i32,
    pub ret: Ty,
    pub params: Vec<SParam>,
    pub vtbl: u16,
    /// `None` writes the shorter record without function flags.
    pub flags: Option<u16>,
}

pub struct SVar {
    pub name: String,
    pub memid: i32,
    pub ty: Ty,
    pub flags: Option<u16>,
    pub byte_offs: u16,
    /// A constant with its value stored in `byte_offs`.
    pub konst: bool,
}

pub enum SRef {
    Local(usize),
    /// A type of another library: name table entry of the library, type number.
    Import(usize, usize),
}

pub struct SImport {
    pub guid: String,
    pub version: (u16, u16),
    pub lcid: u32,
    pub file: String,
}

pub struct SType {
    pub name: String,
    pub kind: u16,
    pub guid: [u8; 16],
    pub flags: u16,
    pub version: (u16, u16),
    pub refs: Vec<SRef>,
    /// `(index into refs, IMPLTYPEFLAG_*)`.
    pub impls: Vec<(usize, u8)>,
    pub funcs: Vec<SFunc>,
    pub vars: Vec<SVar>,
    pub size: u16,
    pub align: u16,
    pub vft: u16,
}

impl SType {
    pub fn new(name: &str, kind: u16) -> SType {
        SType {
            name: name.to_owned(),
            kind,
            guid: [0; 16],
            flags: 0,
            version: (0, 0),
            refs: Vec::new(),
            impls: Vec::new(),
            funcs: Vec::new(),
            vars: Vec::new(),
            size: 4,
            align: 4,
            vft: 0,
        }
    }
}

pub struct SLib {
    pub name: String,
    pub doc: Option<String>,
    pub help_context: u32,
    pub guid: [u8; 16],
    pub lcid: u16,
    pub sys_kind: u16,
    pub flags: u16,
    pub version: (u16, u16),
    pub imports: Vec<SImport>,
    pub types: Vec<SType>,
    /// Physical order of the blocks: type indices, the `dir` block being
    /// `types.len()`. `None` is the usual order, `dir` last.
    pub order: Option<Vec<usize>>,
}

impl SLib {
    pub fn new(name: &str) -> SLib {
        SLib {
            name: name.to_owned(),
            doc: None,
            help_context: 0,
            guid: [0x11; 16],
            lcid: 0x409,
            sys_kind: 1,
            flags: 0,
            version: (1, 0),
            imports: Vec::new(),
            types: Vec::new(),
            order: None,
        }
    }
}

struct Names {
    buf: Vec<u8>,
}

impl Names {
    /// Adds an entry (eight hash bytes, the text, a NUL, padded to an even
    /// length) and returns its offset.
    fn add(&mut self, s: &str) -> u16 {
        let off = self.buf.len();
        self.buf.extend_from_slice(&[0xff; 8]);
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.push(0);
        if !(self.buf.len() - off).is_multiple_of(2) {
            self.buf.push(0);
        }
        u16::try_from(off).unwrap()
    }
}

fn p16(b: &mut Vec<u8>, v: u16) {
    b.extend_from_slice(&v.to_le_bytes());
}

fn p32(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}

fn pstr(b: &mut Vec<u8>, s: Option<&str>) {
    match s {
        None => p16(b, 0xffff),
        Some(s) => {
            p16(b, u16::try_from(s.len()).unwrap());
            b.extend_from_slice(s.as_bytes());
        }
    }
}

/// Encodes `t` as type words. A fixed array's `SAFEARRAY` header is
/// appended to `buf` (whose first byte is at group offset `base`).
fn words(t: &Ty, buf: &mut Vec<u8>, base: usize) -> Vec<u16> {
    match t {
        Ty::Prim(v) => vec![*v],
        Ty::Ptr(inner) => match &**inner {
            Ty::Prim(v) => vec![0x0e00 | *v],
            Ty::User(i) => vec![0x0e1d, u16::try_from(4 * i).unwrap()],
            other => {
                let mut w = vec![26];
                w.extend(words(other, buf, base));
                w
            }
        },
        Ty::SafeArray(inner) => {
            let mut w = vec![27, 0];
            w.extend(words(inner, buf, base));
            w
        }
        Ty::User(i) => vec![29, u16::try_from(4 * i).unwrap()],
        Ty::CArray(bounds, elem) => {
            let off = base + buf.len();
            p16(buf, u16::try_from(bounds.len()).unwrap());
            p16(buf, 0);
            p32(buf, 1);
            p32(buf, 0);
            p32(buf, 0);
            for (n, lo) in bounds {
                p32(buf, *n);
                p32(buf, *lo as u32);
            }
            let mut w = vec![28, u16::try_from(off).unwrap()];
            w.extend(words(elem, buf, base));
            w
        }
    }
}

fn put_words(buf: &mut Vec<u8>, w: &[u16]) {
    for x in w {
        p16(buf, *x);
    }
}

fn func_record(f: &SFunc, base: usize, last: bool, nt: &mut Names) -> Vec<u8> {
    let fixed = if f.flags.is_some() { 24 } else { 22 };
    let n = f.params.len();
    let mut rec = vec![0u8; fixed + 4 * n];
    rec[0] = if f.flags.is_some() { 0x6c } else { 0x4c };
    rec[1] = (f.invoke << 4) | 2;
    let name = nt.add(&f.name);
    rec[4..6].copy_from_slice(&name.to_le_bytes());
    rec[6..10].copy_from_slice(&f.dispid.to_le_bytes());
    rec[10..12].copy_from_slice(&0xfffeu16.to_le_bytes());
    rec[12..14].copy_from_slice(&0xffffu16.to_le_bytes());
    let arg_off = if n == 0 { 0xffff } else { base + fixed };
    rec[14..16].copy_from_slice(&u16::try_from(arg_off).unwrap().to_le_bytes());
    rec[16] = u8::try_from((n << 3) | 4).unwrap();
    rec[20..22].copy_from_slice(&f.vtbl.to_le_bytes());
    if let Some(fl) = f.flags {
        rec[22..24].copy_from_slice(&fl.to_le_bytes());
    }
    for (i, p) in f.params.iter().enumerate() {
        let mut w = words(&p.ty, &mut rec, base);
        w[0] |= p.bits;
        let name = p.name.as_deref().map_or(0xfffe, |s| nt.add(s));
        let (raw, tw) = if w.len() == 1 {
            (name | 1, w[0])
        } else {
            let off = u16::try_from(base + rec.len()).unwrap();
            put_words(&mut rec, &w);
            (name, off)
        };
        let a = fixed + 4 * i;
        rec[a..a + 2].copy_from_slice(&raw.to_le_bytes());
        rec[a + 2..a + 4].copy_from_slice(&tw.to_le_bytes());
    }
    let w = words(&f.ret, &mut rec, base);
    if w.len() == 1 {
        rec[17] = 0x80;
        rec[18..20].copy_from_slice(&w[0].to_le_bytes());
    } else {
        let off = u16::try_from(base + rec.len()).unwrap();
        put_words(&mut rec, &w);
        rec[17] = 0;
        rec[18..20].copy_from_slice(&off.to_le_bytes());
    }
    let next = if last {
        0xffff
    } else {
        u16::try_from(base + rec.len()).unwrap()
    };
    rec[2..4].copy_from_slice(&next.to_le_bytes());
    rec
}

fn type_block(t: &SType, index: usize, imports: &[u16], nt: &mut Names) -> Vec<u8> {
    // Member data.
    let mut group = Vec::new();
    let impls_off = if t.impls.is_empty() { 0xffff } else { 0 };
    for (k, (r, fl)) in t.impls.iter().enumerate() {
        let mut rec = vec![0u8; 22];
        rec[0] = 0x4a;
        let next = if k + 1 == t.impls.len() {
            0xffff
        } else {
            u16::try_from(group.len() + 22).unwrap()
        };
        rec[2..4].copy_from_slice(&next.to_le_bytes());
        rec[4..6].copy_from_slice(&0xffffu16.to_le_bytes());
        rec[6] = *fl;
        rec[7] = 0x80;
        rec[8] = 0x12;
        rec[12..14].copy_from_slice(&0x4001u16.to_le_bytes());
        rec[14..16].copy_from_slice(&0xfffeu16.to_le_bytes());
        rec[16..18].copy_from_slice(&0xffffu16.to_le_bytes());
        rec[18..20].copy_from_slice(&29u16.to_le_bytes());
        rec[20..22].copy_from_slice(&u16::try_from(4 * r).unwrap().to_le_bytes());
        group.extend_from_slice(&rec);
    }
    let funcs_off = if t.funcs.is_empty() {
        0xffff
    } else {
        u16::try_from(group.len()).unwrap()
    };
    for (i, f) in t.funcs.iter().enumerate() {
        let rec = func_record(f, group.len(), i + 1 == t.funcs.len(), nt);
        group.extend_from_slice(&rec);
    }
    let vars_off = if t.vars.is_empty() {
        0xffff
    } else {
        u16::try_from(group.len()).unwrap()
    };
    let vbase = group.len();
    let mut vrecs = Vec::new();
    let mut vdata = Vec::new();
    let sizes: Vec<usize> = t
        .vars
        .iter()
        .map(|v| if v.flags.is_some() { 0x14 } else { 0x12 })
        .collect();
    let dbase = vbase + sizes.iter().sum::<usize>();
    let mut pos = vbase;
    for (i, v) in t.vars.iter().enumerate() {
        let size = sizes[i];
        let mut rec = vec![0u8; size];
        rec[0] = if v.flags.is_some() { 0x2a } else { 0x0a };
        let mut fl = 0u8;
        let mut w = words(&v.ty, &mut vdata, dbase);
        let tw = if w.len() == 1 {
            fl |= 0x02;
            w.remove(0)
        } else {
            let off = u16::try_from(dbase + vdata.len()).unwrap();
            put_words(&mut vdata, &w);
            off
        };
        if v.konst {
            fl |= 0x40 | 0x08;
        }
        rec[1] = fl;
        let next = if i + 1 == t.vars.len() {
            0xffff
        } else {
            u16::try_from(pos + size).unwrap()
        };
        rec[2..4].copy_from_slice(&next.to_le_bytes());
        let name = nt.add(&v.name);
        rec[4..6].copy_from_slice(&name.to_le_bytes());
        rec[6..8].copy_from_slice(&v.byte_offs.to_le_bytes());
        rec[8..10].copy_from_slice(&tw.to_le_bytes());
        rec[10..14].copy_from_slice(&v.memid.to_le_bytes());
        rec[14..16].copy_from_slice(&0xfffeu16.to_le_bytes());
        rec[16..18].copy_from_slice(&0xffffu16.to_le_bytes());
        if let Some(f) = v.flags {
            rec[18..20].copy_from_slice(&f.to_le_bytes());
        }
        pos += size;
        vrecs.extend_from_slice(&rec);
    }
    group.extend_from_slice(&vrecs);
    group.extend_from_slice(&vdata);

    // Reference table.
    let mut refs = Vec::new();
    if !t.refs.is_empty() {
        p16(&mut refs, 0x00df);
        refs.extend_from_slice(&[0xff; 66]);
        let number = t.refs.len() * 8;
        p32(&mut refs, u32::try_from(number).unwrap());
        refs.extend(std::iter::repeat_n(0xff, number + 2));
        refs.extend_from_slice(&[1, 0, 0, 0, 0]);
        for r in &t.refs {
            let s = match r {
                SRef::Local(n) => format!("*\\Rffff*#{n:x}"),
                SRef::Import(lib, n) => format!("*\\R{:04x}*#{n:x}", imports[*lib]),
            };
            pstr(&mut refs, Some(&s));
        }
        refs.push(0xdf);
    }

    let mut b = Vec::new();
    p16(&mut b, 0x0501);
    p32(&mut b, if refs.is_empty() { u32::MAX } else { 0x22 });
    p32(&mut b, u32::MAX);
    p32(&mut b, u32::try_from(0x22 + refs.len()).unwrap());
    p32(&mut b, u32::MAX);
    p32(
        &mut b,
        u32::from(t.version.0) | (u32::from(t.version.1) << 16),
    );
    p32(&mut b, 0xfffe_0000);
    b.push(((t.flags & 0x1f) << 3) as u8 | 2);
    b.push((t.flags >> 5) as u8);
    b.push(2);
    b.push(u8::try_from(t.kind).unwrap());
    p32(&mut b, 0);
    assert_eq!(b.len(), 0x22);
    b.extend_from_slice(&refs);
    // Member header.
    p16(&mut b, 1);
    p16(&mut b, 0xffff);
    b.push(1);
    p32(&mut b, u32::try_from(group.len()).unwrap());
    b.extend_from_slice(&group);
    // Tail.
    let mut tail = [0xffu8; 0x36];
    let mut set = |o: usize, v: u16| tail[o..o + 2].copy_from_slice(&v.to_le_bytes());
    set(0, u16::try_from(t.funcs.len()).unwrap());
    set(2, u16::try_from(t.vars.len()).unwrap());
    set(4, u16::try_from(t.impls.len()).unwrap());
    set(0x08, funcs_off);
    set(0x0a, vars_off);
    set(0x0c, impls_off);
    set(0x20, t.size);
    set(0x22, t.align);
    set(0x28, t.vft);
    set(0x34, u16::try_from(group.len()).unwrap());
    b.extend_from_slice(&tail);
    let _ = index;
    b
}

/// Builds an SLTG library.
pub fn build_sltg(lib: &SLib) -> Vec<u8> {
    let mut nt = Names { buf: Vec::new() };
    let lib_name = nt.add(&lib.name);
    let type_names: Vec<u16> = lib.types.iter().map(|t| nt.add(&t.name)).collect();
    let imports: Vec<u16> = lib
        .imports
        .iter()
        .map(|i| {
            nt.add(&format!(
                "*\\G{{{}}}#{}.{}#{:x}#{}#",
                i.guid, i.version.0, i.version.1, i.lcid, i.file
            ))
        })
        .collect();
    let blocks: Vec<Vec<u8>> = lib
        .types
        .iter()
        .enumerate()
        .map(|(i, t)| type_block(t, i, &imports, &mut nt))
        .collect();

    // The dir block.
    let mut dir = Vec::new();
    p16(&mut dir, 0x51cc);
    p16(&mut dir, 3);
    p16(&mut dir, lib_name);
    p16(&mut dir, 0xffff);
    pstr(&mut dir, lib.doc.as_deref());
    pstr(&mut dir, None);
    p32(&mut dir, lib.help_context);
    p16(&mut dir, lib.sys_kind);
    p16(&mut dir, lib.lcid);
    p32(&mut dir, 0);
    p16(&mut dir, lib.flags);
    p16(&mut dir, lib.version.0);
    p16(&mut dir, lib.version.1);
    dir.extend_from_slice(&lib.guid);
    dir.extend_from_slice(&[0xff; 0x40]);
    p16(&mut dir, u16::try_from(lib.types.len()).unwrap());
    let streams: Vec<String> = (0..lib.types.len()).map(|i| format!("B{i:09}")).collect();
    for (i, t) in lib.types.iter().enumerate() {
        pstr(&mut dir, Some(&streams[i]));
        pstr(&mut dir, Some(&format!("C{i:09}")));
        p16(&mut dir, 0xffff);
        p16(&mut dir, type_names[i]);
        p16(&mut dir, 0);
        p16(&mut dir, 0xffff);
        p32(&mut dir, 0);
        p16(&mut dir, 0xffff);
        dir.extend_from_slice(&t.guid);
        p16(&mut dir, t.kind);
    }
    dir.extend_from_slice(&[0; 0x16]);
    dir.extend_from_slice(&[0xff; 0x200]);
    p16(&mut dir, u16::try_from(nt.buf.len()).unwrap());
    p16(&mut dir, 0);
    dir.extend_from_slice(&nt.buf);
    dir.extend_from_slice(&[1, 0xff, 0xff, 1, 0, 0, 0, 0]);

    // File layout.
    let nt_ = lib.types.len();
    let mut all_blocks = blocks;
    all_blocks.push(dir);
    let n = nt_ + 2;
    let mut index = Vec::new();
    index.extend_from_slice(b"\x01CompObj\0");
    let dir_name = index.len();
    index.extend_from_slice(b"dir\0");
    let mut stream_off = Vec::new();
    for s in &streams {
        stream_off.push(index.len());
        index.extend_from_slice(s.as_bytes());
        index.push(0);
    }
    let order: Vec<usize> = lib.order.clone().unwrap_or_else(|| (0..=nt_).collect());
    let mut next = vec![0u16; nt_ + 1];
    for (k, &e) in order.iter().enumerate() {
        next[e] = order
            .get(k + 1)
            .map_or(0, |&m| u16::try_from(m + 1).unwrap());
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"SLTG");
    p16(&mut out, u16::try_from(n).unwrap());
    p16(&mut out, 9);
    p16(&mut out, 0x4f);
    p16(&mut out, u16::try_from(order[0] + 1).unwrap());
    out.extend_from_slice(&[
        0xff, 0x04, 0x02, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46,
    ]);
    p32(&mut out, 0x44);
    p32(&mut out, 0xffff_0000);
    assert_eq!(out.len(), 0x24);
    for e in 0..=nt_ {
        p32(&mut out, u32::try_from(all_blocks[e].len()).unwrap());
        let s = if e == nt_ { dir_name } else { stream_off[e] };
        p16(&mut out, u16::try_from(s).unwrap());
        p16(&mut out, next[e]);
    }
    out.extend_from_slice(&index);
    out.extend_from_slice(&[0; 9]);
    for &e in &order {
        out.extend_from_slice(&all_blocks[e]);
    }
    out
}
