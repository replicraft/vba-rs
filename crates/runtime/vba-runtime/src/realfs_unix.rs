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
