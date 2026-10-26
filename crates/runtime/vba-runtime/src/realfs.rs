//! An opt-in [`Vfs`] over the real file system.
//!
//! Using a `RealFs` is the embedder's decision; the VBA program still needs
//! `FileRead` / `FileWrite` grants on the policy of the host, checked
//! against [`Vfs::canonical`] — the path with symbolic links resolved — so a
//! link inside a granted directory cannot reach outside it.

use std::fs;
#[cfg(not(unix))]
use std::io::Write;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
#[cfg(not(unix))]
use std::time::{SystemTime, UNIX_EPOCH};

use crate::vfs::*;

#[cfg(unix)]
use crate::realfs_unix as sys;

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
    // The verbatim forms Windows returns: `\\?\UNC\server\share` is the
    // network path `//server/share`, `\\?\C:\x` is `c:/x`.
    let s = match s.strip_prefix("//?/UNC/") {
        Some(rest) => format!("//{rest}"),
        None => s.strip_prefix("//?/").unwrap_or(&s).to_owned(),
    };
    match s.as_bytes() {
        [d, b':', ..] if d.is_ascii_alphabetic() => {
            format!("{}{}", (*d as char).to_ascii_lowercase(), &s[1..])
        }
        _ => s,
    }
}

pub(crate) fn map_err(e: &std::io::Error) -> VfsError {
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
#[cfg(not(unix))]
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

#[cfg(not(unix))]
fn ole_date(t: SystemTime) -> f64 {
    let secs = match t.duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs_f64(),
        Err(e) => -e.duration().as_secs_f64(),
    };
    25569.0 + secs / 86_400.0
}

#[cfg(not(unix))]
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

impl RealFs {
    /// Fails unless `path` is already in the form [`Vfs::canonical`] returns.
    /// The policy was checked against that form; a path that resolves
    /// differently now (a link swapped in between the check and the use)
    /// would be used for something the policy never saw.
    fn exact(&mut self, path: &str) -> VfsResult<()> {
        if self.canonical(path) == path {
            Ok(())
        } else {
            Err(VfsError::Denied)
        }
    }

    /// [`RealFs::exact`] for the directory that holds `path`, for the
    /// operations that act on a final link itself and do not follow it.
    fn exact_parent(&mut self, path: &str) -> VfsResult<()> {
        let (parent, _) = split_parent(path);
        self.exact(&parent)
    }
}

#[cfg(test)]
type Hook = Box<dyn FnMut(&str)>;

#[cfg(test)]
thread_local! {
    static HOOK: std::cell::RefCell<Option<Hook>> = const { std::cell::RefCell::new(None) };
}

/// A point between the check of a path and its use, where a test can play
/// the part of another process: `"checked"` after the path was re-resolved,
/// `"opened"` after the directories were opened and before the system call.
#[cfg(test)]
pub(crate) fn checkpoint(stage: &str) {
    let hook = HOOK.with(|h| h.borrow_mut().take());
    if let Some(mut f) = hook {
        f(stage);
        HOOK.with(|h| *h.borrow_mut() = Some(f));
    }
}

#[cfg(not(test))]
#[inline]
pub(crate) fn checkpoint(_stage: &str) {}

/// Whether the open `file` is the file now found at `path`. Without
/// directory handles (not Unix) this cannot be checked and holds.
#[cfg(not(unix))]
fn is_same_file(_file: &fs::File, _path: &str) -> bool {
    true
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
        self.exact(path)?;
        checkpoint("checked");
        #[cfg(unix)]
        {
            sys::stat(path)
        }
        #[cfg(not(unix))]
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
        self.exact(path)?;
        checkpoint("checked");
        #[cfg(unix)]
        let file = sys::open_read(path)?;
        #[cfg(not(unix))]
        let file = {
            if fs::metadata(path).is_ok_and(|m| m.is_dir()) {
                return Err(VfsError::IsADirectory);
            }
            ensure_not_special(path)?;
            fs::File::open(path).map_err(|e| map_err(&e))?
        };
        // Checked on the open handle too, so a swap after the test above
        // cannot turn the read into a device read, and the handle must be
        // the file that the approved path names now.
        #[cfg(not(unix))]
        {
            if !file.metadata().map_err(|e| map_err(&e))?.is_file() {
                return Err(VfsError::Denied);
            }
            self.exact(path)?;
            if !is_same_file(&file, path) {
                return Err(VfsError::Denied);
            }
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
        self.exact(path)?;
        checkpoint("checked");
        #[cfg(unix)]
        {
            sys::write(path, data)
        }
        #[cfg(not(unix))]
        {
            let (parent, _) = split_parent(path);
            if !fs::metadata(&parent).is_ok_and(|m| m.is_dir()) {
                return Err(VfsError::PathNotFound);
            }
            if fs::metadata(path).is_ok_and(|m| m.is_dir()) {
                return Err(VfsError::IsADirectory);
            }
            ensure_not_special(path)?;
            // Opened without truncating: nothing is destroyed until the
            // handle is known to be the approved file. A file may still be
            // created at a swapped place here; see the security model.
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(path)
                .map_err(|e| map_err(&e))?;
            self.exact(path)?;
            if !file.metadata().map_err(|e| map_err(&e))?.is_file() || !is_same_file(&file, path) {
                return Err(VfsError::Denied);
            }
            file.set_len(0).map_err(|e| map_err(&e))?;
            file.write_all(data).map_err(|e| map_err(&e))
        }
    }

    fn remove_file(&mut self, path: &str) -> VfsResult<()> {
        self.exact_parent(path)?;
        checkpoint("checked");
        #[cfg(unix)]
        {
            sys::remove_file(path)
        }
        #[cfg(not(unix))]
        {
            if fs::metadata(path).is_ok_and(|m| m.is_dir()) {
                return Err(VfsError::IsADirectory);
            }
            fs::remove_file(path).map_err(|e| map_err(&e))
        }
    }

    fn rename(&mut self, from: &str, to: &str) -> VfsResult<()> {
        self.exact_parent(from)?;
        self.exact_parent(to)?;
        checkpoint("checked");
        #[cfg(unix)]
        {
            sys::rename(from, to)
        }
        #[cfg(not(unix))]
        {
            if fs::symlink_metadata(to).is_ok() {
                return Err(VfsError::AlreadyExists);
            }
            fs::rename(from, to).map_err(|e| map_err(&e))
        }
    }

    fn copy(&mut self, from: &str, to: &str) -> VfsResult<()> {
        self.exact(from)?;
        self.exact(to)?;
        checkpoint("checked");
        #[cfg(unix)]
        {
            sys::copy(from, to)
        }
        #[cfg(not(unix))]
        {
            if fs::metadata(from).is_ok_and(|m| m.is_dir()) {
                return Err(VfsError::IsADirectory);
            }
            ensure_not_special(from)?;
            ensure_not_special(to)?;
            fs::copy(from, to).map(|_| ()).map_err(|e| map_err(&e))
        }
    }

    fn create_dir(&mut self, path: &str) -> VfsResult<()> {
        self.exact_parent(path)?;
        checkpoint("checked");
        #[cfg(unix)]
        {
            sys::create_dir(path)
        }
        #[cfg(not(unix))]
        {
            let (parent, _) = split_parent(path);
            if fs::symlink_metadata(path).is_ok() {
                return Err(VfsError::AlreadyExists);
            }
            if !fs::metadata(&parent).is_ok_and(|m| m.is_dir()) {
                return Err(VfsError::PathNotFound);
            }
            fs::create_dir(path).map_err(|e| map_err(&e))
        }
    }

    fn remove_dir(&mut self, path: &str) -> VfsResult<()> {
        self.exact_parent(path)?;
        checkpoint("checked");
        #[cfg(unix)]
        {
            sys::remove_dir(path)
        }
        #[cfg(not(unix))]
        {
            match fs::metadata(path) {
                Ok(m) if m.is_dir() => fs::remove_dir(path).map_err(|e| map_err(&e)),
                Ok(_) => Err(VfsError::NotADirectory),
                Err(e) => Err(map_err(&e)),
            }
        }
    }

    fn list(&mut self, dir: &str) -> VfsResult<Vec<DirEntry>> {
        self.exact(dir)?;
        checkpoint("checked");
        #[cfg(unix)]
        let mut v = sys::list(dir)?;
        #[cfg(not(unix))]
        let mut v = Vec::new();
        #[cfg(not(unix))]
        let rd = fs::read_dir(dir).map_err(|e| match e.kind() {
            ErrorKind::NotFound => VfsError::PathNotFound,
            _ => map_err(&e),
        })?;
        #[cfg(not(unix))]
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
        self.exact(path)?;
        checkpoint("checked");
        #[cfg(unix)]
        {
            sys::set_readonly(path, attributes & ATTR_READONLY != 0)
        }
        #[cfg(not(unix))]
        {
            let m = fs::metadata(path).map_err(|e| map_err(&e))?;
            let mut perm = m.permissions();
            perm.set_readonly(attributes & ATTR_READONLY != 0);
            fs::set_permissions(path, perm).map_err(|e| map_err(&e))
        }
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

    /// The state after another process swapped a link into a path that the
    /// policy had already approved: the file system is handed a path that is
    /// no longer canonical, and must not follow it.
    #[cfg(unix)]
    #[test]
    fn a_link_swapped_in_after_the_check_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let base = to_canonical_text(&fs::canonicalize(dir.path()).unwrap());
        fs::create_dir(format!("{base}/allowed")).unwrap();
        fs::create_dir(format!("{base}/outside")).unwrap();
        fs::write(format!("{base}/outside/secret.txt"), "outside").unwrap();
        std::os::unix::fs::symlink(
            format!("{base}/outside/secret.txt"),
            format!("{base}/allowed/file"),
        )
        .unwrap();
        std::os::unix::fs::symlink(format!("{base}/outside"), format!("{base}/allowed/dir"))
            .unwrap();
        let mut f = RealFs::new();
        let via_file = format!("{base}/allowed/file");
        let via_dir = format!("{base}/allowed/dir/secret.txt");
        assert_eq!(f.read(&via_file), Err(VfsError::Denied));
        assert_eq!(f.read_limited(&via_dir, 100), Err(VfsError::Denied));
        assert_eq!(f.write(&via_file, b"x"), Err(VfsError::Denied));
        assert_eq!(f.write(&via_dir, b"x"), Err(VfsError::Denied));
        assert_eq!(
            f.copy(&via_dir, &format!("{base}/allowed/c")),
            Err(VfsError::Denied)
        );
        assert_eq!(
            f.copy(&format!("{base}/allowed/x"), &via_dir),
            Err(VfsError::Denied)
        );
        assert_eq!(f.remove_file(&via_dir), Err(VfsError::Denied));
        assert_eq!(
            f.rename(&via_dir, &format!("{base}/allowed/r")),
            Err(VfsError::Denied)
        );
        assert_eq!(f.stat(&via_dir), Err(VfsError::Denied));
        assert_eq!(
            f.list(&format!("{base}/allowed/dir")),
            Err(VfsError::Denied)
        );
        assert_eq!(
            f.create_dir(&format!("{base}/allowed/dir/sub")),
            Err(VfsError::Denied)
        );
        assert_eq!(
            fs::read_to_string(format!("{base}/outside/secret.txt")).unwrap(),
            "outside"
        );
        assert!(!Path::new(&format!("{base}/allowed/c")).exists());
        assert!(!Path::new(&format!("{base}/allowed/r")).exists());
        assert!(!Path::new(&format!("{base}/outside/sub")).exists());
    }

    /// A scratch tree: `allowed/dir/` (real directory with `file`, `sub/`)
    /// and `outside/` with a `secret` file and a `keep/` directory.
    #[cfg(unix)]
    struct Tree {
        _guard: tempfile::TempDir,
        base: String,
    }

    #[cfg(unix)]
    impl Tree {
        fn new() -> Tree {
            let dir = tempfile::tempdir().unwrap();
            let base = to_canonical_text(&fs::canonicalize(dir.path()).unwrap());
            fs::create_dir_all(format!("{base}/allowed/dir/sub")).unwrap();
            fs::write(format!("{base}/allowed/dir/file"), "inside").unwrap();
            fs::create_dir_all(format!("{base}/outside/keep")).unwrap();
            fs::write(format!("{base}/outside/file"), "outside").unwrap();
            fs::write(format!("{base}/outside/other"), "other").unwrap();
            fs::create_dir(format!("{base}/outside/sub")).unwrap();
            fs::write(format!("{base}/allowed/loose"), "loose").unwrap();
            Tree { _guard: dir, base }
        }

        fn p(&self, rel: &str) -> String {
            format!("{}/{rel}", self.base)
        }

        /// Nothing under `outside` was created, changed or removed.
        fn assert_outside_untouched(&self, what: &str) {
            let mut names: Vec<_> = fs::read_dir(self.p("outside"))
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            assert_eq!(names, ["file", "keep", "other", "sub"], "{what}");
            let read = |n: &str| fs::read_to_string(self.p(n)).unwrap();
            assert_eq!(read("outside/file"), "outside", "{what}");
            assert_eq!(read("outside/other"), "other", "{what}");
            assert!(Path::new(&self.p("outside/sub")).is_dir(), "{what}");
        }
    }

    /// Moves `allowed/dir` away and puts a link to `outside` in its place.
    #[cfg(unix)]
    fn swap_dir_for_link(base: &str) {
        fs::rename(
            format!("{base}/allowed/dir"),
            format!("{base}/allowed/moved"),
        )
        .unwrap();
        std::os::unix::fs::symlink(format!("{base}/outside"), format!("{base}/allowed/dir"))
            .unwrap();
    }

    #[cfg(unix)]
    fn on_stage(stage: &'static str, f: impl Fn() + 'static) {
        HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(move |s: &str| {
                if s == stage {
                    f();
                }
            }));
        });
    }

    #[cfg(unix)]
    fn clear_hook() {
        HOOK.with(|h| *h.borrow_mut() = None);
    }

    /// A parent directory swapped for a link between the re-resolution and
    /// the use must not redirect any operation.
    #[cfg(unix)]
    #[test]
    fn a_parent_swapped_after_the_check_does_not_redirect_an_operation() {
        type Op = Box<dyn Fn(&mut RealFs, &Tree) -> VfsResult<()>>;
        let ops: Vec<(&str, Op)> = vec![
            (
                "kill",
                Box::new(|f, t| f.remove_file(&t.p("allowed/dir/file"))),
            ),
            (
                "name from",
                Box::new(|f, t| f.rename(&t.p("allowed/dir/file"), &t.p("allowed/renamed"))),
            ),
            (
                "name to",
                Box::new(|f, t| f.rename(&t.p("allowed/loose"), &t.p("allowed/dir/file2"))),
            ),
            (
                "mkdir",
                Box::new(|f, t| f.create_dir(&t.p("allowed/dir/new"))),
            ),
            (
                "rmdir",
                Box::new(|f, t| f.remove_dir(&t.p("allowed/dir/sub"))),
            ),
            (
                "copy from",
                Box::new(|f, t| f.copy(&t.p("allowed/dir/file"), &t.p("allowed/copy"))),
            ),
            (
                "copy to",
                Box::new(|f, t| f.copy(&t.p("allowed/loose"), &t.p("allowed/dir/copy"))),
            ),
            (
                "write",
                Box::new(|f, t| f.write(&t.p("allowed/dir/created"), b"x")),
            ),
            (
                "write over",
                Box::new(|f, t| f.write(&t.p("allowed/dir/file"), b"x")),
            ),
            (
                "read",
                Box::new(|f, t| f.read(&t.p("allowed/dir/file")).map(|_| ())),
            ),
            (
                "setattr",
                Box::new(|f, t| f.set_attributes(&t.p("allowed/dir/file"), 1)),
            ),
        ];
        for stage in ["checked", "opened"] {
            for (name, op) in &ops {
                let t = Tree::new();
                let base = t.base.clone();
                on_stage(stage, move || swap_dir_for_link(&base));
                let mut f = RealFs::new();
                let _ = op(&mut f, &t);
                clear_hook();
                t.assert_outside_untouched(&format!("{name} at {stage}"));
            }
        }
    }

    /// Metadata and listings go through the verified directory too: a parent
    /// swapped for a link never shows the program what is behind the link.
    #[cfg(unix)]
    #[test]
    fn metadata_and_listings_do_not_follow_a_swapped_parent() {
        for stage in ["checked", "opened"] {
            let t = Tree::new();
            let base = t.base.clone();
            on_stage(stage, move || swap_dir_for_link(&base));
            let mut f = RealFs::new();
            let st = f.stat(&t.p("allowed/dir/file"));
            clear_hook();
            match st {
                // Refused, or reported missing (a link is not a directory
                // to the system, and the platforms differ in the error).
                Err(VfsError::Denied) | Ok(None) => {}
                // Already behind the handle: the file that was verified.
                Ok(Some(s)) => assert_eq!((stage, s.len), ("opened", 6)),
                other => panic!("{stage}: {other:?}"),
            }
            let t = Tree::new();
            let base = t.base.clone();
            on_stage(stage, move || swap_dir_for_link(&base));
            let names = f.list(&t.p("allowed/dir"));
            clear_hook();
            match names {
                Err(VfsError::Denied | VfsError::PathNotFound) => {}
                Ok(v) => {
                    let n: Vec<_> = v.iter().map(|e| e.name.as_str()).collect();
                    assert_eq!((stage, n), ("opened", vec!["file", "sub"]));
                }
                other => panic!("{stage}: {other:?}"),
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn stat_of_the_root_and_of_a_missing_parent() {
        let mut f = RealFs::new();
        assert!(f.stat("/").unwrap().unwrap().is_dir);
        let t = Tree::new();
        assert_eq!(f.stat(&t.p("nowhere/x")), Ok(None));
        assert_eq!(f.stat(&t.p("allowed/nothing")), Ok(None));
        let s = f.stat(&t.p("allowed/loose")).unwrap().unwrap();
        assert_eq!((s.is_dir, s.len), (false, 5));
        let entries = f.list(&t.p("allowed")).unwrap();
        assert_eq!(
            entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
            ["dir", "loose"]
        );
        assert!(entries[0].stat.is_dir);
    }

    /// The same swap, played for a final component: a link to a name that
    /// does not exist yet must not make a write create it.
    #[cfg(unix)]
    #[test]
    fn a_write_through_a_swapped_name_creates_nothing_outside() {
        for stage in ["checked", "opened"] {
            let t = Tree::new();
            let (link, target) = (t.p("allowed/new"), t.p("outside/created"));
            on_stage(stage, move || {
                let _ = std::os::unix::fs::symlink(&target, &link);
            });
            let mut f = RealFs::new();
            let r = f.write(&t.p("allowed/new"), b"x");
            clear_hook();
            assert!(r.is_err(), "{stage}");
            t.assert_outside_untouched(stage);
        }
    }

    /// A swap between the opening of the directory and the system call
    /// lands on the directory that was verified.
    #[cfg(unix)]
    #[test]
    fn an_operation_acts_on_the_directory_that_was_verified() {
        let t = Tree::new();
        let base = t.base.clone();
        on_stage("opened", move || swap_dir_for_link(&base));
        let mut f = RealFs::new();
        assert_eq!(f.remove_file(&t.p("allowed/dir/file")), Ok(()));
        clear_hook();
        assert!(!Path::new(&t.p("allowed/moved/file")).exists());
        t.assert_outside_untouched("opened");
    }

    /// A swap of the parent while a write is pending leaves no empty file
    /// at the new place either.
    #[cfg(unix)]
    #[test]
    fn a_swapped_parent_leaves_no_stray_empty_file() {
        let t = Tree::new();
        let base = t.base.clone();
        on_stage("checked", move || swap_dir_for_link(&base));
        let mut f = RealFs::new();
        let r = f.write(&t.p("allowed/dir/stray"), b"data");
        clear_hook();
        assert!(r.is_err());
        t.assert_outside_untouched("stray");
        assert!(!Path::new(&t.p("allowed/moved/stray")).exists());
    }

    #[test]
    fn windows_verbatim_paths_become_the_canonical_spelling() {
        let t = |s: &str| to_canonical_text(Path::new(s));
        assert_eq!(t(r"\\?\C:\Data\Me"), "c:/Data/Me");
        assert_eq!(t(r"\\?\UNC\srv\share\x"), "//srv/share/x");
        assert_eq!(t(r"D:\a"), "d:/a");
        assert_eq!(t("/usr/lib"), "/usr/lib");
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
