//! Hostile-input harness for the container, OOXML, PowerPoint, `dir`,
//! PROJECT and module-stream readers.
//!
//! Everything here is deterministic: seeded xorshift generators, fixed case
//! counts, inputs written with the crate's own writer or hand-assembled. No
//! test depends on a file outside the repository. The harness asserts that a
//! reader returns `Ok` or `Err` (never panics), stays inside a time budget and
//! that whatever a successful read retains is bounded by the configured
//! limits.
//!
//! Memory: the workspace forbids `unsafe`, so a counting global allocator
//! cannot be written here. The bound is instead checked on the result
//! ([`retained`] sums every byte a returned project keeps) and, for work that
//! is discarded on error, by the time budget plus the structural limits
//! documented in `docs/formats/vba-format.md` ("Hostile input").
//!
//! Coverage-guided fuzzing can target [`fuzz_entry`], which runs every public
//! byte-level entry point on one input.

mod common;

use std::time::{Duration, Instant};

use common::packages::*;
use common::*;
use vba_format::signature;
use vba_format::{
    Error, LicenseKeys, Limits, ProjectStreams, ProjectWm, ReadMode, compress, decompress,
    extract_project, read_document, read_project, read_project_from_streams,
};
use vba_project::VbaProject;

// ------------------------------------------------------------ infrastructure

/// Deterministic xorshift64* generator.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
}

/// Values that sit on arithmetic boundaries.
const U32_SPECIAL: [u32; 8] = [
    0,
    1,
    0xFFFF_FFFF,
    0xFFFF_FFFE,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF,
    0x1_0000,
];

/// Total bytes a project keeps alive.
fn retained(p: &VbaProject) -> usize {
    p.modules
        .iter()
        .map(|m| m.source.len() + m.preserved.total_bytes() + m.name.len())
        .sum::<usize>()
        + p.preserved.total_bytes()
}

fn small_limits() -> Limits {
    Limits {
        max_container_bytes: 1 << 20,
        max_stream_bytes: 1 << 19,
        max_decompressed_bytes: 1 << 19,
        max_entries: 256,
        max_archive_entries: 64,
        ..Limits::default()
    }
}

/// Runs `f` and fails if it takes longer than `budget`.
fn within<T>(budget: Duration, what: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    let took = start.elapsed();
    assert!(took <= budget, "{what} took {took:?} (budget {budget:?})");
    out
}

/// Every byte-level entry point on one input; the single hook a
/// coverage-guided fuzzer needs. Results are ignored: only a panic, a hang or
/// an unbounded allocation is a failure.
fn fuzz_entry(bytes: &[u8]) {
    let limits = small_limits();
    let _ = extract_project(bytes, &limits);
    let _ = read_document(bytes, &limits, ReadMode::Strict);
    let _ = read_document(bytes, &limits, ReadMode::Tolerant);
    let _ = read_project(bytes, &limits);
    let _ = signature::read_signature_part(bytes, &limits);
    let _ = signature::parse_blob(bytes);
    let _ = decompress(bytes, 1 << 20);
    let _ = ProjectWm::parse(bytes, 1252);
    let _ = LicenseKeys::parse(bytes);
    let _ = vba_format::crypt::decrypt(bytes);
    let _ = vba_format::crypt::hex_decode(&String::from_utf8_lossy(bytes));
}

fn assert_bounded(result: Result<VbaProject, Error>, limits: &Limits, what: &str) {
    if let Ok(p) = result {
        let r = retained(&p);
        assert!(
            r as u64 <= 4 * limits.max_container_bytes + (1 << 16),
            "{what}: a successful read retains {r} bytes, far over the {} byte limit",
            limits.max_container_bytes
        );
    }
}

// ------------------------------------------------------------- stream sets

fn base_streams() -> ProjectStreams {
    let (streams, _) = dump_cfb(sample());
    streams
        .into_iter()
        .map(|(p, d)| (p.trim_start_matches('/').to_owned(), d))
        .collect()
}

/// A `dir` stream whose modules all name the stream `stream`.
fn aliased_dir(modules: usize, stream: &str) -> Vec<u8> {
    let mut v = project_info();
    v.extend(spec_references());
    v.extend(rec(0x000F, &u16le(modules as u16)));
    v.extend(rec(0x0013, &u16le(0xFFFF)));
    for i in 0..modules {
        let name = format!("M{i}");
        v.extend(rec(0x0019, name.as_bytes()));
        v.extend(rec(0x0047, &utf16(&name)));
        v.extend(pair(0x001A, 0x0032, stream));
        v.extend(pair(0x001C, 0x0048, ""));
        v.extend(rec(0x0031, &u32le(0)));
        v.extend(rec(0x001E, &u32le(0)));
        v.extend(rec(0x002C, &u16le(0xFFFF)));
        v.extend(u16le(0x0021));
        v.extend(u32le(0));
        v.extend(u16le(0x002B));
        v.extend(u32le(0));
    }
    v.extend(u16le(0x0010));
    v.extend(u32le(0));
    v
}

// ------------------------------------------------------------- known attacks

/// Many module records naming one large stream: each would decompress and
/// copy it again, multiplying the memory held by the model.
#[test]
fn aliased_module_streams_are_bounded() {
    let limits = Limits {
        max_container_bytes: 512 << 10,
        max_stream_bytes: 512 << 10,
        max_decompressed_bytes: 256 << 10,
        ..Limits::default()
    };
    let mut streams = base_streams();
    streams.insert("VBA/dir", compress(&aliased_dir(64, "Big")));
    streams.insert(
        "VBA/Big",
        module_stream(&[], &vec![b'a'; limits.max_decompressed_bytes]),
    );
    let r = within(Duration::from_secs(20), "aliased modules", || {
        read_project_from_streams(&streams, ReadMode::Strict, &limits)
    });
    assert!(r.is_err(), "64 aliased 256 KiB modules must be refused");
    assert_bounded(r, &limits, "aliased modules");
}

/// A PROJECT stream with very many module lines and a `dir` with many
/// modules must not cost time proportional to their product.
#[test]
fn module_kind_lookup_is_not_quadratic() {
    let mut text = String::new();
    for i in 0..120_000 {
        text.push_str(&format!("Module=Other{i}\r\n"));
    }
    let limits = Limits {
        max_container_bytes: 64 << 20,
        max_stream_bytes: 64 << 20,
        ..Limits::default()
    };
    let mut streams = base_streams();
    streams.insert("PROJECT", project_stream(&text));
    streams.insert("VBA/dir", compress(&aliased_dir(4000, "ThisWorkbook")));
    let r = within(Duration::from_secs(10), "module kind lookup", || {
        read_project_from_streams(&streams, ReadMode::Strict, &limits)
    });
    assert!(r.is_ok() || r.is_err());
}

/// Builds a presentation whose user-edit chain has `edits` entries that all
/// point at the same persist directory.
fn long_edit_chain(edits: usize, dir_entries: usize) -> Vec<u8> {
    let doc = document_container(2, 1);
    let old_stg = vba_stg(false, b"outdated storage");
    let mut stream = Vec::new();
    let old_off = stream.len();
    stream.extend(&old_stg);
    let doc_off = stream.len();
    stream.extend(&doc);
    let dir_off = stream.len();
    let mut entries = vec![(1u32, doc_off), (2, old_off)];
    for i in 0..dir_entries {
        entries.push((3 + i as u32, old_off));
    }
    stream.extend(persist_dir(&entries));
    let mut last = 0;
    for _ in 0..edits {
        let here = stream.len();
        stream.extend(user_edit(last, dir_off, 1));
        last = here;
    }
    build_cfb(
        &[
            ("Current User".into(), current_user(last, 0xE391_C05F)),
            ("PowerPoint Document".into(), stream),
        ],
        &[],
    )
}

/// A long user-edit chain pointing at one large persist directory must not
/// replay that directory once per edit.
#[test]
fn powerpoint_edit_chain_is_linear() {
    let bytes = long_edit_chain(20_000, 8_000);
    let limits = Limits::default();
    within(Duration::from_secs(10), "PowerPoint edit chain", || {
        let _ = extract_project(&bytes, &limits);
    });
}

// ----------------------------------------------------------- CFB mutation

/// A minimal view of a compound file (either sector size), enough to rewrite fields.
struct CfbView {
    bytes: Vec<u8>,
}

impl CfbView {
    fn sector_size(&self) -> usize {
        1usize << u16::from_le_bytes([self.bytes[0x1E], self.bytes[0x1F]]).min(16)
    }
    fn sector_off(&self, id: u32) -> usize {
        self.sector_size() * (1 + id as usize)
    }
    fn u32(&self, at: usize) -> u32 {
        u32::from_le_bytes(self.bytes[at..at + 4].try_into().unwrap())
    }
    fn set_u32(&mut self, at: usize, v: u32) {
        self.bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn fat_sectors(&self) -> Vec<u32> {
        let n = self.u32(0x2C) as usize;
        (0..n.min(109)).map(|i| self.u32(0x4C + 4 * i)).collect()
    }
    fn fat_entry_at(&self, index: usize) -> Option<usize> {
        let per = self.sector_size() / 4;
        let s = *self.fat_sectors().get(index / per)?;
        Some(self.sector_off(s) + 4 * (index % per))
    }
    fn fat_len(&self) -> usize {
        self.fat_sectors().len() * (self.sector_size() / 4)
    }
    /// Offsets of the directory entries (128 bytes each).
    fn dir_entries(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut s = self.u32(0x30);
        let mut guard = 0;
        while s < 0xFFFF_FFF0 && guard < 4096 {
            for k in 0..self.sector_size() / 128 {
                out.push(self.sector_off(s) + 128 * k);
            }
            let Some(e) = self.fat_entry_at(s as usize) else { break };
            s = self.u32(e);
            guard += 1;
        }
        out.retain(|&o| o + 128 <= self.bytes.len());
        out
    }
}

fn cfb_corpus() -> Vec<Vec<u8>> {
    vec![
        sample().to_vec(),
        legacy_document("Macros", &[("WordDocument", vec![1; 20])]),
    ]
}

/// Sets every header field, FAT entry and directory-entry field in turn to
/// boundary values.
#[test]
fn cfb_field_inflation_never_panics() {
    let limits = small_limits();
    let mut runs = 0usize;
    for base in cfb_corpus() {
        let view = CfbView { bytes: base };
        // Header words up to the first DIFAT entries, then directory fields.
        let mut offsets: Vec<usize> = (0..0x60).step_by(4).collect();
        for e in view.dir_entries() {
            // name length, type/colour, siblings, child, start sector, size
            for rel in [64, 68, 72, 76, 96, 116, 120, 124] {
                offsets.push(e + rel);
            }
        }
        for i in 0..view.fat_len().min(48) {
            if let Some(o) = view.fat_entry_at(i) {
                offsets.push(o);
            }
        }
        let len = view.bytes.len() as u32;
        for &at in &offsets {
            if at + 4 > view.bytes.len() {
                continue;
            }
            for v in [0, 1, 0xFFFF_FFFF, 0xFFFF_FFFE, len + 1, len / 512] {
                let mut m = CfbView { bytes: view.bytes.clone() };
                m.set_u32(at, v);
                within(Duration::from_secs(5), "inflated CFB field", || {
                    let r = read_project(&m.bytes, &limits);
                    assert_bounded(r, &limits, "inflated CFB field");
                    let _ = extract_project(&m.bytes, &limits);
                });
                runs += 1;
            }
        }
    }
    assert!(runs > 1000, "{runs}");
}

/// FAT chains rewritten into self loops, back references and loops through
/// the directory and mini-stream sectors.
#[test]
fn cfb_cyclic_chains_never_hang() {
    let limits = small_limits();
    for base in cfb_corpus() {
        let view = CfbView { bytes: base };
        let n = view.fat_len().min(256);
        for i in 0..n {
            let Some(at) = view.fat_entry_at(i) else { continue };
            let targets = [
                i as u32,
                (i as u32).wrapping_sub(1),
                0,
                1,
                (i as u32 + 1) % n as u32,
                view.u32(0x30),
                view.u32(0x3C),
            ];
            for t in targets {
                let mut m = CfbView { bytes: view.bytes.clone() };
                m.set_u32(at, t);
                within(Duration::from_secs(5), "cyclic CFB chain", || {
                    let _ = read_project(&m.bytes, &limits);
                    let _ = extract_project(&m.bytes, &limits);
                });
            }
        }
        // Directory sibling and child cycles.
        let entries = view.dir_entries();
        for (ei, &e) in entries.iter().enumerate() {
            for rel in [68, 72, 76] {
                for t in [ei as u32, 0, 1, (ei as u32 + 1) % entries.len() as u32] {
                    let mut m = CfbView { bytes: view.bytes.clone() };
                    m.set_u32(e + rel, t);
                    within(Duration::from_secs(5), "cyclic directory", || {
                        let _ = read_project(&m.bytes, &limits);
                        let _ = extract_project(&m.bytes, &limits);
                    });
                }
            }
        }
    }
}

/// Several directory entries sharing one large chain, so that reading all
/// streams multiplies the data held.
#[test]
fn streams_sharing_one_chain_are_bounded() {
    let big = filler(300_000, 5);
    let streams: Vec<(String, Vec<u8>)> = (0..24)
        .map(|i| (format!("S{i:02}"), if i == 0 { big.clone() } else { vec![0; 5000] }))
        .collect();
    let vba = StorageSpec {
        path: "VBA",
        clsid: [0; 16],
        state_bits: 0,
    };
    let base = build_cfb(&streams, &[vba]);
    let view = CfbView { bytes: base.clone() };
    let entries = view.dir_entries();
    // Entry 0 is the root; entries 1.. are the streams in creation order of
    // the red-black tree, so find S00 by its start sector and size.
    let mut src = None;
    for &e in &entries {
        if view.u32(e + 120) == 300_000 {
            src = Some((view.u32(e + 116), view.u32(e + 120)));
        }
    }
    let (start, size) = src.expect("S00 entry");
    let mut m = CfbView { bytes: base };
    for &e in &entries {
        let ty = m.bytes[e + 66];
        if ty == 2 {
            m.set_u32(e + 116, start);
            m.set_u32(e + 120, size);
        }
    }
    let limits = Limits {
        max_container_bytes: 1 << 20,
        max_stream_bytes: 1 << 20,
        ..Limits::default()
    };
    // The stream set is not a project; only the stream-gathering step is
    // under test, which runs before the project grammar is checked.
    let r = within(Duration::from_secs(20), "shared chains", || {
        read_project(&m.bytes, &limits)
    });
    assert!(
        matches!(r, Err(Error::InputTooLarge { .. })),
        "{:?}",
        r.map(|_| ())
    );
}

// ------------------------------------------------------- generic mutation

fn mutate(rng: &mut Rng, base: &[u8]) -> Vec<u8> {
    let mut b = base.to_vec();
    if b.is_empty() {
        return b;
    }
    match rng.below(9) {
        0 => {
            for _ in 0..1 + rng.below(8) {
                let i = rng.below(b.len());
                b[i] ^= 1 << rng.below(8);
            }
        }
        1 => {
            for _ in 0..1 + rng.below(8) {
                let i = rng.below(b.len());
                b[i] = [0x00, 0xFF, 0x7F, 0x80][rng.below(4)];
            }
        }
        2 => {
            let cut = rng.below(b.len() + 1);
            b.truncate(cut);
        }
        3 => {
            let extra = rng.below(4096);
            for _ in 0..extra {
                b.push(rng.next() as u8);
            }
        }
        4 => {
            // duplicate a chunk
            let s = rng.below(b.len());
            let e = (s + 1 + rng.below(600)).min(b.len());
            let chunk = b[s..e].to_vec();
            let at = rng.below(b.len() + 1);
            b.splice(at..at, chunk);
        }
        5 => {
            // remove a chunk
            let s = rng.below(b.len());
            let e = (s + 1 + rng.below(600)).min(b.len());
            b.drain(s..e);
        }
        6 => {
            // overwrite a u32 with a boundary value
            if b.len() >= 4 {
                let i = rng.below(b.len() - 3);
                let v = U32_SPECIAL[rng.below(U32_SPECIAL.len())];
                b[i..i + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        7 => {
            // overwrite a u16
            if b.len() >= 2 {
                let i = rng.below(b.len() - 1);
                let v = [0u16, 1, 0xFFFF, 0xFFFE, 0x8000][rng.below(5)];
                b[i..i + 2].copy_from_slice(&v.to_le_bytes());
            }
        }
        _ => {
            // splice random bytes
            let at = rng.below(b.len() + 1);
            let n = rng.below(64);
            let junk: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
            b.splice(at..at, junk);
        }
    }
    b
}

#[test]
fn mutated_containers_never_panic() {
    let limits = small_limits();
    let mut corpus = cfb_corpus();
    corpus.push(docm());
    corpus.push(presentation(true, 1, 0xE391_C05F));
    corpus.push(legacy_document("_VBA_PROJECT_CUR", &[("Workbook", vec![2; 20])]));
    corpus.push(presentation(false, 1, 0xE391_C05F));
    let mut rng = Rng::new(0x5EED_0001);
    for round in 0..1500 {
        let base = &corpus[round % corpus.len()];
        let mut m = mutate(&mut rng, base);
        if rng.below(3) == 0 {
            m = mutate(&mut rng, &m);
        }
        within(Duration::from_secs(5), "mutated container", || {
            fuzz_entry(&m);
            let r = read_document(&m, &limits, ReadMode::Tolerant).map(|(p, _)| p);
            assert_bounded(r, &limits, "mutated container");
        });
    }
}

// ------------------------------------------------- stream-level mutation

fn recompress_set(dir: &[u8], mut streams: ProjectStreams) -> ProjectStreams {
    streams.insert("VBA/dir", compress(dir));
    streams
}

/// Every offset of the decompressed `dir` stream rewritten with boundary
/// values (counts, lengths, record identifiers).
#[test]
fn dir_field_inflation_never_panics() {
    let limits = small_limits();
    let streams = base_streams();
    let dir = decompress(streams.get("VBA/dir").unwrap(), 1 << 20).unwrap();
    let mut runs = 0;
    for at in (0..dir.len()).step_by(2) {
        let mut variants: Vec<Vec<u8>> = Vec::new();
        for v in [0, 0xFFFF_FFFF, dir.len() as u32 + 1] {
            if at + 4 <= dir.len() {
                let mut d = dir.clone();
                d[at..at + 4].copy_from_slice(&v.to_le_bytes());
                variants.push(d);
            }
        }
        for v in [0xFFFFu16] {
            if at + 2 <= dir.len() {
                let mut d = dir.clone();
                d[at..at + 2].copy_from_slice(&v.to_le_bytes());
                variants.push(d);
            }
        }
        for d in &mut variants {
            for mode in [if at % 4 == 0 { ReadMode::Strict } else { ReadMode::Tolerant }] {
                let set = recompress_set(d, streams.clone());
                within(Duration::from_secs(5), "inflated dir field", || {
                    let r = read_project_from_streams(&set, mode, &limits);
                    assert_bounded(r, &limits, "inflated dir field");
                });
                runs += 1;
            }
        }
        // Truncation at every third offset.
        if at % 6 == 0 {
            let set = recompress_set(&dir[..at], streams.clone());
            let _ = read_project_from_streams(&set, ReadMode::Tolerant, &limits);
        }
    }
    assert!(runs > 1500, "{runs}");
}

/// Random mutation of the decompressed `dir`, PROJECT, `_VBA_PROJECT` and
/// module streams, recompressed so the damage reaches the grammar.
#[test]
fn mutated_streams_never_panic() {
    let limits = small_limits();
    let streams = base_streams();
    let dir = decompress(streams.get("VBA/dir").unwrap(), 1 << 20).unwrap();
    let names: Vec<String> = streams.paths().map(str::to_owned).collect();
    let mut rng = Rng::new(0x5EED_0002);
    for _ in 0..1500 {
        let mut set = streams.clone();
        for _ in 0..1 + rng.below(3) {
            let pick = &names[rng.below(names.len())];
            if pick.eq_ignore_ascii_case("VBA/dir") {
                let d = mutate(&mut rng, &dir);
                set.insert("VBA/dir", compress(&d));
            } else {
                let cur = set.get(pick).unwrap().to_vec();
                set.insert(pick.clone(), mutate(&mut rng, &cur));
            }
        }
        let mode = if rng.below(2) == 0 { ReadMode::Strict } else { ReadMode::Tolerant };
        within(Duration::from_secs(5), "mutated streams", || {
            let r = read_project_from_streams(&set, mode, &limits);
            assert_bounded(r, &limits, "mutated streams");
        });
    }
}

// --------------------------------------------------------- decompression

#[test]
fn decompression_bombs_respect_the_limit() {
    // Maximal-length copy tokens: one literal then tokens of 4098 bytes would
    // exceed a chunk; the decoder must stop at the chunk and at the limit.
    let zeros = vec![0u8; 4 << 20];
    let packed = compress(&zeros);
    assert!(packed.len() < 8000);
    assert!(matches!(
        decompress(&packed, 1 << 20),
        Err(vba_format::DecompressError::LimitExceeded { .. })
    ));
    assert_eq!(decompress(&packed, 4 << 20).unwrap().len(), 4 << 20);
    // A copy token whose length would run past the chunk end.
    let mut crafted = vec![0x01];
    // chunk header: compressed, 3 bits signature, size = 5 bytes of data
    let data = [0b0000_0010u8, b'a', 0xFF, 0xFF];
    let header = 0xB000u16 | (data.len() as u16 + 2 - 3);
    crafted.extend(header.to_le_bytes());
    crafted.extend(data);
    assert!(decompress(&crafted, 1 << 20).is_err());
}

#[test]
fn decompress_random_chunks_never_panics() {
    let mut rng = Rng::new(0x5EED_0003);
    for _ in 0..20_000 {
        let n = rng.below(300);
        let mut v = vec![0x01];
        let mut remaining = n;
        while remaining > 0 {
            let len = (1 + rng.below(remaining.min(200))).min(remaining);
            let hdr = 0x3000u16 | if rng.below(4) != 0 { 0x8000 } else { 0 } | (len as u16 + 1) % 0x1000;
            v.extend(hdr.to_le_bytes());
            v.extend((0..len).map(|_| rng.next() as u8));
            remaining -= len;
        }
        if rng.below(5) == 0 && !v.is_empty() {
            let c = rng.below(v.len());
            v.truncate(c);
        }
        let _ = decompress(&v, 1 << 16);
    }
}

#[test]
fn compress_round_trips_pathological_inputs() {
    let mut rng = Rng::new(0x5EED_0004);
    let mut inputs: Vec<Vec<u8>> = vec![
        vec![],
        vec![0; 1],
        vec![0; 4096],
        vec![0; 4097],
        vec![0xFF; 9000],
        (0..9000).map(|i| (i % 3) as u8).collect(),
        (0..9000).map(|i| (i % 251) as u8).collect(),
    ];
    for _ in 0..40 {
        let n = rng.below(10_000);
        let alphabet = 1 + rng.below(6);
        inputs.push((0..n).map(|_| (rng.next() % alphabet as u64) as u8).collect());
    }
    for input in inputs {
        let packed = within(Duration::from_secs(10), "compress", || compress(&input));
        let back = decompress(&packed, input.len() + 4096).unwrap();
        assert_eq!(&back[..input.len()], &input[..]);
    }
}
