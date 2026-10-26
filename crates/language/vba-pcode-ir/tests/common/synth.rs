//! A tiny assembler for hand-built VBA 7.0 x86 caches. The layouts follow
//! the repository documents; nothing here is derived from any real project.

#![allow(dead_code, missing_docs)]

use vba_pcode::Opcode;

/// Intrinsic type ids as stored in type fields.
pub const T_INTEGER: u8 = 2;
pub const T_LONG: u8 = 3;
pub const T_DOUBLE: u8 = 5;
pub const T_DATE: u8 = 7;
pub const T_STRING: u8 = 8;
pub const T_BOOLEAN: u8 = 11;
pub const T_VARIANT: u8 = 12;

/// Stored (x86 VBA7) number of a canonical opcode.
pub fn stored(c: u16) -> u16 {
    match c {
        0..=173 => c,
        175 | 176 => c - 1,
        178..=180 => c - 2,
        _ => c - 3,
    }
}

/// Builder for the identifier table and the indirect table.
pub struct Tables {
    pub idents: Vec<String>,
    pub ind: Vec<u8>,
}

impl Default for Tables {
    /// The table starts with 8 bytes of padding: offset 0 never names an
    /// argument record (a next-argument field of 0 means "none").
    fn default() -> Self {
        Self {
            idents: Vec::new(),
            ind: vec![0; 8],
        }
    }
}

impl Tables {
    /// The name word of a user identifier (added on first use).
    pub fn name(&mut self, s: &str) -> u16 {
        let i = match self.idents.iter().position(|x| x == s) {
            Some(i) => i,
            None => {
                self.idents.push(s.to_owned());
                self.idents.len() - 1
            }
        };
        (i as u16 + 4 + 0x100) << 1
    }

    fn pad8(&mut self) {
        while !self.ind.len().is_multiple_of(8) {
            self.ind.push(0);
        }
    }

    fn type_field(id: Option<u8>) -> u32 {
        match id {
            Some(t) => 0xFFFF_0000 | u32::from(t),
            None => u32::MAX,
        }
    }

    /// Adds a function record with its argument chain; returns its offset.
    /// `args` are `(name, type id, by_val)`.
    pub fn function(&mut self, name: &str, ret: Option<u8>, args: &[(&str, u8, bool)]) -> u32 {
        // Arguments first (they are chained by absolute offset).
        let mut arg_offs = Vec::new();
        for (n, t, by_val) in args {
            self.pad8();
            let off = self.ind.len() as u32;
            let nw = self.name(n);
            let mut r = vec![0u8; 32];
            r[0..2].copy_from_slice(&0x8369u16.to_le_bytes());
            r[2..4].copy_from_slice(&nw.to_le_bytes());
            r[12..16].copy_from_slice(&Self::type_field(Some(*t)).to_le_bytes());
            r[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
            let opt: u16 = if *by_val { 0x4 } else { 0x2 };
            r[24..26].copy_from_slice(&opt.to_le_bytes());
            self.ind.extend_from_slice(&r);
            arg_offs.push(off);
        }
        for k in 0..arg_offs.len().saturating_sub(1) {
            let at = arg_offs[k] as usize + 20;
            let next = arg_offs[k + 1];
            self.ind[at..at + 4].copy_from_slice(&next.to_le_bytes());
        }
        self.pad8();
        let off = self.ind.len() as u32;
        let nw = self.name(name);
        let mut r = vec![0u8; 72];
        let flags: u16 = 0x1000 | if ret.is_some() { 0x20 } else { 0 };
        r[0..2].copy_from_slice(&flags.to_le_bytes());
        r[2..4].copy_from_slice(&nw.to_le_bytes());
        let first = arg_offs.first().copied().unwrap_or(u32::MAX);
        r[40..44].copy_from_slice(&first.to_le_bytes());
        r[44..48].copy_from_slice(&Self::type_field(ret).to_le_bytes());
        r[48..50].copy_from_slice(&0xFFFFu16.to_le_bytes());
        r[58] = 0x8C;
        r[59] = args.len() as u8;
        r[61] = 0x02;
        self.ind.extend_from_slice(&r);
        off
    }

    /// Marks the `index`-th argument of the function record at `func` as
    /// `Optional`, with a default value when `default` is set.
    pub fn mark_optional(&mut self, func: u32, index: usize, default: bool) {
        let f = func as usize;
        let mut at = u32::from_le_bytes(self.ind[f + 40..f + 44].try_into().unwrap()) as usize;
        for _ in 0..index {
            at = u32::from_le_bytes(self.ind[at + 20..at + 24].try_into().unwrap()) as usize;
        }
        let mut opt = u16::from_le_bytes(self.ind[at + 24..at + 26].try_into().unwrap());
        opt |= 0x200 | if default { 0x400 } else { 0 };
        self.ind[at + 24..at + 26].copy_from_slice(&opt.to_le_bytes());
    }

    /// Adds a variable record; returns its offset.
    pub fn variable(&mut self, name: &str, ty: Option<u8>) -> u32 {
        self.pad8();
        let off = self.ind.len() as u32;
        let nw = self.name(name);
        let mut r = vec![0u8; 24];
        r[0] = if ty.is_some() { 0x20 } else { 0 };
        r[2..4].copy_from_slice(&nw.to_le_bytes());
        r[12..16].copy_from_slice(&Self::type_field(ty).to_le_bytes());
        self.ind.extend_from_slice(&r);
        off
    }
}

/// Instruction-stream builder for one line.
#[derive(Default, Clone)]
pub struct Line(pub Vec<u8>);

impl Line {
    pub fn new() -> Self {
        Self::default()
    }

    fn word(&mut self, op: Opcode, modifier: u16) -> &mut Self {
        let w = stored(op.canonical()) | (modifier << 10);
        self.0.extend_from_slice(&w.to_le_bytes());
        self
    }

    pub fn op(mut self, op: Opcode) -> Self {
        self.word(op, 0);
        self
    }

    pub fn opm(mut self, op: Opcode, modifier: u16) -> Self {
        self.word(op, modifier);
        self
    }

    pub fn name(mut self, op: Opcode, name: u16) -> Self {
        self.word(op, 0);
        self.0.extend_from_slice(&name.to_le_bytes());
        self
    }

    pub fn name_args(mut self, op: Opcode, modifier: u16, name: u16, argc: u16) -> Self {
        self.word(op, modifier);
        self.0.extend_from_slice(&name.to_le_bytes());
        self.0.extend_from_slice(&argc.to_le_bytes());
        self
    }

    pub fn rec(mut self, op: Opcode, modifier: u16, off: u32) -> Self {
        self.word(op, modifier);
        self.0.extend_from_slice(&off.to_le_bytes());
        self
    }

    pub fn i16(mut self, v: i16) -> Self {
        self.word(Opcode::ConstInt16Decimal, 0);
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn i32(mut self, v: i32) -> Self {
        self.word(Opcode::ConstInt32Decimal, 0);
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn date(mut self, serial: f64) -> Self {
        self.word(Opcode::ConstDate, 0);
        self.0.extend_from_slice(&serial.to_bits().to_le_bytes());
        self
    }

    pub fn hex16(mut self, v: u16) -> Self {
        self.word(Opcode::ConstInt16Hex, 0);
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn hex32(mut self, v: u32) -> Self {
        self.word(Opcode::ConstInt32Hex, 0);
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn f64(mut self, v: f64) -> Self {
        self.word(Opcode::ConstFloat64, 0);
        self.0.extend_from_slice(&v.to_bits().to_le_bytes());
        self
    }

    pub fn text(mut self, s: &str) -> Self {
        self.word(Opcode::ConstText, 0);
        self.0.extend_from_slice(&(s.len() as u16).to_le_bytes());
        self.0.extend_from_slice(s.as_bytes());
        if !s.len().is_multiple_of(2) {
            self.0.push(0);
        }
        self
    }

    pub fn read(self, name: u16) -> Self {
        self.name(Opcode::ReadSimple, name)
    }

    pub fn write(self, name: u16) -> Self {
        self.name(Opcode::WriteSimple, name)
    }

    pub fn call(self, name: u16, argc: u16) -> Self {
        self.name_args(Opcode::ReadSimpleParams, 0, name, argc)
    }
}

/// Assembles a module cache (VBA 7.0 x86 arrangement).
pub fn build_cache(lines: &[Line], indirect: &[u8]) -> Vec<u8> {
    fn put32(v: &mut [u8], at: usize, x: u32) {
        v[at..at + 4].copy_from_slice(&x.to_le_bytes());
    }
    let mut v = vec![0u8; 0x3F];
    v[..5].copy_from_slice(&[0x01, 0x16, 0x01, 0x00, 0x06]);
    v.extend_from_slice(&0u32.to_le_bytes());
    let decl_end = v.len();
    v.extend_from_slice(&[0u8; 20]);
    let q = v.len();
    v.extend_from_slice(&[0u8; 0x8A]);
    v.extend_from_slice(&20u32.to_le_bytes());
    v.extend_from_slice(&[0u8; 20]);
    v.extend_from_slice(&[0u8; 24]);
    let p = v.len();
    v.extend_from_slice(&[0u8; 10]);
    v.extend_from_slice(&(indirect.len() as u32).to_le_bytes());
    v.extend_from_slice(indirect);
    v.extend_from_slice(&[0u8; 8]);
    let h9 = v.len();
    v.extend_from_slice(&0x5AFE_B7A9u32.to_le_bytes());
    v.extend_from_slice(&9u16.to_le_bytes());
    let t: Vec<u8> = "*\\Rtest"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    v.push(1);
    v.extend_from_slice(&(t.len() as u16).to_le_bytes());
    v.extend_from_slice(&t);
    v.push(0xDF);
    let r = v.len();
    v.extend_from_slice(&[0u8; 0x3C]);
    v.extend_from_slice(&0xCAFEu16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&(lines.len() as u16).to_le_bytes());
    let mut offset = 0u32;
    let mut code = Vec::new();
    for l in lines {
        v.extend_from_slice(&[0, 0x80, 0x08, 0]);
        v.extend_from_slice(&(l.0.len() as u16).to_le_bytes());
        v.extend_from_slice(&[0, 0]);
        if l.0.is_empty() {
            v.extend_from_slice(&u32::MAX.to_le_bytes());
        } else {
            v.extend_from_slice(&offset.to_le_bytes());
            code.extend_from_slice(&l.0);
            let pad = (8 - l.0.len() % 8) % 8;
            code.extend(std::iter::repeat_n(0xAAu8, pad));
            offset += (l.0.len() + pad) as u32;
        }
    }
    v.extend_from_slice(&[0u8; 10]);
    v.extend_from_slice(&code);
    v.extend_from_slice(&[0u8; 8]);
    let end = v.len();
    v.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0, 0]);
    put32(&mut v, 5, q as u32);
    put32(&mut v, 9, h9 as u32);
    put32(&mut v, 13, decl_end as u32);
    put32(&mut v, 0x11, p as u32);
    put32(&mut v, 0x15, u32::MAX);
    put32(&mut v, 0x19, r as u32);
    put32(&mut v, 0x1D, end as u32);
    v
}

/// Builds the `_VBA_PROJECT` stream with the given identifier spellings.
pub fn build_project_stream(idents: &[String]) -> Vec<u8> {
    let mut v = vec![0u8; 0x1E];
    v[0] = 0xCC;
    v[1] = 0x61;
    v[2..4].copy_from_slice(&0x97u16.to_le_bytes());
    v.extend_from_slice(&[0, 0, 0, 0]);
    v.extend_from_slice(&[0, 0]);
    v.extend_from_slice(&[0, 0]);
    v.extend_from_slice(&[0, 0]);
    for _ in 0..3 {
        v.extend_from_slice(&[0xFF, 0xFF]);
    }
    v.extend_from_slice(&[0u8; 0x64]);
    v.extend_from_slice(&[0, 0]);
    v.extend_from_slice(&[0u8; 6]);
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(&[0u8; 6]);
    let n = idents.len() as u16;
    v.extend_from_slice(&n.to_le_bytes());
    v.extend_from_slice(&n.to_le_bytes());
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&[0u8; 4]);
    for s in idents {
        v.push(s.len() as u8);
        v.push(0);
        v.extend_from_slice(s.as_bytes());
        v.extend_from_slice(&[0u8; 4]);
    }
    v
}
