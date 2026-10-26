//! A minimal PE writer (one `.rsrc` section) for tests.

fn put16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn put32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

/// A resource: `(type name or id, resource id, language, bytes)`.
pub struct Res {
    pub ty: Ty,
    pub id: u16,
    pub lang: u16,
    pub data: Vec<u8>,
}

pub enum Ty {
    Name(&'static str),
    Id(u16),
}

/// Builds a PE image with the given resources. `pe64` selects PE32+.
pub fn build_pe(pe64: bool, resources: &[Res]) -> Vec<u8> {
    // Resource section layout: root, per-type dirs, per-id dirs, data
    // entries, type names, data.
    let mut types: Vec<(&Ty, Vec<&Res>)> = Vec::new();
    for r in resources {
        let same = |t: &Ty| match (t, &r.ty) {
            (Ty::Name(a), Ty::Name(b)) => a == b,
            (Ty::Id(a), Ty::Id(b)) => a == b,
            _ => false,
        };
        if let Some(e) = types.iter_mut().find(|(t, _)| same(t)) {
            e.1.push(r);
        } else {
            types.push((&r.ty, vec![r]));
        }
    }
    let mut sec: Vec<u8> = Vec::new();
    // Compute sizes first.
    let root_size = 16 + 8 * types.len();
    let mut cursor = root_size;
    let mut type_dir_off = Vec::new();
    for (_, rs) in &types {
        type_dir_off.push(cursor);
        cursor += 16 + 8 * rs.len();
    }
    let mut id_dir_off = Vec::new();
    for (_, rs) in &types {
        for _ in rs {
            id_dir_off.push(cursor);
            cursor += 16 + 8;
        }
    }
    let mut entry_off = Vec::new();
    for _ in resources {
        entry_off.push(cursor);
        cursor += 16;
    }
    let mut name_off = Vec::new();
    for (t, _) in &types {
        match t {
            Ty::Name(n) => {
                name_off.push(Some(cursor));
                cursor += 2 + 2 * n.len();
                cursor = (cursor + 3) & !3;
            }
            Ty::Id(_) => name_off.push(None),
        }
    }
    let mut data_off = Vec::new();
    for r in resources {
        cursor = (cursor + 3) & !3;
        data_off.push(cursor);
        cursor += r.data.len();
    }
    let rva = 0x1000u32;
    // Root.
    sec.extend_from_slice(&[0; 12]);
    let named = types
        .iter()
        .filter(|(t, _)| matches!(t, Ty::Name(_)))
        .count() as u16;
    put16(&mut sec, named);
    put16(&mut sec, types.len() as u16 - named);
    // Named entries must come first; keep input order stable by sorting.
    let mut order: Vec<usize> = (0..types.len()).collect();
    order.sort_by_key(|i| !matches!(types[*i].0, Ty::Name(_)));
    for i in &order {
        match types[*i].0 {
            Ty::Name(_) => put32(&mut sec, 0x8000_0000 | name_off[*i].unwrap() as u32),
            Ty::Id(id) => put32(&mut sec, u32::from(*id)),
        }
        put32(&mut sec, 0x8000_0000 | type_dir_off[*i] as u32);
    }
    // Per-type directories.
    let mut flat = 0usize;
    let mut id_index = Vec::new();
    for (_, rs) in &types {
        let mut v = Vec::new();
        for _ in rs {
            v.push(flat);
            flat += 1;
        }
        id_index.push(v);
    }
    for (ti, (_, rs)) in types.iter().enumerate() {
        sec.extend_from_slice(&[0; 12]);
        put16(&mut sec, 0);
        put16(&mut sec, rs.len() as u16);
        for (k, r) in rs.iter().enumerate() {
            put32(&mut sec, u32::from(r.id));
            put32(&mut sec, 0x8000_0000 | id_dir_off[id_index[ti][k]] as u32);
        }
    }
    // Per-id directories (language level).
    let mut flat_res: Vec<&Res> = Vec::new();
    for (_, rs) in &types {
        for r in rs {
            flat_res.push(r);
        }
    }
    for (n, r) in flat_res.iter().enumerate() {
        sec.extend_from_slice(&[0; 12]);
        put16(&mut sec, 0);
        put16(&mut sec, 1);
        put32(&mut sec, u32::from(r.lang));
        let orig = resources
            .iter()
            .position(|x| std::ptr::eq(x, *r))
            .expect("resource");
        let _ = n;
        put32(&mut sec, entry_off[orig] as u32);
    }
    // Data entries (in input order).
    for (k, r) in resources.iter().enumerate() {
        put32(&mut sec, rva + data_off[k] as u32);
        put32(&mut sec, r.data.len() as u32);
        put32(&mut sec, 0);
        put32(&mut sec, 0);
    }
    // Names.
    for (i, (t, _)) in types.iter().enumerate() {
        if let Ty::Name(n) = t {
            assert_eq!(sec.len(), name_off[i].unwrap());
            put16(&mut sec, n.len() as u16);
            for c in n.encode_utf16() {
                put16(&mut sec, c);
            }
            while !sec.len().is_multiple_of(4) {
                sec.push(0);
            }
        }
    }
    for (k, r) in resources.iter().enumerate() {
        while !sec.len().is_multiple_of(4) {
            sec.push(0);
        }
        assert_eq!(sec.len(), data_off[k]);
        sec.extend_from_slice(&r.data);
    }
    let raw_size = (sec.len() + 0x1ff) & !0x1ff;
    let virt_size = sec.len() as u32;

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"MZ");
    out.resize(0x3c, 0);
    put32(&mut out, 0x40);
    out.extend_from_slice(b"PE\0\0");
    put16(&mut out, if pe64 { 0x8664 } else { 0x14c });
    put16(&mut out, 1);
    put32(&mut out, 0);
    put32(&mut out, 0);
    put32(&mut out, 0);
    let opt_size: u16 = if pe64 { 240 } else { 224 };
    put16(&mut out, opt_size);
    put16(&mut out, if pe64 { 0x2022 } else { 0x2102 });
    // Optional header.
    put16(&mut out, if pe64 { 0x20b } else { 0x10b });
    out.extend_from_slice(&[14, 0]);
    put32(&mut out, 0); // size of code
    put32(&mut out, raw_size as u32); // initialized data
    put32(&mut out, 0);
    put32(&mut out, 0); // entry point
    put32(&mut out, 0x1000); // base of code
    if pe64 {
        out.extend_from_slice(&0x1_8000_0000u64.to_le_bytes());
    } else {
        put32(&mut out, 0x1000); // base of data
        put32(&mut out, 0x1000_0000);
    }
    put32(&mut out, 0x1000); // section alignment
    put32(&mut out, 0x200); // file alignment
    for v in [6u16, 0, 0, 0, 6, 0] {
        put16(&mut out, v);
    }
    put32(&mut out, 0); // win32 version
    put32(&mut out, 0x1000 + ((virt_size + 0xfff) & !0xfff)); // size of image
    put32(&mut out, 0x200); // size of headers
    put32(&mut out, 0); // checksum
    put16(&mut out, 3);
    put16(&mut out, 0);
    if pe64 {
        for v in [0x10_0000u64, 0x1000, 0x10_0000, 0x1000] {
            out.extend_from_slice(&v.to_le_bytes());
        }
    } else {
        for v in [0x10_0000u32, 0x1000, 0x10_0000, 0x1000] {
            put32(&mut out, v);
        }
    }
    put32(&mut out, 0); // loader flags
    put32(&mut out, 16);
    for i in 0..16 {
        if i == 2 {
            put32(&mut out, rva);
            put32(&mut out, virt_size);
        } else {
            put32(&mut out, 0);
            put32(&mut out, 0);
        }
    }
    // Section header.
    out.extend_from_slice(b".rsrc\0\0\0");
    put32(&mut out, virt_size);
    put32(&mut out, rva);
    put32(&mut out, raw_size as u32);
    put32(&mut out, 0x200);
    put32(&mut out, 0);
    put32(&mut out, 0);
    put16(&mut out, 0);
    put16(&mut out, 0);
    put32(&mut out, 0x4000_0040);
    out.resize(0x200, 0);
    out.extend_from_slice(&sec);
    out.resize(0x200 + raw_size, 0);
    out
}
