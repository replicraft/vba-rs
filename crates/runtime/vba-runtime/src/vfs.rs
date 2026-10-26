//! The virtual file system the file statements run against.
//!
//! A [`Vfs`] sees only canonical absolute paths produced by [`normalize`]:
//! forward slashes, no `.` or `..` segments, an optional lower-case drive
//! (`c:/dir/file`). Windows paths written with backslashes are converted.
//! The host policy is checked by the caller (see `files`), never by a
//! `Vfs`.

use std::fmt;

/// Failures of a file system operation. The caller maps them to VBA errors
/// (53, 58, 70, 75, 76, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VfsError {
    /// The file or directory does not exist.
    NotFound,
    /// The parent directory does not exist.
    PathNotFound,
    /// The target already exists.
    AlreadyExists,
    /// A directory was used as a file.
    IsADirectory,
    /// A file was used as a directory.
    NotADirectory,
    /// `RmDir` of a directory that is not empty.
    NotEmpty,
    /// The file system refused the operation (read-only, no permission).
    Denied,
    /// The file is larger than the caller allows to be read.
    TooLarge,
    /// The operation is not available on this file system.
    Unsupported(String),
    /// Any other failure.
    Other(String),
}

impl fmt::Display for VfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VfsError::NotFound => f.write_str("not found"),
            VfsError::PathNotFound => f.write_str("path not found"),
            VfsError::AlreadyExists => f.write_str("already exists"),
            VfsError::IsADirectory => f.write_str("is a directory"),
            VfsError::NotADirectory => f.write_str("not a directory"),
            VfsError::NotEmpty => f.write_str("directory not empty"),
            VfsError::Denied => f.write_str("access denied"),
            VfsError::TooLarge => f.write_str("file too large"),
            VfsError::Unsupported(s) => write!(f, "unsupported: {s}"),
            VfsError::Other(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for VfsError {}

/// Result alias of the file system operations.
pub type VfsResult<T> = Result<T, VfsError>;

/// `vbReadOnly`.
pub const ATTR_READONLY: u8 = 1;
/// `vbHidden`.
pub const ATTR_HIDDEN: u8 = 2;
/// `vbSystem`.
pub const ATTR_SYSTEM: u8 = 4;
/// `vbDirectory`.
pub const ATTR_DIRECTORY: u8 = 16;
/// `vbArchive`.
pub const ATTR_ARCHIVE: u8 = 32;

/// What `stat` reports.
#[derive(Debug, Clone, PartialEq)]
pub struct Stat {
    /// A directory.
    pub is_dir: bool,
    /// Size in bytes (0 for directories).
    pub len: u64,
    /// Last modification as an OLE date serial.
    pub modified: f64,
    /// `GetAttr` bits (`ATTR_*`).
    pub attributes: u8,
}

/// An entry of a directory listing.
#[derive(Debug, Clone, PartialEq)]
pub struct DirEntry {
    /// The entry name (no path).
    pub name: String,
    /// Its metadata.
    pub stat: Stat,
}

/// A file system. All paths are canonical (see [`normalize`]).
pub trait Vfs {
    /// Called when the file system joins a run: an implementation that keeps
    /// contents in memory charges them to `meter` (`max_memory_bytes`) and
    /// credits them when they are removed. The default does nothing.
    fn set_memory_meter(&mut self, _meter: &vba_core::MemoryMeter) {}

    /// Maps a normalized path to the form the policy is checked against:
    /// an implementation backed by a real file system resolves symbolic
    /// links here so a link cannot escape a scoped grant. The default is the
    /// identity.
    fn canonical(&mut self, path: &str) -> String {
        path.to_owned()
    }

    /// The directory a relative path is resolved against at start.
    fn initial_dir(&self) -> String {
        "/".to_owned()
    }

    /// Metadata, or `None` if the path does not exist.
    fn stat(&mut self, path: &str) -> VfsResult<Option<Stat>>;

    /// Reads a whole file.
    fn read(&mut self, path: &str) -> VfsResult<Vec<u8>>;

    /// Reads a whole file of at most `max` bytes; a larger file is
    /// [`VfsError::TooLarge`]. An implementation over a real file system must
    /// not read more than `max + 1` bytes, whatever the file claims to be.
    fn read_limited(&mut self, path: &str, max: usize) -> VfsResult<Vec<u8>> {
        let data = self.read(path)?;
        if data.len() > max {
            return Err(VfsError::TooLarge);
        }
        Ok(data)
    }

    /// Creates or replaces a file. The parent directory must exist.
    fn write(&mut self, path: &str, data: &[u8]) -> VfsResult<()>;

    /// Deletes a file.
    fn remove_file(&mut self, path: &str) -> VfsResult<()>;

    /// Renames or moves a file or directory; the target must not exist.
    fn rename(&mut self, from: &str, to: &str) -> VfsResult<()>;

    /// Copies a file, replacing the target.
    fn copy(&mut self, from: &str, to: &str) -> VfsResult<()>;

    /// Creates a directory whose parent exists.
    fn create_dir(&mut self, path: &str) -> VfsResult<()>;

    /// Removes an empty directory.
    fn remove_dir(&mut self, path: &str) -> VfsResult<()>;

    /// Lists a directory, sorted by name (case-insensitive).
    fn list(&mut self, dir: &str) -> VfsResult<Vec<DirEntry>>;

    /// Sets the attribute bits (`ATTR_READONLY`, `ATTR_HIDDEN`, ...).
    fn set_attributes(&mut self, path: &str, attributes: u8) -> VfsResult<()>;
}

/// Normalizes `path` against `cwd` into the canonical form: backslashes
/// become slashes, `.` and `..` are resolved lexically (`..` stops at the
/// root), a drive letter is lower-cased. The result is absolute.
///
/// A network path (two separators and then a name: `\\server\share\x`)
/// stays `//server/share/x`, and `..` cannot climb above the share. A device
/// namespace (`\\.\` or `\\?\`) keeps its `.` or `?` as the server, so
/// that nothing mistakes it for a plain name. Three or more separators are
/// one.
pub fn normalize(cwd: &str, path: &str) -> String {
    let path = path.replace('\\', "/");
    let unc_input = path.starts_with("//") && path.as_bytes().get(2).is_some_and(|c| *c != b'/');
    let cwd = cwd.replace('\\', "/");
    let cwd_unc = cwd.starts_with("//") && cwd.as_bytes().get(2).is_some_and(|c| *c != b'/');
    if unc_input {
        return normalize_unc(&[&path[2..]]);
    }
    let (drive, rest, absolute) = split_drive(&path);
    if cwd_unc && !absolute && drive.is_none() {
        return normalize_unc(&[&cwd[2..], rest]);
    }
    let cwd = if cwd_unc { "/".to_owned() } else { cwd };
    let (cwd_drive, cwd_rest, _) = split_drive(&cwd);
    // `x:rel` (a drive without a root) is relative to the one working
    // directory; a path without a drive keeps the working directory's drive.
    let relative = !absolute;
    let drive = drive.or(cwd_drive);
    let mut segs: Vec<&str> = Vec::new();
    let parts = relative
        .then_some(cwd_rest)
        .into_iter()
        .chain(std::iter::once(rest));
    for part in parts {
        for s in part.split('/') {
            match s {
                "" | "." => {}
                ".." => {
                    segs.pop();
                }
                s => segs.push(s),
            }
        }
    }
    let mut out = String::new();
    if let Some(d) = drive {
        out.push(d);
        out.push(':');
    }
    out.push('/');
    out.push_str(&segs.join("/"));
    out
}

/// `//server/share/...` from path pieces joined in order; the server and
/// the share are never removed by `..`.
fn normalize_unc(parts: &[&str]) -> String {
    let mut segs: Vec<&str> = Vec::new();
    for part in parts {
        for s in part.split('/') {
            match s {
                "" => {}
                "." if !segs.is_empty() => {}
                ".." => {
                    if segs.len() > 2 {
                        segs.pop();
                    }
                }
                s => segs.push(s),
            }
        }
    }
    format!("//{}", segs.join("/"))
}

fn split_drive(p: &str) -> (Option<char>, &str, bool) {
    let b = p.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        let rest = &p[2..];
        (
            Some(b[0].to_ascii_lowercase() as char),
            rest.trim_start_matches('/'),
            rest.starts_with('/'),
        )
    } else {
        (None, p.trim_start_matches('/'), p.starts_with('/'))
    }
}

/// The parent directory and the last segment of a canonical path.
pub fn split_parent(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(0) => ("/".to_owned(), path[1..].to_owned()),
        Some(i) => {
            let parent = &path[..i];
            let parent = if parent.ends_with(':') {
                format!("{parent}/")
            } else {
                parent.to_owned()
            };
            (parent, path[i + 1..].to_owned())
        }
        None => ("/".to_owned(), path.to_owned()),
    }
}

/// Joins a directory and a name.
pub fn join(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

/// Case-insensitive wildcard match with `*` and `?` (as used by `Dir` and
/// `Kill`).
pub fn wildcard(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    let (mut pi, mut ni) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_paths() {
        assert_eq!(normalize("/", "a.txt"), "/a.txt");
        assert_eq!(normalize("/work", "a/../b\\c.txt"), "/work/b/c.txt");
        assert_eq!(normalize("/work", "/x/./y"), "/x/y");
        assert_eq!(normalize("/work", "../../.."), "/");
        assert_eq!(normalize("/", "C:\\Data\\File.TXT"), "c:/Data/File.TXT");
        assert_eq!(normalize("c:/data", "x.txt"), "c:/data/x.txt");
        assert_eq!(normalize("c:/data", "/x.txt"), "c:/x.txt");
        assert_eq!(normalize("/", "D:"), "d:/");
        // A drive-relative path (`a:x`) is resolved against the one working
        // directory.
        assert_eq!(normalize("/w", "d:x"), "d:/w/x");
        assert_eq!(normalize("..", ":"), "/:");
        // A drive-relative path (`a:x`) is resolved against the one working
        // directory; dots in the working directory are ignored.
        assert_eq!(normalize(".", "A::"), "a:/:");
    }

    #[test]
    fn unc_paths_keep_their_server_and_share() {
        assert_eq!(normalize("/", "\\\\srv\\share\\a\\b"), "//srv/share/a/b");
        assert_eq!(
            normalize("/w", "\\\\srv\\share\\a\\..\\..\\..\\b"),
            "//srv/share/b"
        );
        assert_eq!(normalize("c:/w", "\\/srv/share/x"), "//srv/share/x");
        assert_eq!(normalize("/", "\\\\srv"), "//srv");
        assert_eq!(normalize("/", "//srv/share/a"), "//srv/share/a");
        assert_eq!(normalize("/", "///srv/share/a"), "/srv/share/a");
        assert_eq!(normalize("/", "\\\\.\\pipe\\x"), "//./pipe/x");
        assert_eq!(normalize("/", "\\\\?\\c:\\x"), "//?/c:/x");
        // A UNC working directory is kept for relative paths.
        assert_eq!(normalize("//srv/share/d", "x"), "//srv/share/d/x");
        assert_eq!(normalize("//srv/share/d", "../../x"), "//srv/share/x");
    }

    #[test]
    fn parent_and_join() {
        assert_eq!(split_parent("/a/b"), ("/a".into(), "b".into()));
        assert_eq!(split_parent("/a"), ("/".into(), "a".into()));
        assert_eq!(split_parent("c:/a"), ("c:/".into(), "a".into()));
        assert_eq!(join("/", "a"), "/a");
        assert_eq!(join("/a", "b"), "/a/b");
    }

    #[test]
    fn wildcards() {
        assert!(wildcard("*.txt", "A.TXT"));
        assert!(wildcard("a?c", "abc"));
        assert!(!wildcard("a?c", "ac"));
        assert!(wildcard("*", ""));
        assert!(wildcard("a*b*c", "aXXbYYc"));
        assert!(!wildcard("a*b", "abc"));
    }
}
