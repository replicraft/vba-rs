//! Mutation of real documents named by the environment.
//!
//! `VBA_PCODE_CORPUS` names a directory of project files (documents of any
//! supported container or standalone storages) and `VBA_PCODE_PROJECT` a
//! single file. The files are local, untrusted data that is never committed;
//! the tests read them through these variables only and return at once when
//! both are unset. Nothing about the files is printed except counts.
//!
//! Run with `VBA_PCODE_CORPUS=<dir> cargo test -p vba-format --test
//! hostile_corpus -- --nocapture`.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::fuzz::*;
use vba_format::{Limits, ReadMode, extract_project, read_document, read_project};

fn files() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    if let Some(dir) = std::env::var_os("VBA_PCODE_CORPUS") {
        walk(Path::new(&dir), &mut v);
    }
    if let Some(f) = std::env::var_os("VBA_PCODE_PROJECT") {
        v.push(PathBuf::from(f));
    }
    v.sort();
    v
}

fn limits() -> Limits {
    Limits {
        max_container_bytes: 64 << 20,
        max_stream_bytes: 16 << 20,
        max_decompressed_bytes: 16 << 20,
        max_entries: 4096,
        max_archive_entries: 4096,
        ..Limits::default()
    }
}

fn exercise(bytes: &[u8], limits: &Limits) {
    let _ = extract_project(bytes, limits);
    let _ = read_document(bytes, limits, ReadMode::Strict);
    let _ = read_document(bytes, limits, ReadMode::Tolerant);
    let _ = read_project(bytes, limits);
}

#[test]
fn mutated_real_files_never_panic() {
    let files = files();
    if files.is_empty() {
        return;
    }
    let limits = limits();
    let mut rng = Rng::new(0x5EED_0040);
    let (mut seen, mut runs) = (0usize, 0usize);
    for path in files {
        let Ok(base) = std::fs::read(&path) else {
            continue;
        };
        if base.len() > 24 << 20 {
            continue;
        }
        seen += 1;
        // Untouched first: the file itself must be handled.
        within(Duration::from_secs(60), "real file", || {
            exercise(&base, &limits)
        });
        for _ in 0..40 {
            let mut m = mutate(&mut rng, &base);
            if rng.below(3) == 0 {
                m = mutate(&mut rng, &m);
            }
            within(Duration::from_secs(60), "mutated real file", || {
                exercise(&m, &limits)
            });
            runs += 1;
        }
        // Compound-file header words, set to boundary values one at a time.
        if base.starts_with(&[0xD0, 0xCF, 0x11, 0xE0]) {
            for at in (0..0x200.min(base.len().saturating_sub(4))).step_by(4) {
                for v in [0u32, 1, 0xFFFF_FFFF, 0xFFFF_FFFE, 0x8000_0000] {
                    let mut m = base.clone();
                    m[at..at + 4].copy_from_slice(&v.to_le_bytes());
                    within(Duration::from_secs(60), "inflated header word", || {
                        exercise(&m, &limits)
                    });
                    runs += 1;
                }
            }
        }
    }
    println!("{seen} files, {runs} mutations");
}
