//! An in-memory [`Vfs`]: case-insensitive, case-preserving, deterministic.

use std::collections::BTreeMap;

use crate::vfs::*;

#[derive(Debug, Clone)]
enum Node {
    File {
        data: Vec<u8>,
        attributes: u8,
        modified: f64,
    },
    Dir {
        attributes: u8,
        modified: f64,
    },
}

#[derive(Debug, Clone)]
struct Entry {
    /// The path with its original spelling.
    display: String,
    node: Node,
}

/// An in-memory file system. Drive roots (`c:/`) and `/` always exist.
/// Timestamps are the value of [`MemFs::set_time`] (2000-01-01 by default),
/// so runs are reproducible.
#[derive(Debug, Clone)]
pub struct MemFs {
    entries: BTreeMap<String, Entry>,
    time: f64,
    initial_dir: String,
}

impl Default for MemFs {
    fn default() -> Self {
        MemFs::new()
    }
}

fn key(path: &str) -> String {
    path.to_lowercase()
}

impl MemFs {
    /// An empty file system whose working directory is `/`.
    pub fn new() -> MemFs {
        MemFs {
            entries: BTreeMap::new(),
            time: 36526.0,
            initial_dir: "/".to_owned(),
        }
    }

    /// Sets the timestamp given to files and directories from now on.
    pub fn set_time(&mut self, serial: f64) {
        self.time = serial;
    }

    /// Sets the directory the VBA program starts in (created if needed).
    #[must_use]
    pub fn with_initial_dir(mut self, dir: &str) -> MemFs {
        let d = normalize("/", dir);
        self.make_dirs(&d);
        self.initial_dir = d;
        self
    }

    /// Adds a file (creating missing parent directories); convenient for
    /// setting up tests.
    #[must_use]
    pub fn with_file(mut self, path: &str, data: &[u8]) -> MemFs {
        let p = normalize("/", path);
        let (parent, _) = split_parent(&p);
        self.make_dirs(&parent);
        let _ = self.write(&p, data);
        self
    }

    /// Creates a directory and all its parents.
    pub fn make_dirs(&mut self, path: &str) {
        let p = normalize("/", path);
        let (prefix, rest) = match p.find('/') {
            Some(i) => (p[..=i].to_owned(), p[i + 1..].to_owned()),
            None => (p.clone(), String::new()),
        };
        let mut cur = prefix.trim_end_matches('/').to_owned();
        for seg in rest.split('/').filter(|s| !s.is_empty()) {
            cur = format!("{cur}/{seg}");
            self.entries.entry(key(&cur)).or_insert_with(|| Entry {
                display: cur.clone(),
                node: Node::Dir {
                    attributes: ATTR_DIRECTORY,
                    modified: 36526.0,
                },
            });
        }
    }

    /// All file paths (original spelling), sorted; for inspection in tests.
    pub fn files(&self) -> Vec<String> {
        self.entries
            .values()
            .filter(|e| matches!(e.node, Node::File { .. }))
            .map(|e| e.display.clone())
            .collect()
    }

    fn is_root(path: &str) -> bool {
        path == "/" || (path.len() == 3 && path.ends_with(":/"))
    }

    fn dir_exists(&self, path: &str) -> bool {
        Self::is_root(path)
            || matches!(
                self.entries.get(&key(path)),
                Some(Entry {
                    node: Node::Dir { .. },
                    ..
                })
            )
    }

    fn children(&self, dir: &str) -> Vec<(&Entry, String)> {
        let prefix = if dir.ends_with('/') {
            key(dir)
        } else {
            format!("{}/", key(dir))
        };
        self.entries
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix) && !k[prefix.len()..].contains('/'))
            .map(|(_, e)| (e, split_parent(&e.display).1))
            .collect()
    }

    fn stat_of(node: &Node) -> Stat {
        match node {
            Node::File {
                data,
                attributes,
                modified,
            } => Stat {
                is_dir: false,
                len: data.len() as u64,
                modified: *modified,
                attributes: *attributes,
            },
            Node::Dir {
                attributes,
                modified,
            } => Stat {
                is_dir: true,
                len: 0,
                modified: *modified,
                attributes: *attributes | ATTR_DIRECTORY,
            },
        }
    }
}

impl Vfs for MemFs {
    fn initial_dir(&self) -> String {
        self.initial_dir.clone()
    }

    fn stat(&mut self, path: &str) -> VfsResult<Option<Stat>> {
        if Self::is_root(path) {
            return Ok(Some(Stat {
                is_dir: true,
                len: 0,
                modified: self.time,
                attributes: ATTR_DIRECTORY,
            }));
        }
        Ok(self.entries.get(&key(path)).map(|e| Self::stat_of(&e.node)))
    }

    fn read(&mut self, path: &str) -> VfsResult<Vec<u8>> {
        match self.entries.get(&key(path)) {
            Some(Entry {
                node: Node::File { data, .. },
                ..
            }) => Ok(data.clone()),
            Some(_) => Err(VfsError::IsADirectory),
            None => Err(VfsError::NotFound),
        }
    }

    fn write(&mut self, path: &str, data: &[u8]) -> VfsResult<()> {
        let (parent, _) = split_parent(path);
        if !self.dir_exists(&parent) {
            return Err(VfsError::PathNotFound);
        }
        match self.entries.get_mut(&key(path)) {
            Some(Entry {
                node: Node::Dir { .. },
                ..
            }) => return Err(VfsError::IsADirectory),
            Some(Entry {
                node:
                    Node::File {
                        data: d,
                        attributes,
                        modified,
                    },
                ..
            }) => {
                if *attributes & ATTR_READONLY != 0 {
                    return Err(VfsError::Denied);
                }
                *d = data.to_vec();
                *modified = self.time;
                return Ok(());
            }
            None => {}
        }
        self.entries.insert(
            key(path),
            Entry {
                display: path.to_owned(),
                node: Node::File {
                    data: data.to_vec(),
                    attributes: ATTR_ARCHIVE,
                    modified: self.time,
                },
            },
        );
        Ok(())
    }

    fn remove_file(&mut self, path: &str) -> VfsResult<()> {
        match self.entries.get(&key(path)) {
            Some(Entry {
                node: Node::File { attributes, .. },
                ..
            }) => {
                if attributes & ATTR_READONLY != 0 {
                    return Err(VfsError::Denied);
                }
                self.entries.remove(&key(path));
                Ok(())
            }
            Some(_) => Err(VfsError::IsADirectory),
            None => Err(VfsError::NotFound),
        }
    }

    fn rename(&mut self, from: &str, to: &str) -> VfsResult<()> {
        if !self.entries.contains_key(&key(from)) {
            return Err(VfsError::NotFound);
        }
        if self.entries.contains_key(&key(to)) || Self::is_root(to) {
            return Err(VfsError::AlreadyExists);
        }
        let (parent, _) = split_parent(to);
        if !self.dir_exists(&parent) {
            return Err(VfsError::PathNotFound);
        }
        let from_prefix = format!("{}/", key(from));
        let moved: Vec<String> = self
            .entries
            .keys()
            .filter(|k| **k == key(from) || k.starts_with(&from_prefix))
            .cloned()
            .collect();
        for k in moved {
            let mut e = self.entries.remove(&k).expect("listed");
            let suffix = e.display[from.len()..].to_owned();
            e.display = format!("{to}{suffix}");
            self.entries.insert(key(&e.display), e);
        }
        Ok(())
    }

    fn copy(&mut self, from: &str, to: &str) -> VfsResult<()> {
        let data = self.read(from)?;
        self.write(to, &data)
    }

    fn create_dir(&mut self, path: &str) -> VfsResult<()> {
        if Self::is_root(path) || self.entries.contains_key(&key(path)) {
            return Err(VfsError::AlreadyExists);
        }
        let (parent, _) = split_parent(path);
        if !self.dir_exists(&parent) {
            return Err(VfsError::PathNotFound);
        }
        self.entries.insert(
            key(path),
            Entry {
                display: path.to_owned(),
                node: Node::Dir {
                    attributes: ATTR_DIRECTORY,
                    modified: self.time,
                },
            },
        );
        Ok(())
    }

    fn remove_dir(&mut self, path: &str) -> VfsResult<()> {
        if Self::is_root(path) {
            return Err(VfsError::Denied);
        }
        match self.entries.get(&key(path)) {
            Some(Entry {
                node: Node::Dir { .. },
                ..
            }) => {
                if !self.children(path).is_empty() {
                    return Err(VfsError::NotEmpty);
                }
                self.entries.remove(&key(path));
                Ok(())
            }
            Some(_) => Err(VfsError::NotADirectory),
            None => Err(VfsError::PathNotFound),
        }
    }

    fn list(&mut self, dir: &str) -> VfsResult<Vec<DirEntry>> {
        if !self.dir_exists(dir) {
            return Err(VfsError::PathNotFound);
        }
        let mut v: Vec<DirEntry> = self
            .children(dir)
            .into_iter()
            .map(|(e, name)| DirEntry {
                name,
                stat: Self::stat_of(&e.node),
            })
            .collect();
        v.sort_by_key(|e| e.name.to_lowercase());
        Ok(v)
    }

    fn set_attributes(&mut self, path: &str, attrs: u8) -> VfsResult<()> {
        match self.entries.get_mut(&key(path)) {
            Some(Entry {
                node: Node::File { attributes, .. },
                ..
            }) => {
                *attributes = attrs & (ATTR_READONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_ARCHIVE);
                Ok(())
            }
            Some(Entry {
                node: Node::Dir { attributes, .. },
                ..
            }) => {
                *attributes = ATTR_DIRECTORY
                    | (attrs & (ATTR_READONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_ARCHIVE));
                Ok(())
            }
            None => Err(VfsError::NotFound),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_directories_and_case_insensitivity() {
        let mut fs = MemFs::new();
        assert_eq!(fs.write("/a.txt", b"x"), Ok(()));
        assert_eq!(fs.read("/A.TXT"), Ok(b"x".to_vec()));
        assert_eq!(fs.write("/nodir/a.txt", b"x"), Err(VfsError::PathNotFound));
        assert_eq!(fs.create_dir("/d"), Ok(()));
        assert_eq!(fs.create_dir("/D"), Err(VfsError::AlreadyExists));
        assert_eq!(fs.write("/d/b.txt", b"yy"), Ok(()));
        assert_eq!(fs.remove_dir("/d"), Err(VfsError::NotEmpty));
        let names: Vec<_> = fs.list("/").unwrap().into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["a.txt", "d"]);
        assert_eq!(fs.rename("/d", "/e"), Ok(()));
        assert_eq!(fs.read("/e/b.txt"), Ok(b"yy".to_vec()));
        assert_eq!(fs.read("/d/b.txt"), Err(VfsError::NotFound));
        assert_eq!(fs.remove_file("/e/b.txt"), Ok(()));
        assert_eq!(fs.remove_dir("/e"), Ok(()));
        assert_eq!(fs.stat("/e"), Ok(None));
    }

    #[test]
    fn read_only_files_are_protected() {
        let mut fs = MemFs::new().with_file("/r.txt", b"a");
        fs.set_attributes("/r.txt", ATTR_READONLY).unwrap();
        assert_eq!(fs.write("/r.txt", b"b"), Err(VfsError::Denied));
        assert_eq!(fs.remove_file("/r.txt"), Err(VfsError::Denied));
        assert_eq!(
            fs.stat("/r.txt").unwrap().unwrap().attributes,
            ATTR_READONLY
        );
    }

    #[test]
    fn drives_and_setup_helpers() {
        let mut fs = MemFs::new().with_file("c:/data/in.txt", b"1");
        assert_eq!(fs.read("c:/DATA/IN.TXT"), Ok(b"1".to_vec()));
        assert_eq!(fs.files(), ["c:/data/in.txt"]);
        assert!(fs.stat("c:/").unwrap().unwrap().is_dir);
    }
}
