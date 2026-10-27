//! Writes the modules of Office documents and `vbaProject.bin` files out as
//! `.bas`, `.cls` and `.frm` files, the input of the `source_corpus` test
//! (see `docs/tooling/source-corpus.md`).
//!
//! ```text
//! cargo run -p vba-rs --example export_modules -- <input> <output directory>
//! ```
//!
//! `<input>` is a file or a directory searched recursively. Every readable
//! document becomes a numbered subdirectory of the output directory with one
//! file per module that has source, a `project.txt` (project name and
//! conditional-compilation line) and, in the output directory, a
//! `manifest.tsv` of `path<TAB>module kind` lines. The files are only read:
//! nothing in them is ever run. Documents the reader refuses are listed on
//! standard error and make the exit code 1.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vba_format::{Limits, ReadMode, read_document};

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        if let Ok(rd) = fs::read_dir(path) {
            for entry in rd.flatten() {
                collect(&entry.path(), out);
            }
        }
    } else {
        out.push(path.to_path_buf());
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let [_, input, output] = args.as_slice() else {
        eprintln!("usage: export_modules <input file or directory> <output directory>");
        return ExitCode::from(2);
    };
    let mut files = Vec::new();
    collect(Path::new(input), &mut files);
    files.sort();
    let output = Path::new(output);
    let mut manifest = String::new();
    let (mut projects, mut failed) = (0, 0);
    for file in files {
        let Ok(bytes) = fs::read(&file) else {
            eprintln!("cannot read {}", file.display());
            failed += 1;
            continue;
        };
        let limits = Limits::default();
        let mut read = read_document(&bytes, &limits, ReadMode::Strict);
        if let Err(strict) = &read {
            // Unknown records in the `dir` stream are an error in strict
            // mode; report it and read the document anyway.
            eprintln!("strict mode refused {}: {strict}", file.display());
            read = read_document(&bytes, &limits, ReadMode::Tolerant);
        }
        match read {
            Ok((project, location)) => {
                projects += 1;
                let dir = output.join(format!("{projects:03}"));
                let written = fs::create_dir_all(&dir).and_then(|()| {
                    fs::write(
                        dir.join("project.txt"),
                        format!("{}\n{}\n", project.name, project.constants),
                    )
                });
                if let Err(e) = written {
                    eprintln!("cannot write {}: {e}", dir.display());
                    return ExitCode::FAILURE;
                }
                let mut modules = 0;
                for m in project.modules.iter().filter(|m| m.has_source) {
                    let name = format!("{}.{}", m.name, m.file_extension());
                    if let Err(e) = fs::write(dir.join(&name), m.to_source_file()) {
                        eprintln!("cannot write {name}: {e}");
                        return ExitCode::FAILURE;
                    }
                    manifest.push_str(&format!("{projects:03}/{name}\t{}\n", m.kind.name()));
                    modules += 1;
                }
                eprintln!("{projects:03}: {modules} modules ({location:?})");
            }
            Err(e) => {
                eprintln!("FAILED {}: {e}", file.display());
                failed += 1;
            }
        }
    }
    if let Err(e) = fs::write(output.join("manifest.tsv"), manifest) {
        eprintln!("cannot write the manifest: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("{projects} projects read, {failed} documents failed");
    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
