//! The VBA file statements and file system functions over a [`Vfs`].
//!
//! Every operation that touches a path first normalizes it against the
//! virtual working directory, resolves it with [`Vfs::canonical`] and asks
//! the host policy: reading needs `FileRead`, anything that creates,
//! changes or deletes needs `FileWrite`. There is no other way to reach the
//! file system.
//!
//! Open files are held in memory (loaded at `Open`, written back at `Close`,
//! `End`, `Reset` and when the [`FileSystem`] is dropped). Sequential text is
//! Windows-1252 with CR LF line ends. Rules marked **doc** follow the
//! documentation of the VBA statements and are not verified against Office.

use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use vba_ast::{OpenAccess, OpenLock, OpenMode};
use vba_automation::ErrInfo;
use vba_ffi::{Codepage, Unmappable};
use vba_host::{Capability, Host};
use vba_value::{Date, VarType, Variant, VbaString, convert};
use vba_vm::{
    IoReply, IoRequest, LimitKind, PrintItem, RecordValue, ServiceError, Services, Shape,
    format_print_width,
};

use crate::error::FsError;
use crate::records::{self, Reader};
use crate::shell::ShellHook;
use crate::vfs::*;

type R<T> = Result<T, FsError>;

/// Largest in-memory size of one open file (error 61 beyond it).
pub const DEFAULT_MAX_FILE_BYTES: usize = 64 << 20;

/// The bytes of one file, shared by every file number that has it open.
#[derive(Debug)]
struct Shared {
    data: Vec<u8>,
    dirty: bool,
    /// File numbers that refer to this buffer.
    refs: usize,
}

/// A range locked by `Lock`: the units written in the statement (records for
/// a Random file, bytes for Binary) and the bytes they cover, half open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LockRange {
    start: i64,
    end: i64,
    bytes: (u64, u64),
}

#[derive(Debug)]
struct OpenFile {
    path: String,
    /// Key of the shared buffer: the path in lower case.
    key: String,
    mode: OpenMode,
    readable: bool,
    writable: bool,
    lock: OpenLock,
    /// `Lock` without a range, or on a sequential file.
    whole_locked: bool,
    locks: Vec<LockRange>,
    pos: usize,
    record_len: usize,
    last_record: i64,
    width: usize,
    col: usize,
}

/// The state behind the file statements of one session.
pub struct FileSystem {
    host: Rc<dyn Host>,
    vfs: Box<dyn Vfs>,
    cwd: String,
    files: BTreeMap<i32, OpenFile>,
    buffers: BTreeMap<String, Shared>,
    dir_iter: Option<VecDeque<String>>,
    command: String,
    shell: Box<dyn ShellHook>,
    max_file_bytes: usize,
}

fn encode_text(s: &VbaString) -> Vec<u8> {
    Codepage::Windows1252
        .encode(s.as_units(), Unmappable::Replace(b'?'), false)
        .map(|e| e.bytes)
        .unwrap_or_default()
}

fn decode_text(b: &[u8]) -> VbaString {
    VbaString::from_units(Codepage::Windows1252.decode(b, false).unwrap_or_default())
}

fn map_vfs(e: VfsError) -> FsError {
    match e {
        VfsError::NotFound => FsError::Vba(53),
        VfsError::PathNotFound => FsError::Vba(76),
        VfsError::AlreadyExists => FsError::Vba(58),
        VfsError::IsADirectory | VfsError::NotADirectory | VfsError::NotEmpty => FsError::Vba(75),
        VfsError::Denied => FsError::Vba(70),
        VfsError::TooLarge => FsError::Vba(61),
        VfsError::Unsupported(s) => FsError::Unsupported(s),
        VfsError::Other(_) => FsError::Vba(57),
    }
}

/// The policy resource for an operation on a directory itself: with a
/// trailing slash, so that a grant `/data/` covers `ChDir "/data"`.
fn dir_resource(p: &str) -> String {
    if p.ends_with('/') {
        p.to_owned()
    } else {
        format!("{p}/")
    }
}

/// Whether a `Lock` clause keeps another channel with this access out.
fn lock_denies(lock: OpenLock, reads: bool, writes: bool) -> bool {
    match lock {
        OpenLock::Shared => false,
        OpenLock::LockRead => reads,
        OpenLock::LockWrite => writes,
        OpenLock::LockReadWrite => reads || writes,
    }
}

fn vba<T>(n: i32) -> R<T> {
    Err(FsError::Vba(n))
}

fn conv<T>(r: Result<T, vba_value::VbaError>) -> R<T> {
    r.map_err(|e| FsError::Vba(e.number()))
}

impl FileSystem {
    /// A file system over `vfs`, checked against the policy of `host`.
    pub fn new(host: Rc<dyn Host>, mut vfs: Box<dyn Vfs>, shell: Box<dyn ShellHook>) -> FileSystem {
        vfs.set_memory_meter(host.budget().meter());
        let cwd = vfs.initial_dir();
        FileSystem {
            host,
            vfs,
            cwd,
            files: BTreeMap::new(),
            buffers: BTreeMap::new(),
            dir_iter: None,
            command: String::new(),
            shell,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }

    /// Sets the string `Command` returns.
    pub fn set_command(&mut self, command: &str) {
        self.command = command.to_owned();
    }

    /// Sets the largest size of an open file.
    pub fn set_max_file_bytes(&mut self, n: usize) {
        self.max_file_bytes = n;
    }

    /// The underlying file system.
    pub fn vfs(&mut self) -> &mut dyn Vfs {
        &mut *self.vfs
    }

    /// The current virtual directory.
    pub fn current_dir(&self) -> &str {
        &self.cwd
    }

    /// File numbers currently open.
    pub fn open_files(&self) -> Vec<i32> {
        self.files.keys().copied().collect()
    }

    fn allow(&self, cap: Capability, path: &str) -> R<()> {
        self.host.check(cap, path).map_err(|info| {
            if self.cwd_is_visible() {
                FsError::Info(info)
            } else {
                // The message names the resolved path, which would give away
                // a working directory the program has no grant for.
                FsError::Info(
                    ErrInfo::standard(70).with_source(format!("vba-host: {} denied", cap.name())),
                )
            }
        })
    }

    fn cwd_is_root(&self) -> bool {
        self.cwd == "/" || self.cwd.len() == 3 && self.cwd.ends_with(":/")
    }

    /// Whether the program may know the working directory: it may read or
    /// write there. A start directory that the policy does not cover is the
    /// embedding application's, and `CurDir` and denial messages keep it
    /// to themselves.
    fn cwd_is_visible(&self) -> bool {
        self.cwd_is_root() || self.cwd_is_granted()
    }

    fn cwd_is_granted(&self) -> bool {
        let dir = dir_resource(&self.cwd);
        self.host.check(Capability::FileRead, &dir).is_ok()
            || self.host.check(Capability::FileWrite, &dir).is_ok()
    }

    fn resolve(&mut self, path: &str) -> R<String> {
        if path.is_empty() || path.contains('\0') {
            return vba(53);
        }
        let n = normalize(&self.cwd, path);
        Ok(self.vfs.canonical(&n))
    }

    /// A path resolved for a read that the program asks the host to make
    /// (`GetObject`): the policy is asked here with the resolved path, and
    /// the host asks again with the same path.
    fn resolve_for_read(&mut self, path: &str) -> R<String> {
        let resolved = self.resolve(path)?;
        self.allow(Capability::FileRead, &resolved)?;
        Ok(resolved)
    }

    fn file(&mut self, n: i32) -> R<&mut OpenFile> {
        self.files.get_mut(&n).ok_or(FsError::Vba(52))
    }

    /// A file number with the buffer it shares with other channels.
    fn chan(&mut self, n: i32) -> R<(&mut OpenFile, &mut Shared)> {
        let f = self.files.get_mut(&n).ok_or(FsError::Vba(52))?;
        let sh = self.buffers.get_mut(&f.key).ok_or(FsError::Vba(52))?;
        Ok((f, sh))
    }

    fn charge(&self, bytes: usize) -> R<()> {
        self.host
            .budget()
            .alloc(bytes as u64)
            .map_err(|_| FsError::Limit(LimitKind::Memory))
    }

    // ---- Open and Close -------------------------------------------------------

    fn open(
        &mut self,
        path: &str,
        mode: OpenMode,
        access: Option<OpenAccess>,
        lock: Option<OpenLock>,
        file: i32,
        record_len: Option<i32>,
    ) -> R<()> {
        if !(1..=511).contains(&file) {
            return vba(52);
        }
        if self.files.contains_key(&file) {
            return vba(55);
        }
        let (readable, writable) = match (mode, access) {
            (OpenMode::Input, None | Some(OpenAccess::Read)) => (true, false),
            (OpenMode::Output | OpenMode::Append, None | Some(OpenAccess::Write)) => (false, true),
            (OpenMode::Binary | OpenMode::Random, None | Some(OpenAccess::ReadWrite)) => {
                (true, true)
            }
            (OpenMode::Binary | OpenMode::Random, Some(OpenAccess::Read)) => (true, false),
            (OpenMode::Binary | OpenMode::Random, Some(OpenAccess::Write)) => (false, true),
            _ => return vba(54),
        };
        let p = self.resolve(path)?;
        if readable {
            self.allow(Capability::FileRead, &p)?;
        }
        if writable {
            self.allow(Capability::FileWrite, &p)?;
        }
        let key = p.to_ascii_lowercase();
        let lock = lock.unwrap_or(OpenLock::Shared);
        for f in self.files.values().filter(|f| f.key == key) {
            // A sequential writer needs the file to itself; Input, Binary and
            // Random channels can share it when their Lock clauses agree.
            let sequential_writer = |m: OpenMode| matches!(m, OpenMode::Output | OpenMode::Append);
            if sequential_writer(mode) || sequential_writer(f.mode) {
                return vba(55);
            }
            if lock_denies(f.lock, readable, writable) || lock_denies(lock, f.readable, f.writable)
            {
                return vba(70);
            }
        }
        let record_len = match (mode, record_len) {
            (OpenMode::Binary, _) => 128,
            // MS-VBAL 5.4.5.1: Len is 1 to 32767 (59 is the number of a bad
            // record length; the text names no number).
            (_, Some(n)) if !(1..=32767).contains(&n) => return vba(59),
            (_, Some(n)) => n as usize,
            (_, None) => 128,
        };
        // A record is buffered whole on every Get and Put; one that no file
        // could hold must not be allocated first.
        if mode == OpenMode::Random && record_len > self.max_file_bytes {
            return vba(59);
        }
        let st = self.vfs.stat(&p).map_err(map_vfs)?;
        if st.as_ref().is_some_and(|s| s.is_dir) {
            return vba(75);
        }
        let max = self.max_file_bytes;
        let shared = self.buffers.contains_key(&key);
        let data = match mode {
            OpenMode::Input => {
                if st.is_none() {
                    return vba(53);
                }
                if shared {
                    Vec::new()
                } else {
                    self.vfs.read_limited(&p, max).map_err(map_vfs)?
                }
            }
            OpenMode::Output => {
                self.vfs.write(&p, &[]).map_err(map_vfs)?;
                Vec::new()
            }
            OpenMode::Append => {
                if st.is_some() {
                    self.vfs.read_limited(&p, max).map_err(map_vfs)?
                } else {
                    self.vfs.write(&p, &[]).map_err(map_vfs)?;
                    Vec::new()
                }
            }
            OpenMode::Binary | OpenMode::Random => {
                if shared {
                    Vec::new()
                } else if st.is_some() {
                    self.vfs.read_limited(&p, max).map_err(map_vfs)?
                } else if writable {
                    self.vfs.write(&p, &[]).map_err(map_vfs)?;
                    Vec::new()
                } else {
                    return vba(53);
                }
            }
        };
        if data.len() > self.max_file_bytes {
            return vba(61);
        }
        self.charge(data.len())?;
        let pos = if mode == OpenMode::Append {
            data.len()
        } else {
            0
        };
        match self.buffers.get_mut(&key) {
            Some(sh) => sh.refs += 1,
            None => {
                self.buffers.insert(
                    key.clone(),
                    Shared {
                        data,
                        dirty: false,
                        refs: 1,
                    },
                );
            }
        }
        self.files.insert(
            file,
            OpenFile {
                path: p,
                key,
                mode,
                readable,
                writable,
                lock,
                whole_locked: false,
                locks: Vec::new(),
                pos,
                record_len,
                last_record: 0,
                width: 0,
                col: 0,
            },
        );
        Ok(())
    }

    fn flush(&mut self, n: i32) -> R<()> {
        let Some(f) = self.files.get(&n) else {
            return Ok(());
        };
        let (path, key) = (f.path.clone(), f.key.clone());
        let Some(sh) = self.buffers.get_mut(&key) else {
            return Ok(());
        };
        if sh.dirty && self.files.get(&n).is_some_and(|f| f.writable) {
            let data = std::mem::take(&mut sh.data);
            let r = self.vfs.write(&path, &data).map_err(map_vfs);
            if let Some(sh) = self.buffers.get_mut(&key) {
                sh.data = data;
                if r.is_ok() {
                    sh.dirty = false;
                }
            }
            r?;
        }
        Ok(())
    }

    fn close_one(&mut self, n: i32) -> R<()> {
        let r = self.flush(n);
        if let Some(f) = self.files.remove(&n)
            && let Some(sh) = self.buffers.get_mut(&f.key)
        {
            sh.refs -= 1;
            if sh.refs == 0 {
                let len = sh.data.len();
                self.buffers.remove(&f.key);
                self.host.budget().free(len as u64);
            }
        }
        r
    }

    /// Closes every file, writing back what changed; the first failure is
    /// reported after all files are closed.
    pub fn close_all(&mut self) -> Result<(), ServiceError> {
        let nums: Vec<i32> = self.files.keys().copied().collect();
        let mut first = None;
        for n in nums {
            if let Err(e) = self.close_one(n)
                && first.is_none()
            {
                first = Some(e);
            }
        }
        match first {
            Some(e) => Err(e.into()),
            None => Ok(()),
        }
    }

    // ---- locks ----------------------------------------------------------------------

    /// Error 70 when another file number on the same file holds a lock that
    /// covers the bytes `range` (`None` is the whole file).
    fn check_not_locked(&self, n: i32, range: Option<(u64, u64)>) -> R<()> {
        let Some(me) = self.files.get(&n) else {
            return Ok(());
        };
        for (m, other) in &self.files {
            if *m == n || other.key != me.key {
                continue;
            }
            let hit = other.whole_locked
                || match range {
                    None => !other.locks.is_empty(),
                    Some((lo, hi)) => other.locks.iter().any(|l| l.bytes.0 < hi && lo < l.bytes.1),
                };
            if hit {
                return vba(70);
            }
        }
        Ok(())
    }

    /// `Lock` and `Unlock` (MS-VBAL 5.4.5.4, 5.4.5.5). On a sequential file
    /// the range is ignored and the whole file is meant.
    fn lock(&mut self, n: i32, from: Option<i64>, to: Option<i64>, unlock: bool) -> R<()> {
        let f = self.file(n)?;
        let sequential = !matches!(f.mode, OpenMode::Binary | OpenMode::Random);
        if sequential || (from.is_none() && to.is_none()) {
            if unlock {
                f.whole_locked = false;
                f.locks.clear();
            } else {
                f.whole_locked = true;
            }
            return Ok(());
        }
        // `Lock #1, 3` is the single unit 3; `To 4` starts at 1.
        let start = from.unwrap_or(1);
        let end = to.or(from).unwrap_or(start);
        if start < 1 || start > end {
            return vba(5);
        }
        let unit = if f.mode == OpenMode::Random {
            f.record_len as u64
        } else {
            1
        };
        let bytes = (
            (start as u64 - 1).saturating_mul(unit),
            (end as u64).saturating_mul(unit),
        );
        if unlock {
            // The range must equal one that was locked.
            match f
                .locks
                .iter()
                .position(|l| l.start == start && l.end == end)
            {
                Some(i) => {
                    f.locks.remove(i);
                }
                None => return vba(5),
            }
        } else {
            f.locks.push(LockRange { start, end, bytes });
        }
        Ok(())
    }

    // ---- sequential output --------------------------------------------------------

    fn put_bytes(&mut self, n: i32, bytes: &[u8]) -> R<()> {
        self.check_not_locked(n, None)?;
        let max = self.max_file_bytes;
        let host = self.host.clone();
        let (f, sh) = self.chan(n)?;
        let end = f.pos + bytes.len();
        if end > max {
            return vba(61);
        }
        if end > sh.data.len() {
            host.budget()
                .alloc((end - sh.data.len()) as u64)
                .map_err(|_| FsError::Limit(LimitKind::Memory))?;
            sh.data.resize(end, 0);
        }
        sh.data[f.pos..end].copy_from_slice(bytes);
        f.pos = end;
        sh.dirty = true;
        Ok(())
    }

    fn print(&mut self, n: i32, items: &[PrintItem]) -> R<()> {
        let f = self.file(n)?;
        if !f.writable || f.mode == OpenMode::Binary {
            return vba(54);
        }
        let text = format_print_width(items, &mut f.col, f.width);
        let text = text.replace('\n', "\r\n");
        let bytes = encode_text(&VbaString::from(text));
        self.put_bytes(n, &bytes)
    }

    /// `Write #` (MS-VBAL 5.4.5.9): a comma follows each item unless it is
    /// the last one without a separator; `Spc` and `Tab` take a comma only
    /// from an explicit `,`; the line ends unless the list ends with a
    /// separator.
    fn write_stmt(&mut self, n: i32, items: &[PrintItem]) -> R<()> {
        let f = self.file(n)?;
        if !f.writable || f.mode == OpenMode::Binary {
            return vba(54);
        }
        let width = f.width;
        let mut out = LineOut::new(&mut f.col, width);
        let mut i = 0;
        while i < items.len() {
            let next_is_comma = |i: usize| matches!(items.get(i), Some(PrintItem::Comma));
            let next_is_sep =
                |i: usize| matches!(items.get(i), Some(PrintItem::Comma | PrintItem::Semicolon));
            match &items[i] {
                PrintItem::Value(v) => {
                    out.push(&write_value(v));
                    i += 1;
                    if next_is_sep(i) {
                        i += 1;
                        out.push(",");
                    } else if i < items.len() {
                        out.push(",");
                    }
                }
                PrintItem::Spc(k) => {
                    out.push(&" ".repeat((*k).max(0) as usize));
                    i += 1;
                    if next_is_comma(i) {
                        out.push(",");
                    }
                    if next_is_sep(i) {
                        i += 1;
                    }
                }
                PrintItem::Tab(Some(k)) => {
                    out.tab((*k).max(1) as usize - 1);
                    i += 1;
                    if next_is_comma(i) {
                        out.push(",");
                    }
                    if next_is_sep(i) {
                        i += 1;
                    }
                }
                PrintItem::Tab(None) => {
                    out.push(",");
                    i += 1;
                    if next_is_comma(i) {
                        out.push(",");
                    }
                    if next_is_sep(i) {
                        i += 1;
                    }
                }
                // A separator with no item before it stands for "".
                PrintItem::Comma | PrintItem::Semicolon => {
                    out.push(",");
                    i += 1;
                }
            }
        }
        if !matches!(items.last(), Some(PrintItem::Comma | PrintItem::Semicolon)) {
            out.newline();
        }
        let text = out.finish().replace('\n', "\r\n");
        let bytes = encode_text(&VbaString::from(text));
        self.put_bytes(n, &bytes)
    }

    // ---- sequential input -------------------------------------------------------------

    fn line_input(&mut self, n: i32) -> R<VbaString> {
        self.check_not_locked(n, None)?;
        let (f, sh) = self.chan(n)?;
        if !f.readable || f.mode == OpenMode::Random {
            return vba(54);
        }
        if f.pos >= sh.data.len() {
            return vba(62);
        }
        let rest = &sh.data[f.pos..];
        let end = rest
            .iter()
            .position(|b| *b == b'\r' || *b == b'\n')
            .unwrap_or(rest.len());
        let text = decode_text(&rest[..end]);
        let mut next = end;
        if rest.get(next) == Some(&b'\r') {
            next += 1;
        }
        if rest.get(next) == Some(&b'\n') {
            next += 1;
        }
        f.pos += next;
        Ok(text)
    }

    fn input_items(&mut self, n: i32, kinds: &[VarType]) -> R<Vec<Variant>> {
        self.check_not_locked(n, None)?;
        let (f, sh) = self.chan(n)?;
        if !f.readable || f.mode != OpenMode::Input {
            return vba(54);
        }
        let mut out = Vec::new();
        for k in kinds {
            out.push(read_token(f, &sh.data, *k)?);
        }
        Ok(out)
    }

    // ---- Get and Put ---------------------------------------------------------------------

    fn record_pos(f: &OpenFile, record: Option<i64>) -> R<usize> {
        match record {
            None => Ok(f.pos),
            Some(r) if r >= 1 => {
                let r = (r - 1) as usize;
                if f.mode == OpenMode::Random {
                    r.checked_mul(f.record_len).ok_or(FsError::Vba(63))
                } else {
                    Ok(r)
                }
            }
            Some(_) => vba(63),
        }
    }

    fn get(&mut self, n: i32, record: Option<i64>, shape: &Shape) -> R<RecordValue> {
        let (f, _) = self.chan(n)?;
        if !f.readable || !matches!(f.mode, OpenMode::Binary | OpenMode::Random) {
            return vba(54);
        }
        let random = f.mode == OpenMode::Random;
        let pos = Self::record_pos(f, record)?;
        let record_len = f.record_len;
        let (v, end) = {
            let (_, sh) = self.chan(n)?;
            if random {
                if pos >= sh.data.len() {
                    return vba(62);
                }
                let end = (pos + record_len).min(sh.data.len());
                let mut buf = sh.data[pos..end].to_vec();
                buf.resize(record_len, 0);
                let mut r = Reader::new(&buf, 0);
                (records::decode(shape, &mut r, true)?, pos + record_len)
            } else {
                let mut r = Reader::new(&sh.data, pos);
                (records::decode(shape, &mut r, false)?, r.pos())
            }
        };
        self.check_not_locked(n, Some((pos as u64, end as u64)))?;
        let f = self.file(n)?;
        if random {
            f.last_record = (pos / record_len + 1) as i64;
        }
        f.pos = end;
        Ok(v)
    }

    fn put(&mut self, n: i32, record: Option<i64>, shape: &Shape, value: &RecordValue) -> R<()> {
        let max = self.max_file_bytes;
        let host = self.host.clone();
        let (f, _) = self.chan(n)?;
        if !f.writable || !matches!(f.mode, OpenMode::Binary | OpenMode::Random) {
            return vba(54);
        }
        let random = f.mode == OpenMode::Random;
        let pos = Self::record_pos(f, record)?;
        let mut buf = Vec::new();
        records::encode(shape, value, random, &mut buf)?;
        let record_len = f.record_len;
        if random {
            if buf.len() > record_len {
                return vba(59);
            }
            buf.resize(record_len, 0);
        }
        let end = pos + buf.len();
        self.check_not_locked(n, Some((pos as u64, end as u64)))?;
        if end > max {
            return vba(61);
        }
        let (f, sh) = self.chan(n)?;
        if end > sh.data.len() {
            host.budget()
                .alloc((end - sh.data.len()) as u64)
                .map_err(|_| FsError::Limit(LimitKind::Memory))?;
            sh.data.resize(end, 0);
        }
        sh.data[pos..end].copy_from_slice(&buf);
        f.pos = end;
        sh.dirty = true;
        if random {
            f.last_record = (pos / f.record_len + 1) as i64;
        }
        Ok(())
    }

    fn seek(&mut self, n: i32, position: i64) -> R<()> {
        let f = self.file(n)?;
        if position < 1 {
            return vba(63);
        }
        let p = (position - 1) as usize;
        f.pos = if f.mode == OpenMode::Random {
            p.checked_mul(f.record_len).ok_or(FsError::Vba(63))?
        } else {
            p
        };
        Ok(())
    }

    // ---- path functions -----------------------------------------------------------------

    fn path_arg(&mut self, v: &Option<Variant>) -> R<String> {
        match v {
            Some(v) => Ok(conv(convert::to_vba_string(v))?.to_string_lossy()),
            None => vba(449),
        }
    }

    fn stat_checked(&mut self, p: &str) -> R<Stat> {
        self.vfs.stat(p).map_err(map_vfs)?.ok_or(FsError::Vba(53))
    }

    fn split_pattern(&mut self, pattern: &str) -> R<(String, String)> {
        let n = normalize(&self.cwd, pattern);
        let (dir, name) = split_parent(&n);
        Ok((self.vfs.canonical(&dir), name))
    }

    fn kill(&mut self, pattern: &str) -> R<()> {
        let (dir, name) = self.split_pattern(pattern)?;
        // Nothing about the directory may be learned (not even whether it or
        // the file exists) before the policy has agreed.
        if name.contains(['*', '?']) {
            self.allow(Capability::FileWrite, &dir_resource(&dir))?;
        } else {
            self.allow(Capability::FileWrite, &join(&dir, &name))?;
        }
        let entries = match self.vfs.list(&dir) {
            Ok(e) => e,
            Err(e) => return Err(map_vfs(e)),
        };
        let mut any = false;
        for e in entries {
            if !e.stat.is_dir && wildcard(&name, &e.name) {
                any = true;
                let p = join(&dir, &e.name);
                self.allow(Capability::FileWrite, &p)?;
                if self.files.values().any(|f| f.path.eq_ignore_ascii_case(&p)) {
                    return vba(55);
                }
                self.vfs.remove_file(&p).map_err(map_vfs)?;
            }
        }
        if any { Ok(()) } else { vba(53) }
    }

    fn dir(&mut self, pattern: Option<String>, attrs: i32) -> R<String> {
        if let Some(pat) = pattern {
            let (dir, name) = self.split_pattern(&pat)?;
            self.allow(Capability::FileRead, &dir_resource(&dir))?;
            let mut names = VecDeque::new();
            if name.contains(['*', '?']) {
                let entries = match self.vfs.list(&dir) {
                    Ok(e) => e,
                    Err(VfsError::PathNotFound) => return vba(52),
                    Err(e) => return Err(map_vfs(e)),
                };
                for e in entries {
                    let a = i32::from(e.stat.attributes);
                    let wants_dir = e.stat.is_dir;
                    let ok = wildcard(&name, &e.name)
                        && (!wants_dir || attrs & 16 != 0)
                        && (a & 2 == 0 || attrs & 2 != 0)
                        && (a & 4 == 0 || attrs & 4 != 0);
                    if ok {
                        names.push_back(e.name);
                    }
                }
            } else {
                let p = join(&dir, &name);
                if let Some(st) = self.vfs.stat(&p).map_err(map_vfs)?
                    && (!st.is_dir || attrs & 16 != 0)
                {
                    names.push_back(name);
                }
            }
            self.dir_iter = Some(names);
        }
        match self.dir_iter.as_mut() {
            Some(it) => Ok(it.pop_front().unwrap_or_default()),
            None => vba(5),
        }
    }

    fn chdir(&mut self, path: &str) -> R<()> {
        let p = self.resolve(path)?;
        self.allow(Capability::FileRead, &dir_resource(&p))?;
        match self.vfs.stat(&p).map_err(map_vfs)? {
            Some(s) if s.is_dir => {
                self.cwd = p;
                Ok(())
            }
            _ => vba(76),
        }
    }

    fn name(&mut self, old: &str, new: &str) -> R<()> {
        let o = self.resolve(old)?;
        let n = self.resolve(new)?;
        self.allow(Capability::FileWrite, &o)?;
        self.allow(Capability::FileWrite, &n)?;
        if self.files.values().any(|f| f.path.eq_ignore_ascii_case(&o)) {
            return vba(55);
        }
        if self.vfs.stat(&o).map_err(map_vfs)?.is_none() {
            return vba(53);
        }
        self.vfs.rename(&o, &n).map_err(map_vfs)
    }

    fn call_fn(&mut self, name: &str, a: &[Option<Variant>]) -> R<Variant> {
        let num = |a: &[Option<Variant>], i: usize| -> R<i32> {
            match a.get(i) {
                Some(Some(v)) => conv(convert::to_long(v)),
                _ => vba(449),
            }
        };
        match name {
            "freefile" => {
                let range = match a.first() {
                    Some(Some(v)) => conv(convert::to_long(v))?,
                    _ => 0,
                };
                let (lo, hi) = match range {
                    0 => (1, 255),
                    1 => (256, 511),
                    _ => return vba(5),
                };
                match (lo..=hi).find(|n| !self.files.contains_key(n)) {
                    Some(n) => Ok(Variant::Integer(n as i16)),
                    None => vba(67),
                }
            }
            "input" | "inputb" => {
                // `Input(number, #file)`: **doc** Input or Binary files only;
                // reading past the end is error 62.
                let count = num(a, 0)?;
                let n = num(a, 1)?;
                self.check_not_locked(n, None)?;
                let (f, sh) = self.chan(n)?;
                if !f.readable || !matches!(f.mode, OpenMode::Input | OpenMode::Binary) {
                    return vba(54);
                }
                let count = usize::try_from(count).map_err(|_| FsError::Vba(5))?;
                if sh.data.len() - f.pos.min(sh.data.len()) < count {
                    return vba(62);
                }
                let text = decode_text(&sh.data[f.pos..f.pos + count]);
                f.pos += count;
                Ok(Variant::String(text))
            }
            "fileattr" => {
                // **doc**: return type 1 is the mode (Input 1, Output 2,
                // Random 4, Append 8, Binary 32); type 2 (the OS handle) is
                // an error on 32-bit systems, error 5 here (**unverified**).
                let n = num(a, 0)?;
                // MS-VBAL 6.1.2.5.1.4: ReturnType is optional and defaults to 1.
                let kind = match a.get(1) {
                    Some(Some(Variant::Empty)) | Some(None) | None => 1,
                    Some(Some(_)) => num(a, 1)?,
                };
                let f = self.file(n)?;
                if kind != 1 {
                    return vba(5);
                }
                Ok(Variant::Long(match f.mode {
                    OpenMode::Input => 1,
                    OpenMode::Output => 2,
                    OpenMode::Random => 4,
                    OpenMode::Append => 8,
                    OpenMode::Binary => 32,
                }))
            }
            "eof" => {
                let n = num(a, 0)?;
                let (f, sh) = self.chan(n)?;
                if !f.readable {
                    return vba(54);
                }
                Ok(Variant::Boolean(f.pos >= sh.data.len()))
            }
            "lof" => {
                let n = num(a, 0)?;
                Ok(Variant::Long(self.chan(n)?.1.data.len() as i32))
            }
            "loc" => {
                let n = num(a, 0)?;
                let f = self.file(n)?;
                Ok(Variant::Long(match f.mode {
                    OpenMode::Random => f.last_record as i32,
                    OpenMode::Binary => f.pos as i32,
                    _ => (f.pos / 128) as i32,
                }))
            }
            "seek" => {
                let n = num(a, 0)?;
                let f = self.file(n)?;
                Ok(Variant::Long(if f.mode == OpenMode::Random {
                    (f.pos / f.record_len + 1) as i32
                } else {
                    f.pos as i32 + 1
                }))
            }
            "kill" => {
                let p = self.path_arg(a.first().unwrap_or(&None))?;
                self.kill(&p)?;
                Ok(Variant::Empty)
            }
            "mkdir" => {
                let p = self.path_arg(a.first().unwrap_or(&None))?;
                let p = self.resolve(&p)?;
                self.allow(Capability::FileWrite, &p)?;
                match self.vfs.create_dir(&p) {
                    Ok(()) => Ok(Variant::Empty),
                    Err(VfsError::AlreadyExists) => vba(75),
                    Err(e) => Err(map_vfs(e)),
                }
            }
            "rmdir" => {
                let p = self.path_arg(a.first().unwrap_or(&None))?;
                let p = self.resolve(&p)?;
                self.allow(Capability::FileWrite, &p)?;
                self.vfs.remove_dir(&p).map_err(map_vfs)?;
                Ok(Variant::Empty)
            }
            "chdir" => {
                let p = self.path_arg(a.first().unwrap_or(&None))?;
                self.chdir(&p)?;
                Ok(Variant::Empty)
            }
            "chdrive" => {
                // There is one virtual working directory; a drive change is
                // only accepted when it names the drive already current.
                let d = self.path_arg(a.first().unwrap_or(&None))?;
                let want = d.chars().next().map(|c| c.to_ascii_lowercase());
                let cur = self
                    .cwd
                    .chars()
                    .next()
                    .filter(|_| self.cwd.as_bytes().get(1) == Some(&b':'));
                match (want, cur) {
                    // MS-VBAL 6.1.2.5.2.2: an empty string leaves the drive unchanged.
                    (None, _) => Ok(Variant::Empty),
                    (Some(w), Some(c)) if w == c => Ok(Variant::Empty),
                    (Some(_), None) => Ok(Variant::Empty),
                    _ => vba(68),
                }
            }
            "curdir" => Ok(Variant::from_text(if self.cwd_is_visible() {
                &self.cwd
            } else {
                "/"
            })),
            "dir" => {
                let pat = match a.first() {
                    Some(Some(Variant::Empty)) | Some(None) | None => None,
                    Some(Some(v)) => Some(conv(convert::to_vba_string(v))?.to_string_lossy()),
                };
                let attrs = match a.get(1) {
                    Some(Some(v)) => conv(convert::to_long(v))?,
                    _ => 0,
                };
                Ok(Variant::from_text(&self.dir(pat, attrs)?))
            }
            "filecopy" => {
                let s = self.path_arg(a.first().unwrap_or(&None))?;
                let d = self.path_arg(a.get(1).unwrap_or(&None))?;
                let s = self.resolve(&s)?;
                let d = self.resolve(&d)?;
                self.allow(Capability::FileRead, &s)?;
                self.allow(Capability::FileWrite, &d)?;
                if self.files.values().any(|f| {
                    f.path.eq_ignore_ascii_case(&d)
                        || (f.path.eq_ignore_ascii_case(&s) && f.writable)
                }) {
                    return vba(55);
                }
                self.stat_checked(&s)?;
                self.vfs.copy(&s, &d).map_err(map_vfs)?;
                Ok(Variant::Empty)
            }
            "filelen" => {
                let p = self.path_arg(a.first().unwrap_or(&None))?;
                let p = self.resolve(&p)?;
                self.allow(Capability::FileRead, &p)?;
                let st = self.stat_checked(&p)?;
                // An open file reports its current size, as it would on disk.
                let open = self
                    .buffers
                    .get(&p.to_ascii_lowercase())
                    .map(|sh| sh.data.len() as u64);
                Ok(Variant::Long(open.unwrap_or(st.len) as i32))
            }
            "filedatetime" => {
                let p = self.path_arg(a.first().unwrap_or(&None))?;
                let p = self.resolve(&p)?;
                self.allow(Capability::FileRead, &p)?;
                let st = self.stat_checked(&p)?;
                Ok(Variant::Date(conv(Date::from_serial(st.modified))?))
            }
            "getattr" => {
                let p = self.path_arg(a.first().unwrap_or(&None))?;
                let p = self.resolve(&p)?;
                self.allow(Capability::FileRead, &p)?;
                let st = self.stat_checked(&p)?;
                Ok(Variant::Integer(i16::from(st.attributes)))
            }
            "setattr" => {
                let p = self.path_arg(a.first().unwrap_or(&None))?;
                let attrs = num(a, 1)?;
                let p = self.resolve(&p)?;
                self.allow(Capability::FileWrite, &p)?;
                self.stat_checked(&p)?;
                self.vfs
                    .set_attributes(&p, (attrs & 0xFF) as u8)
                    .map_err(map_vfs)?;
                Ok(Variant::Empty)
            }
            "shell" => {
                let cmd = self.path_arg(a.first().unwrap_or(&None))?;
                let style = match a.get(1) {
                    Some(Some(v)) => conv(convert::to_integer(v))?,
                    _ => 1,
                };
                self.allow(Capability::ProcessSpawn, &cmd)?;
                match self.shell.spawn(&cmd, style) {
                    Ok(pid) => Ok(Variant::Double(f64::from(pid))),
                    Err(e) => Err(e),
                }
            }
            "command" => Ok(Variant::from_text(&self.command)),
            other => Err(FsError::Unsupported(format!(
                "the run-time function `{other}`"
            ))),
        }
    }

    fn io_request(&mut self, req: IoRequest) -> R<IoReply> {
        match req {
            IoRequest::Open {
                path,
                mode,
                access,
                lock,
                file,
                record_len,
            } => {
                self.open(&path, mode, access, lock, file, record_len)?;
                Ok(IoReply::Done)
            }
            IoRequest::Close(files) => {
                if files.is_empty() {
                    self.close_all().map_err(|e| match e {
                        ServiceError::Vba(i) => FsError::Info(i),
                        ServiceError::Unsupported(s) => FsError::Unsupported(s),
                        ServiceError::Limit(l) => FsError::Limit(l),
                    })?;
                } else {
                    for n in files {
                        self.close_one(n)?;
                    }
                }
                Ok(IoReply::Done)
            }
            IoRequest::Print { file, items } => {
                self.print(file, &items)?;
                Ok(IoReply::Done)
            }
            IoRequest::Write { file, items } => {
                self.write_stmt(file, &items)?;
                Ok(IoReply::Done)
            }
            IoRequest::Input { file, targets } => {
                Ok(IoReply::Values(self.input_items(file, &targets)?))
            }
            IoRequest::LineInput { file } => Ok(IoReply::Line(self.line_input(file)?)),
            IoRequest::Get {
                file,
                record,
                shape,
            } => Ok(IoReply::Record(self.get(file, record, &shape)?)),
            IoRequest::Put {
                file,
                record,
                shape,
                value,
            } => {
                self.put(file, record, &shape, &value)?;
                Ok(IoReply::Done)
            }
            IoRequest::Seek { file, position } => {
                self.seek(file, position)?;
                Ok(IoReply::Done)
            }
            IoRequest::Lock { file, from, to } => {
                self.lock(file, from, to, false)?;
                Ok(IoReply::Done)
            }
            IoRequest::Unlock { file, from, to } => {
                self.lock(file, from, to, true)?;
                Ok(IoReply::Done)
            }
            IoRequest::Width { file, width } => {
                let f = self.file(file)?;
                f.width = width.max(0) as usize;
                Ok(IoReply::Done)
            }
            IoRequest::Name { old, new } => {
                self.name(&old, &new)?;
                Ok(IoReply::Done)
            }
        }
    }
}

impl Drop for FileSystem {
    fn drop(&mut self) {
        let _ = self.close_all();
    }
}

/// Output for one `Write #` statement, wrapping at the `Width` of the file.
struct LineOut<'a> {
    out: String,
    col: &'a mut usize,
    width: usize,
}

impl<'a> LineOut<'a> {
    fn new(col: &'a mut usize, width: usize) -> Self {
        LineOut {
            out: String::new(),
            col,
            width,
        }
    }

    /// Writes characters; a line that reaches the width ends first.
    fn push(&mut self, text: &str) {
        for ch in text.chars() {
            if self.width > 0 && *self.col >= self.width {
                self.newline();
            }
            self.out.push(ch);
            *self.col += 1;
        }
    }

    fn newline(&mut self) {
        self.out.push('\n');
        *self.col = 0;
    }

    /// Moves to the 0-based column `target`, on the next line when the
    /// position is already past it.
    fn tab(&mut self, target: usize) {
        let target = if self.width > 0 {
            target % self.width
        } else {
            target
        };
        if target < *self.col {
            self.newline();
        }
        let gap = target - *self.col;
        self.out.push_str(&" ".repeat(gap));
        *self.col = target;
    }

    fn finish(self) -> String {
        self.out
    }
}

/// The text `Write #` produces for one value.
pub fn write_value(v: &Variant) -> String {
    match v {
        Variant::Empty => String::new(),
        Variant::Null => "#NULL#".to_owned(),
        Variant::Boolean(b) => if *b { "#TRUE#" } else { "#FALSE#" }.to_owned(),
        Variant::String(s) => format!("\"{}\"", s.to_string_lossy()),
        Variant::Error(e) => format!("#ERROR {}#", e.number()),
        Variant::Date(d) => {
            let p = d.parts();
            let date = format!("{:04}-{:02}-{:02}", p.year, p.month, p.day);
            let time = format!("{:02}:{:02}:{:02}", p.hour, p.minute, p.second);
            let (has_date, has_time) = (d.serial().trunc() != 0.0, d.serial().fract() != 0.0);
            match (has_date, has_time) {
                (true, true) => format!("#{date} {time}#"),
                (false, true) => format!("#{time}#"),
                _ => format!("#{date}#"),
            }
        }
        other => convert::to_vba_string(other)
            .map(|s| s.to_string_lossy())
            .unwrap_or_default(),
    }
}

fn read_token(f: &mut OpenFile, d: &[u8], kind: VarType) -> R<Variant> {
    let mut i = f.pos;
    while i < d.len() && matches!(d[i], b' ' | b'\t' | b'\r' | b'\n') {
        i += 1;
    }
    if i >= d.len() {
        f.pos = i;
        return vba(62);
    }
    let quoted = d[i] == b'"';
    let text: Vec<u8>;
    if quoted {
        i += 1;
        let mut t = Vec::new();
        while i < d.len() {
            if d[i] == b'"' {
                if d.get(i + 1) == Some(&b'"') {
                    t.push(b'"');
                    i += 2;
                    continue;
                }
                i += 1;
                break;
            }
            t.push(d[i]);
            i += 1;
        }
        text = t;
        while i < d.len() && d[i] == b' ' {
            i += 1;
        }
    } else {
        let start = i;
        while i < d.len() && !matches!(d[i], b',' | b'\r' | b'\n') {
            i += 1;
        }
        text = d[start..i].to_vec();
    }
    // Consume one delimiter.
    match d.get(i) {
        Some(b',') => i += 1,
        Some(b'\r') => {
            i += 1;
            if d.get(i) == Some(&b'\n') {
                i += 1;
            }
        }
        Some(b'\n') => i += 1,
        _ => {}
    }
    f.pos = i;
    let s = decode_text(&text);
    let trimmed = decode_text(text.trim_ascii());
    let upper = trimmed.to_string_lossy().to_uppercase();
    match kind {
        VarType::String => Ok(Variant::String(s)),
        VarType::Boolean => match upper.as_str() {
            "#TRUE#" => Ok(Variant::Boolean(true)),
            "#FALSE#" => Ok(Variant::Boolean(false)),
            _ => {
                let n = conv(vba_stdlib::conversion::val(&trimmed))?;
                Ok(Variant::Boolean(n != 0.0))
            }
        },
        VarType::Date => {
            let inner = upper.trim_matches('#');
            let d = conv(vba_value::text::parse_date(&VbaString::from(inner)))?;
            Ok(Variant::Date(d))
        }
        VarType::Variant | VarType::Empty => {
            if quoted {
                return Ok(Variant::String(s));
            }
            match upper.as_str() {
                "#NULL#" => Ok(Variant::Null),
                "#TRUE#" => Ok(Variant::Boolean(true)),
                "#FALSE#" => Ok(Variant::Boolean(false)),
                u if u.starts_with('#') && u.ends_with('#') && u.len() > 2 => {
                    let inner = u.trim_matches('#');
                    if let Some(n) = inner.strip_prefix("ERROR ") {
                        let n = n.trim().parse::<i32>().unwrap_or(0);
                        Ok(Variant::Error(vba_value::VbaError::new(n)))
                    } else {
                        let d = conv(vba_value::text::parse_date(&VbaString::from(inner)))?;
                        Ok(Variant::Date(d))
                    }
                }
                _ => {
                    let n = conv(vba_stdlib::conversion::val(&trimmed))?;
                    if n.fract() == 0.0 && n.abs() < 32768.0 && !upper.contains(['.', 'E']) {
                        Ok(Variant::Integer(n as i16))
                    } else if n.fract() == 0.0
                        && n.abs() < 2_147_483_648.0
                        && !upper.contains(['.', 'E'])
                    {
                        Ok(Variant::Long(n as i32))
                    } else {
                        Ok(Variant::Double(n))
                    }
                }
            }
        }
        _ => {
            if upper == "#NULL#" {
                return Err(FsError::Vba(94));
            }
            let n = conv(vba_stdlib::conversion::val(&trimmed))?;
            conv(convert::convert_to(&Variant::Double(n), kind))
        }
    }
}

// ---- the Scripting.FileSystemObject view --------------------------------------------

fn to_info(e: FsError) -> ErrInfo {
    match e {
        FsError::Vba(n) => ErrInfo::new(n, vba_vm::describe(n)),
        FsError::Info(i) => i,
        FsError::Unsupported(s) => ErrInfo::new(5, format!("unsupported: {s}")),
        FsError::Limit(_) => ErrInfo::standard(7),
    }
}

fn script_stat(s: Stat) -> vba_scripting::EntryStat {
    vba_scripting::EntryStat {
        is_dir: s.is_dir,
        len: s.len,
        modified: s.modified,
        attributes: s.attributes,
    }
}

impl FileSystem {
    /// A grant for a folder may be written with or without the trailing
    /// slash; either form of the resource is accepted.
    fn allow_entry(&self, cap: Capability, abs: &str) -> R<()> {
        self.allow(cap, abs)
            .or_else(|e| self.allow(cap, &dir_resource(abs)).map_err(|_| e))
    }

    fn script_open_file_guard(&self, abs: &str) -> R<()> {
        if self
            .files
            .values()
            .any(|f| f.path.eq_ignore_ascii_case(abs))
        {
            return vba(55);
        }
        Ok(())
    }

    fn script_stat(&mut self, abs: &str) -> R<Option<Stat>> {
        self.allow_entry(Capability::FileRead, abs)?;
        self.vfs.stat(abs).map_err(map_vfs)
    }

    fn script_read(&mut self, abs: &str) -> R<Vec<u8>> {
        self.allow(Capability::FileRead, abs)?;
        let data = self
            .vfs
            .read_limited(abs, self.max_file_bytes)
            .map_err(map_vfs)?;
        self.charge(data.len())?;
        Ok(data)
    }

    fn script_write(&mut self, abs: &str, data: &[u8]) -> R<()> {
        self.allow(Capability::FileWrite, abs)?;
        if data.len() > self.max_file_bytes {
            return vba(61);
        }
        self.script_open_file_guard(abs)?;
        self.charge(data.len())?;
        self.vfs.write(abs, data).map_err(map_vfs)
    }
}

/// The file system of a session as seen by `Scripting.FileSystemObject`:
/// every call is checked against the host policy exactly like the file
/// statements. It holds the session's file system weakly, so it creates no
/// reference cycle with the host that owns it.
pub struct ScriptFsAdapter(pub std::rc::Weak<std::cell::RefCell<FileSystem>>);

impl ScriptFsAdapter {
    fn with<T>(&self, f: impl FnOnce(&mut FileSystem) -> R<T>) -> vba_scripting::FsResult<T> {
        let fs = self
            .0
            .upgrade()
            .ok_or_else(|| ErrInfo::new(5, "the session's file system is gone"))?;
        let mut guard = fs
            .try_borrow_mut()
            .map_err(|_| ErrInfo::new(5, "the file system is busy"))?;
        f(&mut guard).map_err(to_info)
    }
}

impl vba_scripting::ScriptFs for ScriptFsAdapter {
    fn resolve(&self, path: &str) -> vba_scripting::FsResult<String> {
        self.with(|f| f.resolve(path))
    }
    fn stat(&self, abs: &str) -> vba_scripting::FsResult<Option<vba_scripting::EntryStat>> {
        self.with(|f| Ok(f.script_stat(abs)?.map(script_stat)))
    }
    fn read(&self, abs: &str) -> vba_scripting::FsResult<Vec<u8>> {
        self.with(|f| f.script_read(abs))
    }
    fn write(&self, abs: &str, data: &[u8]) -> vba_scripting::FsResult<()> {
        self.with(|f| f.script_write(abs, data))
    }
    fn remove_file(&self, abs: &str) -> vba_scripting::FsResult<()> {
        self.with(|f| {
            f.allow(Capability::FileWrite, abs)?;
            f.script_open_file_guard(abs)?;
            f.vfs.remove_file(abs).map_err(map_vfs)
        })
    }
    fn remove_dir(&self, abs: &str) -> vba_scripting::FsResult<()> {
        self.with(|f| {
            f.allow_entry(Capability::FileWrite, abs)?;
            f.vfs.remove_dir(abs).map_err(map_vfs)
        })
    }
    fn create_dir(&self, abs: &str) -> vba_scripting::FsResult<()> {
        self.with(|f| {
            f.allow_entry(Capability::FileWrite, abs)?;
            f.vfs.create_dir(abs).map_err(map_vfs)
        })
    }
    fn copy_file(&self, from: &str, to: &str) -> vba_scripting::FsResult<()> {
        self.with(|f| {
            f.allow(Capability::FileRead, from)?;
            f.allow(Capability::FileWrite, to)?;
            f.script_open_file_guard(to)?;
            f.vfs.copy(from, to).map_err(map_vfs)
        })
    }
    fn rename(&self, from: &str, to: &str) -> vba_scripting::FsResult<()> {
        self.with(|f| {
            f.allow_entry(Capability::FileWrite, from)?;
            f.allow_entry(Capability::FileWrite, to)?;
            f.script_open_file_guard(from)?;
            f.vfs.rename(from, to).map_err(map_vfs)
        })
    }
    fn list(&self, abs: &str) -> vba_scripting::FsResult<Vec<vba_scripting::EntryInfo>> {
        self.with(|f| {
            f.allow(Capability::FileRead, &dir_resource(abs))?;
            Ok(f.vfs
                .list(abs)
                .map_err(map_vfs)?
                .into_iter()
                .map(|e| vba_scripting::EntryInfo {
                    name: e.name,
                    stat: script_stat(e.stat),
                })
                .collect())
        })
    }
    fn set_attributes(&self, abs: &str, attributes: u8) -> vba_scripting::FsResult<()> {
        self.with(|f| {
            f.allow_entry(Capability::FileWrite, abs)?;
            f.vfs.set_attributes(abs, attributes).map_err(map_vfs)
        })
    }
}

/// The services handed to the VM.
pub struct FsServices(pub Rc<std::cell::RefCell<FileSystem>>);

impl Services for FsServices {
    fn io(&mut self, req: IoRequest) -> Result<IoReply, ServiceError> {
        self.0.borrow_mut().io_request(req).map_err(Into::into)
    }

    fn call(&mut self, name: &str, args: &[Option<Variant>]) -> Result<Variant, ServiceError> {
        let lname = name.to_ascii_lowercase();
        self.0
            .borrow_mut()
            .call_fn(&lname, args)
            .map_err(Into::into)
    }

    fn resolve_object_path(&mut self, path: &str) -> Result<String, ServiceError> {
        self.0
            .borrow_mut()
            .resolve_for_read(path)
            .map_err(Into::into)
    }

    fn reset(&mut self) {
        let _ = self.0.borrow_mut().close_all();
    }
}
