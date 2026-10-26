//! `Scripting.FileSystemObject`.
//!
//! Implemented from the Learn pages of the Scripting runtime library
//! (FileSystemObject object and its methods). Every file access goes through
//! [`ScriptFs`], which applies the host policy; the object itself holds no
//! path or permission of its own.
//!
//! Provided: `BuildPath`, `CopyFile`, `CopyFolder`, `CreateFolder`,
//! `CreateTextFile`, `DeleteFile`, `DeleteFolder`, `FileExists`,
//! `FolderExists`, `GetAbsolutePathName`, `GetBaseName`, `GetDriveName`,
//! `GetExtensionName`, `GetFile`, `GetFileName`, `GetFolder`,
//! `GetParentFolderName`, `GetTempName`, `MoveFile`, `MoveFolder`,
//! `OpenTextFile`. **Not provided** (the member does not exist, so a call is
//! error 438): `Drives`, `GetDrive`, `DriveExists`, `GetSpecialFolder`; a
//! virtual file system has no drives or Windows folders to describe.
//!
//! Error numbers are not on the pages and are **unverified**: 53 file not
//! found, 76 path not found, 58 file already exists, 70 permission denied
//! (also for the host policy), 5 invalid argument.
//!
//! Deviations by design: paths are the virtual file system's canonical
//! absolute paths (forward slashes, `c:/dir` for a drive), so
//! `GetAbsolutePathName` and `Path` properties do not look like `C:\dir`;
//! `GetTempName` is a counter (`rad00001.tmp`), not random.

use std::cell::Cell;
use std::rc::Rc;
use vba_automation::{
    Bound, DispError, ErrInfo, InvokeFlags, ObjectHandle, ObjectRef, ObjectTable,
};
use vba_value::Variant;

use crate::entries::{File, Folder};
use crate::fs::{
    EntryInfo, ScriptFs, base_name, build_path, extension, file_name, join, last_component,
    parent_name, parent_of, wildcard,
};
use crate::textstream::{Mode, TextStream};
use crate::util::{Def, dispatch_impl, getter, method, opt_bool, opt_long, text_arg};

const BUILD_PATH: i32 = 1;
const COPY_FILE: i32 = 2;
const COPY_FOLDER: i32 = 3;
const CREATE_FOLDER: i32 = 4;
const CREATE_TEXT_FILE: i32 = 5;
const DELETE_FILE: i32 = 6;
const DELETE_FOLDER: i32 = 7;
const FILE_EXISTS: i32 = 8;
const FOLDER_EXISTS: i32 = 9;
const GET_ABSOLUTE_PATH_NAME: i32 = 10;
const GET_BASE_NAME: i32 = 11;
const GET_EXTENSION_NAME: i32 = 12;
const GET_FILE: i32 = 13;
const GET_FILE_NAME: i32 = 14;
const GET_FOLDER: i32 = 15;
const GET_PARENT_FOLDER_NAME: i32 = 16;
const GET_TEMP_NAME: i32 = 17;
const MOVE_FILE: i32 = 18;
const MOVE_FOLDER: i32 = 19;
const OPEN_TEXT_FILE: i32 = 20;
const GET_DRIVE_NAME: i32 = 21;

pub(crate) const SOURCE: &str = "FileSystemObject";

pub(crate) fn err(number: i32) -> DispError {
    DispError::from(ErrInfo::standard(number).with_source(SOURCE))
}

/// Strips one trailing separator (a folder spec may end with one).
fn trim_sep(p: &str) -> &str {
    if p.len() > 1 {
        p.trim_end_matches(['/', '\\'])
    } else {
        p
    }
}

fn ends_with_sep(p: &str) -> bool {
    p.ends_with(['/', '\\'])
}

/// The size of a folder: all files below it.
pub(crate) fn folder_size(fs: &dyn ScriptFs, abs: &str) -> Result<u64, ErrInfo> {
    let mut total = 0;
    for e in fs.list(abs)? {
        if e.stat.is_dir {
            total += folder_size(fs, &join(abs, &e.name))?;
        } else {
            total += e.stat.len;
        }
    }
    Ok(total)
}

/// Copies a folder tree to `to`, creating it. With `overwrite` false an
/// existing file is error 58; a present target folder is merged into.
pub(crate) fn copy_tree(
    fs: &dyn ScriptFs,
    from: &str,
    to: &str,
    overwrite: bool,
) -> Result<(), ErrInfo> {
    if fs.stat(to)?.is_none() {
        fs.create_dir(to)?;
    }
    for e in fs.list(from)? {
        let (src, dst) = (join(from, &e.name), join(to, &e.name));
        if e.stat.is_dir {
            copy_tree(fs, &src, &dst, overwrite)?;
        } else {
            if !overwrite && fs.stat(&dst)?.is_some() {
                return Err(ErrInfo::standard(58).with_source(SOURCE));
            }
            fs.copy_file(&src, &dst)?;
        }
    }
    Ok(())
}

/// Deletes a folder and everything below it.
pub(crate) fn delete_tree(fs: &dyn ScriptFs, abs: &str) -> Result<(), ErrInfo> {
    for e in fs.list(abs)? {
        let p = join(abs, &e.name);
        if e.stat.is_dir {
            delete_tree(fs, &p)?;
        } else {
            fs.remove_file(&p)?;
        }
    }
    fs.remove_dir(abs)
}

/// The entries a (possibly wildcarded) spec names: `(folder, entries)`.
fn expand(fs: &dyn ScriptFs, spec: &str, dirs: bool) -> Result<Vec<String>, ErrInfo> {
    let abs = fs.resolve(trim_sep(spec))?;
    let name = last_component(&abs).to_owned();
    if !name.contains(['*', '?']) {
        return Ok(vec![abs]);
    }
    let Some(parent) = parent_of(&abs) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for e in fs.list(&parent)? {
        if e.stat.is_dir == dirs && wildcard(&name, &e.name) {
            out.push(join(&parent, &e.name));
        }
    }
    Ok(out)
}

/// A `Scripting.FileSystemObject` object.
pub struct FileSystemObject {
    fs: Rc<dyn ScriptFs>,
    temp: Cell<u32>,
}

impl FileSystemObject {
    pub(crate) const CLASS: &'static str = SOURCE;
    pub(crate) const DEFS: &'static [Def] = &[
        method("BuildPath", BUILD_PATH, &["Path", "Name"], 2),
        method(
            "CopyFile",
            COPY_FILE,
            &["Source", "Destination", "OverWriteFiles"],
            2,
        ),
        method(
            "CopyFolder",
            COPY_FOLDER,
            &["Source", "Destination", "OverWriteFiles"],
            2,
        ),
        method("CreateFolder", CREATE_FOLDER, &["Path"], 1),
        method(
            "CreateTextFile",
            CREATE_TEXT_FILE,
            &["FileName", "Overwrite", "Unicode"],
            1,
        ),
        method("DeleteFile", DELETE_FILE, &["FileSpec", "Force"], 1),
        method("DeleteFolder", DELETE_FOLDER, &["FolderSpec", "Force"], 1),
        method("FileExists", FILE_EXISTS, &["FileSpec"], 1),
        method("FolderExists", FOLDER_EXISTS, &["FolderSpec"], 1),
        method(
            "GetAbsolutePathName",
            GET_ABSOLUTE_PATH_NAME,
            &["PathSpec"],
            1,
        ),
        method("GetBaseName", GET_BASE_NAME, &["Path"], 1),
        method("GetDriveName", GET_DRIVE_NAME, &["Path"], 1),
        method("GetExtensionName", GET_EXTENSION_NAME, &["Path"], 1),
        method("GetFile", GET_FILE, &["FilePath"], 1),
        method("GetFileName", GET_FILE_NAME, &["PathSpec"], 1),
        method("GetFolder", GET_FOLDER, &["FolderPath"], 1),
        method("GetParentFolderName", GET_PARENT_FOLDER_NAME, &["Path"], 1),
        getter("GetTempName", GET_TEMP_NAME, &[]),
        method("MoveFile", MOVE_FILE, &["Source", "Destination"], 2),
        method("MoveFolder", MOVE_FOLDER, &["Source", "Destination"], 2),
        method(
            "OpenTextFile",
            OPEN_TEXT_FILE,
            &["FileName", "IOMode", "Create", "Format"],
            1,
        ),
    ];

    /// A file system object over `fs`.
    pub fn new(fs: Rc<dyn ScriptFs>) -> FileSystemObject {
        FileSystemObject {
            fs,
            temp: Cell::new(0),
        }
    }

    fn obj(table: &ObjectTable, o: impl vba_automation::Dispatch + 'static) -> Variant {
        Variant::Object(ObjectRef::Handle(table.insert_obj(o)))
    }

    fn stream(
        &self,
        table: &ObjectTable,
        path: &str,
        mode: Mode,
        create: bool,
        unicode: bool,
    ) -> Result<Variant, DispError> {
        let abs = self.fs.resolve(path)?;
        let ts = TextStream::open(self.fs.clone(), &abs, mode, create, unicode)?;
        Ok(Self::obj(table, ts))
    }

    /// The destination of a copy or move of `src` to `dst`: a trailing
    /// separator or an existing folder means "into that folder".
    fn target(&self, src_abs: &str, dst: &str) -> Result<String, ErrInfo> {
        let abs = self.fs.resolve(trim_sep(dst))?;
        let into = ends_with_sep(dst) || self.fs.stat(&abs)?.is_some_and(|s| s.is_dir);
        Ok(if into {
            join(&abs, last_component(src_abs))
        } else {
            abs
        })
    }

    fn call(
        &self,
        table: &ObjectTable,
        id: i32,
        _: InvokeFlags,
        b: &Bound,
    ) -> Result<Variant, DispError> {
        let fs = &*self.fs;
        let s = |i: usize| text_arg(b, i);
        match id {
            BUILD_PATH => Ok(Variant::from_text(&build_path(&s(0)?, &s(1)?))),
            GET_BASE_NAME => Ok(Variant::from_text(&base_name(&s(0)?))),
            GET_EXTENSION_NAME => Ok(Variant::from_text(&extension(&s(0)?))),
            GET_FILE_NAME => Ok(Variant::from_text(file_name(&s(0)?))),
            GET_PARENT_FOLDER_NAME => Ok(Variant::from_text(&parent_name(&s(0)?))),
            GET_DRIVE_NAME => {
                let p = s(0)?;
                let b = p.as_bytes();
                Ok(Variant::from_text(
                    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
                        &p[..2]
                    } else {
                        ""
                    },
                ))
            }
            GET_ABSOLUTE_PATH_NAME => {
                let p = s(0)?;
                let abs = fs.resolve(if p.is_empty() { "." } else { &p })?;
                Ok(Variant::from_text(&abs))
            }
            GET_TEMP_NAME => {
                let n = self.temp.get() + 1;
                self.temp.set(n);
                Ok(Variant::from_text(&format!("rad{n:05}.tmp")))
            }
            FILE_EXISTS => {
                let abs = fs.resolve(&s(0)?)?;
                Ok(Variant::Boolean(fs.stat(&abs)?.is_some_and(|x| !x.is_dir)))
            }
            FOLDER_EXISTS => {
                let abs = fs.resolve(trim_sep(&s(0)?))?;
                Ok(Variant::Boolean(fs.stat(&abs)?.is_some_and(|x| x.is_dir)))
            }
            GET_FILE => {
                let abs = fs.resolve(&s(0)?)?;
                match fs.stat(&abs)? {
                    Some(st) if !st.is_dir => Ok(Self::obj(table, File::new(self.fs.clone(), abs))),
                    _ => Err(err(53)),
                }
            }
            GET_FOLDER => {
                let abs = fs.resolve(trim_sep(&s(0)?))?;
                match fs.stat(&abs)? {
                    Some(st) if st.is_dir => {
                        Ok(Self::obj(table, Folder::new(self.fs.clone(), abs)))
                    }
                    _ => Err(err(76)),
                }
            }
            CREATE_FOLDER => {
                let abs = fs.resolve(trim_sep(&s(0)?))?;
                if fs.stat(&abs)?.is_some() {
                    return Err(err(58));
                }
                fs.create_dir(&abs)?;
                Ok(Self::obj(table, Folder::new(self.fs.clone(), abs)))
            }
            CREATE_TEXT_FILE => {
                let path = s(0)?;
                let overwrite = opt_bool(b, 1, true)?;
                let unicode = opt_bool(b, 2, false)?;
                let abs = fs.resolve(&path)?;
                if !overwrite && fs.stat(&abs)?.is_some() {
                    return Err(err(58));
                }
                self.stream(table, &path, Mode::Write, true, unicode)
            }
            OPEN_TEXT_FILE => {
                let path = s(0)?;
                let mode = match opt_long(b, 1, 1)? {
                    1 => Mode::Read,
                    2 => Mode::Write,
                    8 => Mode::Append,
                    _ => return Err(err(5)),
                };
                let create = opt_bool(b, 2, false)?;
                // Tristate: -1 Unicode; 0 ASCII; -2 the system default
                // (assumed ASCII).
                let unicode = match opt_long(b, 3, 0)? {
                    -1 => true,
                    0 | -2 => false,
                    _ => return Err(err(5)),
                };
                self.stream(table, &path, mode, create, unicode)
            }
            DELETE_FILE => {
                let force = opt_bool(b, 1, false)?;
                let spec = s(0)?;
                let targets = expand(fs, &spec, false)?;
                let mut any = false;
                for t in targets {
                    match fs.stat(&t)? {
                        Some(st) if !st.is_dir => {
                            any = true;
                            if st.attributes & 1 != 0 {
                                if !force {
                                    return Err(err(70));
                                }
                                fs.set_attributes(&t, st.attributes & !1)?;
                            }
                            fs.remove_file(&t)?;
                        }
                        _ => {}
                    }
                }
                if any {
                    Ok(Variant::Empty)
                } else {
                    Err(err(53))
                }
            }
            DELETE_FOLDER => {
                let spec = s(0)?;
                let targets = expand(fs, &spec, true)?;
                let mut any = false;
                for t in targets {
                    if fs.stat(&t)?.is_some_and(|st| st.is_dir) {
                        any = true;
                        delete_tree(fs, &t)?;
                    }
                }
                if any {
                    Ok(Variant::Empty)
                } else {
                    Err(err(76))
                }
            }
            COPY_FILE | MOVE_FILE => {
                let overwrite = if id == COPY_FILE {
                    opt_bool(b, 2, true)?
                } else {
                    false
                };
                let (spec, dst) = (s(0)?, s(1)?);
                let sources = expand(fs, &spec, false)?;
                let mut any = false;
                for src in sources {
                    if !fs.stat(&src)?.is_some_and(|st| !st.is_dir) {
                        continue;
                    }
                    any = true;
                    let to = self.target(&src, &dst)?;
                    if fs.stat(&to)?.is_some() && !overwrite {
                        return Err(err(58));
                    }
                    if id == COPY_FILE {
                        fs.copy_file(&src, &to)?;
                    } else {
                        fs.rename(&src, &to)?;
                    }
                }
                if any {
                    Ok(Variant::Empty)
                } else {
                    Err(err(53))
                }
            }
            COPY_FOLDER | MOVE_FOLDER => {
                let overwrite = if id == COPY_FOLDER {
                    opt_bool(b, 2, true)?
                } else {
                    false
                };
                let (spec, dst) = (s(0)?, s(1)?);
                let sources = expand(fs, &spec, true)?;
                let mut any = false;
                for src in sources {
                    if !fs.stat(&src)?.is_some_and(|st| st.is_dir) {
                        continue;
                    }
                    any = true;
                    let to = self.target(&src, &dst)?;
                    if id == COPY_FOLDER {
                        if fs.stat(&to)?.is_some() && !overwrite {
                            return Err(err(58));
                        }
                        if to == src || to.starts_with(&format!("{}/", src.trim_end_matches('/'))) {
                            return Err(err(5));
                        }
                        copy_tree(fs, &src, &to, overwrite)?;
                    } else {
                        if fs.stat(&to)?.is_some() {
                            return Err(err(58));
                        }
                        fs.rename(&src, &to)?;
                    }
                }
                if any {
                    Ok(Variant::Empty)
                } else {
                    Err(err(76))
                }
            }
            _ => Err(DispError::MEMBER_NOT_FOUND),
        }
    }

    fn new_enum(&self, _: &ObjectTable) -> Result<Variant, DispError> {
        Err(DispError::MEMBER_NOT_FOUND)
    }

    fn on_terminate(&self, _: &ObjectTable) {}

    fn held(&self) -> Vec<ObjectHandle> {
        Vec::new()
    }
}

dispatch_impl!(FileSystemObject);

/// Lists the entries of `abs`, files or folders only.
pub(crate) fn list_kind(
    fs: &dyn ScriptFs,
    abs: &str,
    dirs: bool,
) -> Result<Vec<EntryInfo>, ErrInfo> {
    Ok(fs
        .list(abs)?
        .into_iter()
        .filter(|e| e.stat.is_dir == dirs)
        .collect())
}
