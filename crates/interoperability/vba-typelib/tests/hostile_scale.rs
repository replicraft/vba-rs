//! Work and output amplification in the type library readers and in the
//! binding that consumes them. Synthetic and deterministic.

#![cfg(feature = "sema")]

mod common;

use std::time::{Duration, Instant};

use common::builder::*;
use vba_sema::LibraryType;
use vba_typelib::{parse_msft, sema::TypelibProvider};

fn many_functions(n: usize) -> LibSpec {
    let mut lib = LibSpec::new("Big");
    let mut t = TypeSpec::new("Huge", TypeKind::Dispatch);
    for i in 0..n {
        t.funcs.push(FuncSpec::new(
            &format!("Member{i}"),
            i as i32 + 1,
            InvokeKind::Func,
            Dt::Prim(3),
        ));
    }
    lib.types.push(t);
    lib
}

/// One type with tens of thousands of members: parsing and building its VBA
/// member list must not compare every name with every other.
#[test]
fn member_lists_are_not_quadratic() {
    let data = build(&many_functions(30_000));
    let started = Instant::now();
    let lib = parse_msft(&data).expect("parses");
    let parsed = started.elapsed();
    assert!(parsed < Duration::from_secs(20), "parse {parsed:?}");
    let provider = TypelibProvider::new().with_library(lib);
    let ty = LibraryType {
        library: "Big".into(),
        name: "Huge".into(),
    };
    let started = Instant::now();
    let members = provider.all_members(&ty).expect("members");
    assert_eq!(members.len(), 30_000);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "members {:?}",
        started.elapsed()
    );
}

/// Several type records that all point at one record multiply the members
/// held by the model: the file stays small while the library does not.
#[test]
fn aliased_type_records_do_not_multiply_the_library() {
    let mut spec = many_functions(500);
    for i in 1..200 {
        let mut t = TypeSpec::new(&format!("T{i}"), TypeKind::Dispatch);
        t.funcs
            .push(FuncSpec::new("F", 1, InvokeKind::Func, Dt::Prim(3)));
        spec.types.push(t);
    }
    let mut data = build(&spec);
    let n = u32::from_le_bytes(data[0x20..0x24].try_into().unwrap()) as usize;
    // Every type-info offset now names the first record.
    let first: [u8; 4] = data[0x54..0x58].try_into().unwrap();
    for i in 0..n {
        data[0x54 + 4 * i..0x58 + 4 * i].copy_from_slice(&first);
    }
    let file_len = data.len();
    let started = Instant::now();
    let held = parse_msft(&data).map_or(0, |l| {
        l.types
            .iter()
            .map(|t| t.functions.len() + t.variables.len())
            .sum::<usize>()
    });
    assert!(started.elapsed() < Duration::from_secs(20));
    // A member needs at least a few bytes of the file; the model must not
    // hold more than a bounded multiple of what the file could describe.
    assert!(
        held <= file_len,
        "{held} members from a {file_len} byte file"
    );
}

/// An image listing hundreds of TYPELIB resources is refused instead of
/// listed (resource tables can share subtables, so the count is not bounded
/// by the size of the file).
#[test]
fn resource_lists_are_capped() {
    use common::pe::{Res, Ty, build_pe};
    let tiny = build(&LibSpec::new("T"));
    let resources: Vec<Res> = (1..=300u16)
        .map(|id| Res {
            ty: Ty::Name("TYPELIB"),
            id,
            lang: 0,
            data: tiny.clone(),
        })
        .collect();
    let img = build_pe(false, &resources);
    assert!(matches!(
        vba_typelib::pe::typelib_resources(&img),
        Err(vba_typelib::TypelibError::LimitExceeded(_))
    ));
    assert!(vba_typelib::load_all(&img).is_err());
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Seeded splice, duplicate, delete and overwrite mutations of an MSFT
/// library, an SLTG library and a PE image holding both, through every
/// public entry point.
#[test]
fn structural_mutations_never_panic() {
    use common::pe::{Res, Ty, build_pe};
    let msft = build(&common::sample());
    let sltg = common::sltg::build_sltg(&common::sltg_libs::sample());
    let pe = build_pe(
        true,
        &[
            Res {
                ty: Ty::Name("TYPELIB"),
                id: 1,
                lang: 0,
                data: msft.clone(),
            },
            Res {
                ty: Ty::Name("TYPELIB"),
                id: 2,
                lang: 0,
                data: sltg.clone(),
            },
        ],
    );
    let bases = [msft, sltg, pe];
    let mut rng = Rng(0x5EED_0070);
    let started = Instant::now();
    for round in 0..6000 {
        let mut b = bases[round % bases.len()].clone();
        for _ in 0..1 + rng.below(5) {
            if b.is_empty() {
                break;
            }
            match rng.below(6) {
                0 => {
                    let i = rng.below(b.len());
                    b[i] = rng.next() as u8;
                }
                1 if b.len() >= 4 => {
                    let i = rng.below(b.len() - 3);
                    let v = [0u32, 1, 0xFFFF_FFFF, 0x8000_0000, 0xFFFF][rng.below(5)];
                    b[i..i + 4].copy_from_slice(&v.to_le_bytes());
                }
                2 => {
                    let n = rng.below(b.len() + 1);
                    b.truncate(n);
                }
                3 => {
                    let s = rng.below(b.len());
                    let e = (s + 1 + rng.below(200)).min(b.len());
                    let chunk = b[s..e].to_vec();
                    let at = rng.below(b.len() + 1);
                    b.splice(at..at, chunk);
                }
                4 => {
                    let s = rng.below(b.len());
                    let e = (s + 1 + rng.below(200)).min(b.len());
                    b.drain(s..e);
                }
                _ => {
                    let i = rng.below(b.len());
                    b[i] ^= 1 << rng.below(8);
                }
            }
        }
        let t = Instant::now();
        let _ = vba_typelib::load(&b);
        if let Ok(all) = vba_typelib::load_all(&b) {
            for (_, lib) in &all {
                let provider = TypelibProvider::new().with_library(lib.clone());
                for t in &lib.types {
                    let _ = provider.all_members(&LibraryType {
                        library: lib.name.clone(),
                        name: t.name.clone(),
                    });
                }
            }
        }
        assert!(t.elapsed() < Duration::from_secs(10), "{:?}", t.elapsed());
    }
    assert!(started.elapsed() < Duration::from_secs(120));
}

/// Mutated real libraries from the directory named by `VBA_TYPELIB_SAMPLES`
/// (never committed); returns at once when the variable is unset. Only
/// counts are printed.
#[test]
fn mutated_real_libraries_never_panic() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    let Some(root) = std::env::var_os("VBA_TYPELIB_SAMPLES") else {
        return;
    };
    let mut files = Vec::new();
    walk(std::path::Path::new(&root), &mut files);
    let mut rng = Rng(0x5EED_0071);
    let (mut seen, mut runs) = (0usize, 0usize);
    for f in files.iter().step_by(7).take(60) {
        let Ok(base) = std::fs::read(f) else { continue };
        if base.len() > 6 << 20 {
            continue;
        }
        seen += 1;
        for _ in 0..12 {
            let mut b = base.clone();
            for _ in 0..1 + rng.below(6) {
                let i = rng.below(b.len());
                match rng.below(3) {
                    0 => b[i] = rng.next() as u8,
                    1 if b.len() >= 4 && i + 4 <= b.len() => {
                        b[i..i + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
                    }
                    _ => b.truncate(i.max(1)),
                }
            }
            let t = Instant::now();
            let _ = vba_typelib::load_all(&b);
            assert!(t.elapsed() < Duration::from_secs(30));
            runs += 1;
        }
    }
    println!("{seen} files, {runs} mutations");
}

/// The single hook a coverage-guided fuzzer needs: every byte-level entry
/// point of the crate on one input.
pub fn fuzz_entry(bytes: &[u8]) {
    let _ = vba_typelib::parse_msft(bytes);
    let _ = vba_typelib::parse_sltg(bytes);
    let _ = vba_typelib::load(bytes);
    if let Ok(all) = vba_typelib::load_all(bytes) {
        for (_, lib) in &all {
            for t in &lib.types {
                for f in &t.functions {
                    let _ = lib.describe(&f.ret);
                }
            }
        }
    }
    let _ = vba_typelib::pe::typelib_resources(bytes);
}

#[test]
fn fuzz_entry_accepts_arbitrary_bytes() {
    let mut rng = Rng(0x5EED_0072);
    for _ in 0..3000 {
        let n = rng.below(600);
        let mut b: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        if rng.below(2) == 0 && b.len() >= 4 {
            b[..4].copy_from_slice([b"MSFT", b"SLTG", b"MZ\0\0"][rng.below(3)]);
        }
        fuzz_entry(&b);
    }
}
