//! Type libraries named by `vba.typeLibraries`, loaded into one provider.
//!
//! Loading never panics and never fails as a whole: a file that is missing,
//! too large or not a type library is reported and skipped, the rest still
//! load. Files are read through a size cap, so a path to a huge or endless
//! file cannot exhaust memory.

use std::io::Read;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use vba_lsp_core::SharedProvider;
use vba_typelib::sema::TypelibProvider;

/// Largest file read (bytes). Real libraries are well below this.
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
/// Most paths taken from the setting; the rest are reported and ignored.
pub const MAX_PATHS: usize = 64;
/// Most libraries loaded in total (a PE image can hold several).
pub const MAX_LIBRARIES: usize = 256;

/// What loading produced.
pub struct Loaded {
    /// The provider, `None` when no library could be loaded.
    pub provider: Option<SharedProvider>,
    /// Number of libraries loaded.
    pub libraries: usize,
    /// One message per problem.
    pub errors: Vec<String>,
}

/// Loads every library of every path (raw `.tlb`/`.olb` of either format, or
/// the `TYPELIB` resources of a PE image).
pub fn load(paths: &[PathBuf]) -> Loaded {
    let mut errors = Vec::new();
    let mut provider = TypelibProvider::new();
    let mut libraries = 0;
    for path in paths {
        let shown = path.display();
        let data = match read_capped(path, MAX_FILE_BYTES) {
            Ok(d) => d,
            Err(e) => {
                errors.push(format!("type library `{shown}`: {e}"));
                continue;
            }
        };
        match catch_unwind(AssertUnwindSafe(|| vba_typelib::load_all(&data))) {
            Ok(Ok(libs)) => {
                for (_, lib) in libs {
                    if libraries >= MAX_LIBRARIES {
                        errors.push(format!(
                            "type library `{shown}`: more than {MAX_LIBRARIES} libraries, the rest are ignored"
                        ));
                        break;
                    }
                    provider.add_library(lib);
                    libraries += 1;
                }
            }
            Ok(Err(e)) => errors.push(format!("type library `{shown}`: {e}")),
            Err(_) => errors.push(format!(
                "type library `{shown}`: the file could not be read"
            )),
        }
    }
    Loaded {
        provider: (libraries > 0).then(|| Arc::new(provider) as SharedProvider),
        libraries,
        errors,
    }
}

fn read_capped(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > limit {
        return Err(format!("larger than {limit} bytes, skipped"));
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_files_are_refused() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let len = std::fs::metadata(&p).unwrap().len();
        assert_eq!(read_capped(&p, len).unwrap().len() as u64, len);
        assert!(read_capped(&p, len - 1).is_err());
    }

    #[test]
    fn nothing_loads_from_nothing() {
        let l = load(&[PathBuf::from("does-not-exist.tlb")]);
        assert!(l.provider.is_none() && l.libraries == 0);
        assert_eq!(l.errors.len(), 1);
    }
}
