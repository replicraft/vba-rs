//! Directory-relative file calls for [`crate::RealFs`] on Unix.
//!
//! The path handed in is already in the form `Vfs::canonical` returns, so it
//! holds no link. It is still walked one directory at a time from `/`, each
//! step an `openat` that refuses a link (`O_NOFOLLOW`), and the operation is
//! then made relative to the directory handle that came out
//! (`unlinkat`, `renameat`, `mkdirat`, `openat`). A directory swapped for a
//! link after the policy was applied is therefore either refused or already
//! behind the handle, and the call acts on the directory that was verified.
//! A file is opened without following a final link and without creating
//! anything through one, so a write cannot leave an empty file at a place
//! the policy never saw.
//!
//! Metadata (`stat`) and directory listings go through the same handles:
//! the parent is opened without following links and the entry is read with
//! `fstatat` relative to it, so a parent swapped for a link after the policy
//! was applied is refused instead of showing the program a directory it was
//! never granted.

use std::fs::File;
use std::io::{self, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;

use rustix::fs::{AtFlags, CWD, FileType, Mode, OFlags};
use rustix::io::Errno;

use crate::realfs::{checkpoint, map_err};
use crate::vfs::*;

fn io_err(e: Errno) -> VfsError {
    if e == Errno::LOOP {
        VfsError::Denied
    } else {
        map_err(&io::Error::from(e))
    }
}

/// An error while walking to a directory: a missing or non-directory step
/// is a missing path.
fn walk_err(e: Errno) -> VfsError {
    if e == Errno::NOENT || e == Errno::NOTDIR {
        VfsError::PathNotFound
    } else {
        io_err(e)
    }
}

const DIR_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

/// Opens the directory `path`, step by step, following no link.
fn open_dir(path: &str) -> VfsResult<OwnedFd> {
    if !path.starts_with('/') {
        return Err(VfsError::Denied);
    }
    let mut cur = rustix::fs::openat(CWD, "/", DIR_FLAGS, Mode::empty()).map_err(walk_err)?;
    for part in path.split('/').filter(|p| !p.is_empty()) {
        if part == "." || part == ".." {
            return Err(VfsError::Denied);
        }
        cur = rustix::fs::openat(&cur, part, DIR_FLAGS, Mode::empty()).map_err(walk_err)?;
    }
    Ok(cur)
}

/// The verified directory that holds `path`, and the final name.
fn open_parent(path: &str) -> VfsResult<(OwnedFd, String)> {
    let (parent, name) = split_parent(path);
    if name.is_empty() || name == "." || name == ".." {
        return Err(VfsError::Denied);
    }
    Ok((open_dir(&parent)?, name))
}

fn kind_of(dir: &OwnedFd, name: &str) -> VfsResult<Option<FileType>> {
    match rustix::fs::statat(dir, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(st) => Ok(Some(FileType::from_raw_mode(st.st_mode))),
        Err(Errno::NOENT) => Ok(None),
        Err(e) => Err(io_err(e)),
    }
}

/// Opens a regular file for reading without following a link.
fn open_regular(dir: &OwnedFd, name: &str) -> VfsResult<File> {
    match kind_of(dir, name)? {
        None => return Err(VfsError::NotFound),
        Some(FileType::Directory) => return Err(VfsError::IsADirectory),
        Some(FileType::RegularFile) => {}
        Some(_) => return Err(VfsError::Denied),
    }
    // Non-blocking so that a name swapped to a pipe cannot stall the open.
    let fd = rustix::fs::openat(
        dir,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(io_err)?;
    let file = File::from(fd);
    if !file.metadata().map_err(|e| map_err(&e))?.is_file() {
        return Err(VfsError::Denied);
    }
    Ok(file)
}

/// Opens (creating if absent, truncating nothing) a regular file for
/// writing. A final link is refused, never followed, so nothing is created
/// behind it.
fn open_for_write(dir: &OwnedFd, name: &str) -> VfsResult<File> {
    match kind_of(dir, name)? {
        Some(FileType::Directory) => return Err(VfsError::IsADirectory),
        Some(FileType::RegularFile) | None => {}
        Some(_) => return Err(VfsError::Denied),
    }
    let fd = rustix::fs::openat(
        dir,
        name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o666),
    )
    .map_err(io_err)?;
    let file = File::from(fd);
    if !file.metadata().map_err(|e| map_err(&e))?.is_file() {
        return Err(VfsError::Denied);
    }
    Ok(file)
}

/// Opens `path` for reading.
pub(crate) fn open_read(path: &str) -> VfsResult<File> {
    let (dir, name) = open_parent(path)?;
    checkpoint("opened");
    open_regular(&dir, &name)
}

/// Replaces the contents of `path`, creating it.
pub(crate) fn write(path: &str, data: &[u8]) -> VfsResult<()> {
    let (dir, name) = open_parent(path)?;
    checkpoint("opened");
    let mut file = open_for_write(&dir, &name)?;
    file.set_len(0).map_err(|e| map_err(&e))?;
    file.write_all(data).map_err(|e| map_err(&e))
}

pub(crate) fn remove_file(path: &str) -> VfsResult<()> {
    let (dir, name) = open_parent(path)?;
    checkpoint("opened");
    match kind_of(&dir, &name)? {
        None => Err(VfsError::NotFound),
        Some(FileType::Directory) => Err(VfsError::IsADirectory),
        Some(_) => rustix::fs::unlinkat(&dir, name.as_str(), AtFlags::empty()).map_err(io_err),
    }
}

pub(crate) fn remove_dir(path: &str) -> VfsResult<()> {
    let (dir, name) = open_parent(path)?;
    checkpoint("opened");
    match kind_of(&dir, &name)? {
        None => Err(VfsError::NotFound),
        Some(FileType::Directory) => {
            rustix::fs::unlinkat(&dir, name.as_str(), AtFlags::REMOVEDIR).map_err(io_err)
        }
        Some(_) => Err(VfsError::NotADirectory),
    }
}

pub(crate) fn create_dir(path: &str) -> VfsResult<()> {
    let (dir, name) = open_parent(path)?;
    checkpoint("opened");
    if kind_of(&dir, &name)?.is_some() {
        return Err(VfsError::AlreadyExists);
    }
    rustix::fs::mkdirat(&dir, name.as_str(), Mode::from_raw_mode(0o777)).map_err(io_err)
}

pub(crate) fn rename(from: &str, to: &str) -> VfsResult<()> {
    let (from_dir, from_name) = open_parent(from)?;
    let (to_dir, to_name) = open_parent(to)?;
    checkpoint("opened");
    if kind_of(&to_dir, &to_name)?.is_some() {
        return Err(VfsError::AlreadyExists);
    }
    // Where the system can say "do not replace", the check above is also
    // atomic with the move.
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
    match rustix::fs::renameat_with(
        &from_dir,
        from_name.as_str(),
        &to_dir,
        to_name.as_str(),
        rustix::fs::RenameFlags::NOREPLACE,
    ) {
        Err(Errno::INVAL | Errno::NOSYS | Errno::OPNOTSUPP) => {}
        Err(Errno::EXIST) => return Err(VfsError::AlreadyExists),
        other => return other.map_err(io_err),
    }
    rustix::fs::renameat(&from_dir, from_name.as_str(), &to_dir, to_name.as_str()).map_err(io_err)
}

pub(crate) fn copy(from: &str, to: &str) -> VfsResult<()> {
    let (from_dir, from_name) = open_parent(from)?;
    let (to_dir, to_name) = open_parent(to)?;
    checkpoint("opened");
    let mut src = open_regular(&from_dir, &from_name)?;
    let mut dst = open_for_write(&to_dir, &to_name)?;
    let (a, b) = (
        src.metadata().map_err(|e| map_err(&e))?,
        dst.metadata().map_err(|e| map_err(&e))?,
    );
    if a.dev() == b.dev() && a.ino() == b.ino() {
        return Err(VfsError::Other(
            "source and destination are the same file".into(),
        ));
    }
    dst.set_len(0).map_err(|e| map_err(&e))?;
    io::copy(&mut src, &mut dst).map_err(|e| map_err(&e))?;
    dst.set_permissions(a.permissions())
        .map_err(|e| map_err(&e))
}

/// Sets the read-only bit of `path`.
pub(crate) fn set_readonly(path: &str, readonly: bool) -> VfsResult<()> {
    let (dir, name) = open_parent(path)?;
    checkpoint("opened");
    let file = match kind_of(&dir, &name)? {
        None => return Err(VfsError::NotFound),
        Some(FileType::Directory) => {
            let fd = rustix::fs::openat(&dir, name.as_str(), DIR_FLAGS, Mode::empty())
                .map_err(io_err)?;
            File::from(fd)
        }
        _ => open_regular(&dir, &name)?,
    };
    let mut perm = file.metadata().map_err(|e| map_err(&e))?.permissions();
    perm.set_readonly(readonly);
    file.set_permissions(perm).map_err(|e| map_err(&e))
}

fn stat_from_raw(st: &rustix::fs::Stat) -> Stat {
    let is_dir = FileType::from_raw_mode(st.st_mode) == FileType::Directory;
    let mut attributes = if is_dir { ATTR_DIRECTORY } else { ATTR_ARCHIVE };
    // The same rule as `std::fs::Permissions::readonly`: no write bit at all.
    if st.st_mode & 0o222 == 0 {
        attributes |= ATTR_READONLY;
    }
    #[allow(clippy::unnecessary_cast)]
    let secs = st.st_mtime as f64 + (st.st_mtime_nsec as f64) / 1e9;
    Stat {
        is_dir,
        len: if is_dir { 0 } else { st.st_size.max(0) as u64 },
        modified: 25569.0 + secs / 86_400.0,
        attributes,
    }
}

/// Metadata of `path`; `None` when it does not exist. A final link is
/// refused (the path given is already canonical, so one means a swap).
pub(crate) fn stat(path: &str) -> VfsResult<Option<Stat>> {
    let st = if split_parent(path).1.is_empty() {
        // The root has no parent: stat the verified handle itself.
        let fd = open_dir(path)?;
        rustix::fs::fstat(&fd).map_err(io_err)?
    } else {
        let (dir, name) = match open_parent(path) {
            Ok(v) => v,
            // A missing parent is a missing path, as `fs::metadata` said.
            Err(VfsError::PathNotFound) => return Ok(None),
            Err(e) => return Err(e),
        };
        checkpoint("opened");
        match rustix::fs::statat(&dir, name.as_str(), AtFlags::SYMLINK_NOFOLLOW) {
            Ok(st) => st,
            Err(Errno::NOENT | Errno::NOTDIR) => return Ok(None),
            Err(e) => return Err(io_err(e)),
        }
    };
    if FileType::from_raw_mode(st.st_mode) == FileType::Symlink {
        return Err(VfsError::Denied);
    }
    Ok(Some(stat_from_raw(&st)))
}

/// The entries of the directory `path`, with the metadata of what each
/// names (a link inside the directory shows its target, relative to the
/// verified handle). Unreadable entries are skipped.
pub(crate) fn list(path: &str) -> VfsResult<Vec<DirEntry>> {
    let fd = open_dir(path)?;
    checkpoint("opened");
    let dir_fd = rustix::io::fcntl_dupfd_cloexec(&fd, 0).map_err(io_err)?;
    let mut rd = rustix::fs::Dir::new(dir_fd).map_err(io_err)?;
    let mut v = Vec::new();
    while let Some(entry) = rd.read() {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "." || name == ".." {
            continue;
        }
        if let Ok(st) = rustix::fs::statat(&fd, entry.file_name(), AtFlags::empty()) {
            v.push(DirEntry {
                name,
                stat: stat_from_raw(&st),
            });
        }
    }
    Ok(v)
}
