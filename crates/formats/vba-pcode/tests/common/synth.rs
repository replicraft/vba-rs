//! Hand-built, fully synthetic cache and project-stream bytes for tests.
//!
//! The builders follow `docs/pcode-format.md` for the VBA 7.0 x86 layout;
//! nothing here is derived from any real project.

#![allow(dead_code, missing_docs)]

/// Specification of a synthetic module cache.
#[derive(Clone, Debug)]
pub struct CacheSpec {
    /// Code bytes of each line (empty = line without code).
    pub lines: Vec<Vec<u8>>,
    /// Object-table entries.
    pub objects: Vec<[u8; 10]>,
    /// Indirect-table contents.
    pub indirect: Vec<u8>,
    /// Declaration-table contents.
    pub decl: Vec<u8>,
    /// Reference-list entries.
    pub references: Vec<String>,
    /// Number of member-table records (None = standard module layout).
    pub members: Option<usize>,
}

impl Default for CacheSpec {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            objects: vec![[0; 10]; 2],
            indirect: vec![0; 16],
            decl: Vec::new(),
            references: vec!["*\\Rtest".into()],
            members: None,
        }
    }
}

fn put32(v: &mut [u8], at: usize, x: u32) {
    v[at..at + 4].copy_from_slice(&x.to_le_bytes());
}

fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// Builds a module cache in the VBA 7.0 x86 arrangement.
pub fn build_cache(spec: &CacheSpec) -> Vec<u8> {
    let mut v = vec![0u8; 0x3F];
    v[..5].copy_from_slice(&[0x01, 0x16, 0x01, 0x00, 0x06]);
    // Declaration table.
    v.extend_from_slice(&(spec.decl.len() as u32).to_le_bytes());
    v.extend_from_slice(&spec.decl);
    let decl_end = v.len();
    v.extend_from_slice(&[0u8; 20]); // block between declarations and Q
    // Object table.
    let q = v.len();
    v.extend_from_slice(&[0u8; 0x8A]);
    v.extend_from_slice(&((spec.objects.len() * 10) as u32).to_le_bytes());
    for e in &spec.objects {
        v.extend_from_slice(e);
    }
    v.extend_from_slice(&[0u8; 24]);
    // Indirect table.
    let p = v.len();
    v.extend_from_slice(&[0u8; 10]);
    v.extend_from_slice(&(spec.indirect.len() as u32).to_le_bytes());
    v.extend_from_slice(&spec.indirect);
    v.extend_from_slice(&[0u8; 8]);
    // Reference list.
    let h9 = v.len();
    v.extend_from_slice(&0x5AFE_B7A9u32.to_le_bytes());
    v.extend_from_slice(&9u16.to_le_bytes());
    for r in &spec.references {
        let t = utf16(r);
        v.push(1);
        v.extend_from_slice(&(t.len() as u16).to_le_bytes());
        v.extend_from_slice(&t);
    }
    v.push(0xDF);
    // Member table.
    let h21 = match spec.members {
        Some(n) => {
            let at = v.len();
            v.extend_from_slice(&(n as u16).to_le_bytes());
            for i in 0..n {
                v.extend_from_slice(&0u32.to_le_bytes());
                v.extend_from_slice(&(0x1770 + i as u32).to_le_bytes());
                v.extend_from_slice(&(0x34 + 4 * i as u32).to_le_bytes());
            }
            at as u32
        }
        None => u32::MAX,
    };
    // Line table.
    let r = v.len();
    v.extend_from_slice(&[0u8; 0x3C]);
    v.extend_from_slice(&0xCAFEu16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&(spec.lines.len() as u16).to_le_bytes());
    let mut offset = 0u32;
    let mut code = Vec::new();
    for l in &spec.lines {
        v.extend_from_slice(&[0, 0x80, 0x08, 0]);
        v.extend_from_slice(&(l.len() as u16).to_le_bytes());
        v.extend_from_slice(&[0, 0]);
        if l.is_empty() {
            v.extend_from_slice(&u32::MAX.to_le_bytes());
        } else {
            v.extend_from_slice(&offset.to_le_bytes());
            code.extend_from_slice(l);
            let pad = (8 - l.len() % 8) % 8;
            code.extend(std::iter::repeat_n(0xAAu8, pad));
            offset += (l.len() + pad) as u32;
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
    put32(&mut v, 0x15, h21);
    put32(&mut v, 0x19, r as u32);
    put32(&mut v, 0x1D, end as u32);
    v
}

/// Builds a project stream with the given identifier spellings.
pub fn build_project_stream(version: u16, idents: &[&str]) -> Vec<u8> {
    let mut v = vec![0u8; 0x1E];
    v[0] = 0xCC;
    v[1] = 0x61;
    v[2..4].copy_from_slice(&version.to_le_bytes());
    v.extend_from_slice(&[0, 0, 0, 0]); // no references, skip 2
    v.extend_from_slice(&[0, 0]); // form table
    v.extend_from_slice(&[0, 0]); // constant pairs
    v.extend_from_slice(&[0, 0]); // skip
    for _ in 0..3 {
        v.extend_from_slice(&[0xFF, 0xFF]);
    }
    v.extend_from_slice(&[0u8; 0x64]);
    v.extend_from_slice(&[0, 0]); // no modules
    v.extend_from_slice(&[0u8; 6]);
    v.extend_from_slice(&0u32.to_le_bytes()); // block
    v.extend_from_slice(&[0u8; 6]);
    let n = idents.len() as u16;
    v.extend_from_slice(&n.to_le_bytes()); // a
    v.extend_from_slice(&n.to_le_bytes()); // b
    v.extend_from_slice(&0u16.to_le_bytes()); // c
    v.extend_from_slice(&[0u8; 4]);
    for s in idents {
        v.push(s.len() as u8);
        v.push(0);
        v.extend_from_slice(s.as_bytes());
        v.extend_from_slice(&[0u8; 4]);
    }
    v
}

/// Instruction word for a *stored* (x86 VBA7) opcode and modifier.
pub fn word(stored: u16, modifier: u16) -> [u8; 2] {
    (stored | (modifier << 10)).to_le_bytes()
}

/// The name word for user identifier `index` in the verified layout.
pub fn user_name(index: u16) -> [u8; 2] {
    ((index + 4 + 0x100) << 1).to_le_bytes()
}

/// A 70-byte function record: public function with a declared `String`
/// return type, no arguments.
pub fn function_record(name_word: u16) -> Vec<u8> {
    let mut r = vec![0u8; 70];
    r[0..2].copy_from_slice(&0x1020u16.to_le_bytes());
    r[2..4].copy_from_slice(&name_word.to_le_bytes());
    r[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
    r[44..48].copy_from_slice(&0xFFFF_0008u32.to_le_bytes());
    r[48..50].copy_from_slice(&0xFFFFu16.to_le_bytes());
    r[58] = 0x8C;
    r[61] = 0x02;
    r
}
