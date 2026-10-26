//! An opt-in [`Vfs`] over the real file system.
//!
//! Using a `RealFs` is the embedder's decision; the VBA program still needs
//! `FileRead` / `FileWrite` grants on the policy of the host, checked
//! against [`Vfs::canonical`] — the path with symbolic links resolved — so a
//! link inside a granted directory cannot reach outside it.

use std::fs;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::vfs::*;

/// The real file system, starting in the process working directory.
#[derive(Debug, Clone)]
pub struct RealFs {
    initial_dir: String,
}

impl Default for RealFs {
    fn default() -> Self {
        RealFs::new()
    }
}

fn to_canonical_text(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let s = s.strip_prefix("//?/").unwrap_or(&s).to_owned();
    match s.as_bytes() {
        [d, b':', ..] if d.is_ascii_alphabetic() => {
            format!("{}{}", (*d as char).to_ascii_lowercase(), &s[1..])
        }
        _ => s,
    }
}

fn map_err(e: &std::io::Error) -> VfsError {
    match e.kind() {
        ErrorKind::NotFound => VfsError::NotFound,
        ErrorKind::AlreadyExists => VfsError::AlreadyExists,
        ErrorKind::PermissionDenied => VfsError::Denied,
        ErrorKind::DirectoryNotEmpty => VfsError::NotEmpty,
        ErrorKind::IsADirectory => VfsError::IsADirectory,
        ErrorKind::NotADirectory => VfsError::NotADirectory,
        _ => VfsError::Other(e.to_string()),
    }
}

/// Refuses devices, pipes and sockets (`/dev/zero` never ends, a FIFO blocks
/// the interpreter). A missing path and a directory pass; callers deal with
/// them.
fn ensure_not_special(path: &str) -> VfsResult<()> {
    match fs::metadata(path) {
        Ok(m) if !m.is_file() && !m.is_dir() => Err(VfsError::Denied),
        _ => Ok(()),
    }
}

/// Symbolic links followed per path before giving up.
const MAX_LINK_HOPS: usize = 40;

/// Resolves `p` whose parent exists (or not) through links that may dangle:
/// canonicalizing the parent is not enough, because writing through a link
/// whose target does not exist yet creates the target.
fn resolve_dangling(p: &Path) -> Option<PathBuf> {
    let mut cur = p.to_path_buf();
    for _ in 0..MAX_LINK_HOPS {
        let (parent, name) = (cur.parent()?, cur.file_name()?);
        let parent = fs::canonicalize(parent).ok()?;
        let candidate = parent.join(name);
        match fs::symlink_metadata(&candidate) {
            Ok(m) if m.file_type().is_symlink() => {
                let target = fs::read_link(&candidate).ok()?;
                cur = if target.is_absolute() {
                    target
                } else {
                    parent.join(target)
                };
                if let Ok(c) = fs::canonicalize(&cur) {
                    return Some(c);
                }
            }
            _ => return Some(candidate),
        }
    }
    None
}

fn ole_date(t: SystemTime) -> f64 {
    let secs = match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs_f64(),
        Err(e) => -e.duration().as_secs_f64(),
    };
    25569.0 + secs / 86_400.0
}

fn stat_of(m: &fs::Metadata) -> Stat {
    let mut attributes = if m.is_dir() {
        ATTR_DIRECTORY
    } else {
        ATTR_ARCHIVE
    };
    if m.permissions().readonly() {
        attributes |= ATTR_READONLY;
    }
    Stat {
        is_dir: m.is_dir(),
        len: if m.is_dir() { 0 } else { m.len() },
        modified: m.modified().map(ole_date).unwrap_or(25569.0),
        attributes,
    }
}

impl RealFs {
    /// A real file system whose working directory is the process's.
    pub fn new() -> RealFs {
        let dir = std::env::current_dir()
            .map(|d| to_canonical_text(&d))
            .unwrap_or_else(|_| "/".to_owned());
        RealFs { initial_dir: dir }
    }

    /// A real file system with a given starting directory.
    pub fn in_dir(dir: &Path) -> RealFs {
        RealFs {
            initial_dir: to_canonical_text(dir),
        }
    }
}

impl Vfs for RealFs {
    fn canonical(&mut self, path: &str) -> String {
        let p = PathBuf::from(path);
        if let Ok(c) = fs::canonicalize(&p) {
            return to_canonical_text(&c);
        }
        // A path that does not exist yet, or a link whose target does not:
        // resolve the parent and follow the link chain by hand.
        if let Some(c) = resolve_dangling(&p) {
            return to_canonical_text(&c);
        }
        path.to_owned()
    }

    fn initial_dir(&self) -> String {
        self.initial_dir.clone()
    }

    fn stat(&mut self, path: &str) -> VfsResult<Option<Stat>> {
        match fs::metadata(path) {
            Ok(m) => Ok(Some(stat_of(&m))),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(map_err(&e)),
        }
    }

    fn read(&mut self, path: &str) -> VfsResult<Vec<u8>> {
        self.read_limited(path, usize::MAX - 1)
    }

    fn read_limited(&mut self, path: &str, max: usize) -> VfsResult<Vec<u8>> {
        if fs::metadata(path).is_ok_and(|m| m.is_dir()) {
            return Err(VfsError::IsADirectory);
        }
        ensure_not_special(path)?;
        let file = fs::File::open(path).map_err(|e| map_err(&e))?;
        // Checked on the open handle too, so a swap after the test above
        // cannot turn the read into a device read.
        if !file.metadata().map_err(|e| map_err(&e))?.is_file() {
            return Err(VfsError::Denied);
        }
        let mut out = Vec::new();
        file.take(max as u64 + 1)
            .read_to_end(&mut out)
            .map_err(|e| map_err(&e))?;
        if out.len() > max {
            return Err(VfsError::TooLarge);
        }
        Ok(out)
    }

    fn write(&mut self, path: &str, data: &[u8]) -> VfsResult<()> {
        let (parent, _) = split_parent(path);
        if !fs::metadata(&parent).is_ok_and(|m| m.is_dir()) {
            return Err(VfsError::PathNotFound);
        }
        if fs::metadata(path).is_ok_and(|m| m.is_dir()) {
            return Err(VfsError::IsADirectory);
        }
        ensure_not_special(path)?;
        fs::write(path, data).map_err(|e| map_err(&e))
    }

    fn remove_file(&mut self, path: &str) -> VfsResult<()> {
        if fs::metadata(path).is_ok_and(|m| m.is_dir()) {
            return Err(VfsError::IsADirectory);
        }
        fs::remove_file(path).map_err(|e| map_err(&e))
    }

    fn rename(&mut self, from: &str, to: &str) -> VfsResult<()> {
        if fs::symlink_metadata(to).is_ok() {
            return Err(VfsError::AlreadyExists);
        }
        fs::rename(from, to).map_err(|e| map_err(&e))
    }

    fn copy(&mut self, from: &str, to: &str) -> VfsResult<()> {
        if fs::metadata(from).is_ok_and(|m| m.is_dir()) {
            return Err(VfsError::IsADirectory);
        }
        ensure_not_special(from)?;
        ensure_not_special(to)?;
        fs::copy(from, to).map(|_| ()).map_err(|e| map_err(&e))
    }

    fn create_dir(&mut self, path: &str) -> VfsResult<()> {
        let (parent, _) = split_parent(path);
        if fs::symlink_metadata(path).is_ok() {
            return Err(VfsError::AlreadyExists);
        }
        if !fs::metadata(&parent).is_ok_and(|m| m.is_dir()) {
            return Err(VfsError::PathNotFound);
        }
        fs::create_dir(path).map_err(|e| map_err(&e))
    }

    fn remove_dir(&mut self, path: &str) -> VfsResult<()> {
        match fs::metadata(path) {
            Ok(m) if m.is_dir() => fs::remove_dir(path).map_err(|e| map_err(&e)),
            Ok(_) => Err(VfsError::NotADirectory),
            Err(e) => Err(map_err(&e)),
        }
    }

    fn list(&mut self, dir: &str) -> VfsResult<Vec<DirEntry>> {
        let rd = fs::read_dir(dir).map_err(|e| match e.kind() {
            ErrorKind::NotFound => VfsError::PathNotFound,
            _ => map_err(&e),
        })?;
        let mut v = Vec::new();
        for e in rd.flatten() {
            if let Ok(m) = fs::metadata(e.path()) {
                v.push(DirEntry {
                    name: e.file_name().to_string_lossy().into_owned(),
                    stat: stat_of(&m),
                });
            }
        }
        v.sort_by_key(|e| e.name.to_lowercase());
        Ok(v)
    }

    fn set_attributes(&mut self, path: &str, attributes: u8) -> VfsResult<()> {
        if attributes & (ATTR_HIDDEN | ATTR_SYSTEM) != 0 {
            return Err(VfsError::Unsupported(
                "the hidden and system attributes".into(),
            ));
        }
        let m = fs::metadata(path).map_err(|e| map_err(&e))?;
        let mut perm = m.permissions();
        perm.set_readonly(attributes & ATTR_READONLY != 0);
        fs::set_permissions(path, perm).map_err(|e| map_err(&e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_in_a_temporary_directory() {
        let dir = tempfile::tempdir().unwrap();
        let base = to_canonical_text(&fs::canonicalize(dir.path()).unwrap());
        let mut fs = RealFs::in_dir(dir.path());
        let file = format!("{base}/a.txt");
        assert_eq!(fs.write(&file, b"hello"), Ok(()));
        assert_eq!(fs.read(&file), Ok(b"hello".to_vec()));
        assert_eq!(fs.stat(&file).unwrap().unwrap().len, 5);
        assert_eq!(
            fs.write(&format!("{base}/missing/a.txt"), b""),
            Err(VfsError::PathNotFound)
        );
        let sub = format!("{base}/sub");
        assert_eq!(fs.create_dir(&sub), Ok(()));
        assert_eq!(fs.create_dir(&sub), Err(VfsError::AlreadyExists));
        let names: Vec<_> = fs
            .list(&base)
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, ["a.txt", "sub"]);
        assert_eq!(fs.remove_file(&file), Ok(()));
        assert_eq!(fs.remove_dir(&sub), Ok(()));
        assert_eq!(fs.stat(&file), Ok(None));
    }

    #[cfg(unix)]
    #[test]
    fn canonical_resolves_symbolic_links() {
        let dir = tempfile::tempdir().unwrap();
        let real = fs::canonicalize(dir.path()).unwrap();
        fs::create_dir(real.join("target")).unwrap();
        std::os::unix::fs::symlink(real.join("target"), real.join("link")).unwrap();
        let mut f = RealFs::new();
        let via_link = format!("{}/link/new.txt", to_canonical_text(&real));
        assert_eq!(
            f.canonical(&via_link),
            format!("{}/target/new.txt", to_canonical_text(&real))
        );
    }
}
