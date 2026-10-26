//! The file system the `Scripting.FileSystemObject` objects work on, and
//! small path helpers.
//!
//! [`ScriptFs`] is the only way these objects reach files. An implementation
//! checks the host policy (`FileRead` / `FileWrite`, with prefix scoping)
//! for every call and reports refusals as error 70; the objects never see a
//! real path they were not allowed to touch.

use vba_automation::ErrInfo;

/// Metadata of a file or folder.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryStat {
    /// A folder.
    pub is_dir: bool,
    /// Size in bytes (0 for folders).
    pub len: u64,
    /// Last modification as an OLE date serial.
    pub modified: f64,
    /// `GetAttr` bits (`vbReadOnly` 1, `vbHidden` 2, `vbSystem` 4,
    /// `vbDirectory` 16, `vbArchive` 32).
    pub attributes: u8,
}

/// An entry of a folder listing.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryInfo {
    /// The name without the path.
    pub name: String,
    /// Its metadata.
    pub stat: EntryStat,
}

/// Result of a [`ScriptFs`] operation: failures are complete VBA errors
/// (53 file not found, 76 path not found, 58 file exists, 70 permission
/// denied, ...).
pub type FsResult<T> = Result<T, ErrInfo>;

/// A policy-checked virtual file system.
///
/// Paths passed to every method except [`ScriptFs::resolve`] are the
/// absolute, canonical paths `resolve` returned.
pub trait ScriptFs {
    /// Makes a path absolute against the working directory and canonical.
    fn resolve(&self, path: &str) -> FsResult<String>;
    /// Metadata, or `None` when the path does not exist (needs `FileRead`).
    fn stat(&self, abs: &str) -> FsResult<Option<EntryStat>>;
    /// Reads a file (needs `FileRead`).
    fn read(&self, abs: &str) -> FsResult<Vec<u8>>;
    /// Creates or replaces a file (needs `FileWrite`).
    fn write(&self, abs: &str, data: &[u8]) -> FsResult<()>;
    /// Deletes a file (needs `FileWrite`).
    fn remove_file(&self, abs: &str) -> FsResult<()>;
    /// Removes an empty folder (needs `FileWrite`).
    fn remove_dir(&self, abs: &str) -> FsResult<()>;
    /// Creates a folder whose parent exists (needs `FileWrite`).
    fn create_dir(&self, abs: &str) -> FsResult<()>;
    /// Copies a file, replacing the target (needs `FileRead` and `FileWrite`).
    fn copy_file(&self, from: &str, to: &str) -> FsResult<()>;
    /// Renames or moves a file or folder (needs `FileWrite`).
    fn rename(&self, from: &str, to: &str) -> FsResult<()>;
    /// Lists a folder, sorted by name (needs `FileRead`).
    fn list(&self, abs: &str) -> FsResult<Vec<EntryInfo>>;
    /// Sets the attribute bits (needs `FileWrite`).
    fn set_attributes(&self, abs: &str, attributes: u8) -> FsResult<()>;
}

/// The index of the last path separator (`/` or `\`).
fn last_sep(path: &str) -> Option<usize> {
    path.rfind(['/', '\\'])
}

/// The last component of a path (`GetFileName`).
pub(crate) fn file_name(path: &str) -> &str {
    match last_sep(path) {
        Some(i) => &path[i + 1..],
        None => match path.find(':') {
            Some(1) => &path[2..],
            _ => path,
        },
    }
}

/// The part before the last separator (`GetParentFolderName`); empty when
/// there is none. The root is its own parent-less folder (empty result).
pub(crate) fn parent_name(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    match last_sep(trimmed) {
        Some(0) => path[..1].to_owned(),
        Some(i) => {
            let p = &trimmed[..i];
            if p.ends_with(':') && p.len() == 2 {
                format!("{p}{}", &trimmed[i..=i])
            } else {
                p.to_owned()
            }
        }
        None => String::new(),
    }
}

/// The file name without its last extension (`GetBaseName`).
pub(crate) fn base_name(path: &str) -> String {
    let n = file_name(path.trim_end_matches(['/', '\\']));
    match n.rfind('.') {
        Some(i) => n[..i].to_owned(),
        None => n.to_owned(),
    }
}

/// The extension after the last dot of the last component (`GetExtensionName`).
pub(crate) fn extension(path: &str) -> String {
    let n = file_name(path);
    match n.rfind('.') {
        Some(i) => n[i + 1..].to_owned(),
        None => String::new(),
    }
}

/// Appends `name` to `path` with a separator only when needed
/// (`BuildPath`). The separator is `/` when `path` uses forward slashes
/// only, `\` otherwise.
pub(crate) fn build_path(path: &str, name: &str) -> String {
    if path.is_empty() {
        return name.to_owned();
    }
    if name.is_empty() {
        return path.to_owned();
    }
    let sep = if path.contains('/') && !path.contains('\\') {
        '/'
    } else {
        '\\'
    };
    let ends = path.ends_with(['/', '\\']);
    let starts = name.starts_with(['/', '\\']);
    match (ends, starts) {
        (true, true) => format!("{path}{}", &name[1..]),
        (false, false) => format!("{path}{sep}{name}"),
        _ => format!("{path}{name}"),
    }
}

/// The last component of a canonical path.
pub(crate) fn last_component(abs: &str) -> &str {
    match abs.rfind('/') {
        Some(i) if i + 1 < abs.len() => &abs[i + 1..],
        _ => "",
    }
}

/// The parent of a canonical path, `None` for a root.
pub(crate) fn parent_of(abs: &str) -> Option<String> {
    let i = abs.rfind('/')?;
    if i + 1 >= abs.len() {
        return None;
    }
    if i == 0 {
        Some("/".to_owned())
    } else if abs[..i].ends_with(':') {
        Some(format!("{}/", &abs[..i]))
    } else {
        Some(abs[..i].to_owned())
    }
}

/// Joins a canonical folder and a name.
pub(crate) fn join(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

/// Case-insensitive wildcard match with `*` and `?`.
pub(crate) fn wildcard(pattern: &str, name: &str) -> bool {
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
    fn name_functions() {
        assert_eq!(file_name("c:\\dir\\a.txt"), "a.txt");
        assert_eq!(file_name("/dir/a.b.txt"), "a.b.txt");
        assert_eq!(file_name("a.txt"), "a.txt");
        assert_eq!(base_name("/dir/a.b.txt"), "a.b");
        assert_eq!(base_name("noext"), "noext");
        assert_eq!(extension("/dir/a.b.txt"), "txt");
        assert_eq!(extension("/dir.d/noext"), "");
        assert_eq!(parent_name("/dir/a.txt"), "/dir");
        assert_eq!(parent_name("c:\\dir\\a.txt"), "c:\\dir");
        assert_eq!(parent_name("/a.txt"), "/");
        assert_eq!(parent_name("a.txt"), "");
    }

    #[test]
    fn build_path_inserts_one_separator() {
        assert_eq!(build_path("c:\\dir", "a.txt"), "c:\\dir\\a.txt");
        assert_eq!(build_path("c:\\dir\\", "a.txt"), "c:\\dir\\a.txt");
        assert_eq!(build_path("/dir", "a.txt"), "/dir/a.txt");
        assert_eq!(build_path("/dir/", "/a.txt"), "/dir/a.txt");
        assert_eq!(build_path("", "a"), "a");
    }

    #[test]
    fn canonical_helpers() {
        assert_eq!(parent_of("/a/b"), Some("/a".into()));
        assert_eq!(parent_of("/a"), Some("/".into()));
        assert_eq!(parent_of("/"), None);
        assert_eq!(parent_of("c:/a"), Some("c:/".into()));
        assert_eq!(last_component("/a/b"), "b");
        assert!(wildcard("*.TXT", "a.txt"));
    }
}
