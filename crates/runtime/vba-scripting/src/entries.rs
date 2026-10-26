//! The `File` and `Folder` objects and the `Files` / `SubFolders`
//! collections of `Scripting.FileSystemObject`.
//!
//! Provided on both: `Name` (assignable: renames), `Path`, `ParentFolder`,
//! `Size`, `Type`, `DateCreated`, `DateLastAccessed`, `DateLastModified`,
//! `Attributes` (assignable), `ShortName`, `ShortPath`, `Copy`, `Delete`,
//! `Move`. `File` adds `OpenAsTextStream`; `Folder` adds `Files`,
//! `SubFolders`, `IsRootFolder` and `CreateTextFile`. Not provided: `Drive`.
//!
//! **Unverified**: the file system keeps one timestamp, so `DateCreated` and
//! `DateLastAccessed` report the modification time; `ShortName` and
//! `ShortPath` equal `Name` and `Path` (no 8.3 names); `Type` is
//! `"<EXT> File"` (real Windows asks the shell for names such as
//! `"Text Document"`) and `"File folder"`; the collections list in name
//! order.

use std::rc::Rc;
use vba_automation::{
    Bound, DispError, ErrInfo, InvokeFlags, ObjectHandle, ObjectRef, ObjectTable,
};
use vba_value::{Date, Variant};

use crate::enumobj::EnumObject;
use crate::fs::{EntryStat, ScriptFs, extension, join, last_component, parent_of};
use crate::fso::{SOURCE, copy_tree, delete_tree, err, folder_size, list_kind};
use crate::textstream::{Mode, TextStream};
use crate::util::{Def, dispatch_impl, getter, method, opt_bool, opt_long, property, text_arg};

const NAME: i32 = 1;
const PATH: i32 = 2;
const PARENT_FOLDER: i32 = 3;
const SIZE: i32 = 4;
const TYPE: i32 = 5;
const DATE_CREATED: i32 = 6;
const DATE_LAST_ACCESSED: i32 = 7;
const DATE_LAST_MODIFIED: i32 = 8;
const ATTRIBUTES: i32 = 9;
const SHORT_NAME: i32 = 10;
const SHORT_PATH: i32 = 11;
const COPY: i32 = 12;
const DELETE: i32 = 13;
const MOVE: i32 = 14;
const OPEN_AS_TEXT_STREAM: i32 = 15;
const IS_ROOT_FOLDER: i32 = 16;
const FILES: i32 = 17;
const SUB_FOLDERS: i32 = 18;
const CREATE_TEXT_FILE: i32 = 19;
const COUNT: i32 = 20;
const ITEM: i32 = 0;

fn stat_of(fs: &dyn ScriptFs, abs: &str, dir: bool) -> Result<EntryStat, DispError> {
    match fs.stat(abs)? {
        Some(s) if s.is_dir == dir => Ok(s),
        _ => Err(err(if dir { 76 } else { 53 })),
    }
}

fn date_of(s: &EntryStat) -> Variant {
    Variant::Date(Date::from_serial(s.modified).unwrap_or_default())
}

fn obj(table: &ObjectTable, o: impl vba_automation::Dispatch + 'static) -> Variant {
    Variant::Object(ObjectRef::Handle(table.insert_obj(o)))
}

/// Properties and methods common to `File` and `Folder`.
fn common(
    fs: &Rc<dyn ScriptFs>,
    abs: &str,
    dir: bool,
    table: &ObjectTable,
    id: i32,
    flags: InvokeFlags,
    b: &Bound,
) -> Result<Option<Variant>, DispError> {
    let st = || stat_of(&**fs, abs, dir);
    Ok(Some(match id {
        NAME if flags.is_write() => {
            let new = b.value.as_ref().map(crate::util::string_of).transpose()?;
            let new = new.map(|s| s.to_string_lossy()).unwrap_or_default();
            if new.is_empty() || new.contains(['/', '\\']) {
                return Err(err(5));
            }
            let parent = parent_of(abs).ok_or_else(|| err(5))?;
            let to = join(&parent, &new);
            if fs.stat(&to)?.is_some() {
                return Err(err(58));
            }
            fs.rename(abs, &to)?;
            // The object keeps designating the old path; a renamed entry
            // must be fetched again (documented deviation).
            Variant::Empty
        }
        NAME | SHORT_NAME => {
            st()?;
            Variant::from_text(last_component(abs))
        }
        PATH | SHORT_PATH => {
            st()?;
            Variant::from_text(abs)
        }
        PARENT_FOLDER => {
            st()?;
            match parent_of(abs) {
                Some(p) => obj(table, Folder::new(fs.clone(), p)),
                None => Variant::Object(ObjectRef::Nothing),
            }
        }
        TYPE => {
            st()?;
            if dir {
                Variant::from_text("File folder")
            } else {
                let e = extension(last_component(abs));
                Variant::from_text(&if e.is_empty() {
                    "File".to_owned()
                } else {
                    format!("{} File", e.to_uppercase())
                })
            }
        }
        DATE_CREATED | DATE_LAST_ACCESSED | DATE_LAST_MODIFIED => date_of(&st()?),
        ATTRIBUTES if flags.is_write() => {
            let v = b.value.clone().unwrap_or(Variant::Empty);
            let n = vba_value::convert::to_long(&v)?;
            // Only the settable bits (read-only, hidden, system, archive).
            let bits = u8::try_from(n & 0x27).map_err(|_| err(5))?;
            let keep = st()?.attributes & 0x10;
            fs.set_attributes(abs, bits | keep)?;
            Variant::Empty
        }
        ATTRIBUTES => Variant::Long(i32::from(st()?.attributes)),
        DELETE => {
            let force = opt_bool(b, 0, false)?;
            let s = st()?;
            if dir {
                delete_tree(&**fs, abs)?;
            } else {
                if s.attributes & 1 != 0 {
                    if !force {
                        return Err(err(70));
                    }
                    fs.set_attributes(abs, s.attributes & !1)?;
                }
                fs.remove_file(abs)?;
            }
            Variant::Empty
        }
        COPY | MOVE => {
            st()?;
            let dst = text_arg(b, 0)?;
            let overwrite = if id == COPY {
                opt_bool(b, 1, true)?
            } else {
                false
            };
            let abs_dst = fs.resolve(dst.trim_end_matches(['/', '\\']))?;
            let into = dst.ends_with(['/', '\\']) || fs.stat(&abs_dst)?.is_some_and(|s| s.is_dir);
            let to = if into {
                join(&abs_dst, last_component(abs))
            } else {
                abs_dst
            };
            if fs.stat(&to)?.is_some() && !overwrite {
                return Err(err(58));
            }
            match (id, dir) {
                (COPY, false) => fs.copy_file(abs, &to)?,
                (COPY, true) => {
                    if to == abs || to.starts_with(&format!("{}/", abs.trim_end_matches('/'))) {
                        return Err(err(5));
                    }
                    copy_tree(&**fs, abs, &to, overwrite)?;
                }
                _ => fs.rename(abs, &to)?,
            }
            Variant::Empty
        }
        _ => return Ok(None),
    }))
}

/// A `File` object.
pub struct File {
    fs: Rc<dyn ScriptFs>,
    abs: String,
}

impl File {
    pub(crate) const CLASS: &'static str = "File";
    pub(crate) const DEFS: &'static [Def] = &[
        property("Name", NAME, &[], false),
        getter("Path", PATH, &[]),
        getter("ParentFolder", PARENT_FOLDER, &[]),
        getter("Size", SIZE, &[]),
        getter("Type", TYPE, &[]),
        getter("DateCreated", DATE_CREATED, &[]),
        getter("DateLastAccessed", DATE_LAST_ACCESSED, &[]),
        getter("DateLastModified", DATE_LAST_MODIFIED, &[]),
        property("Attributes", ATTRIBUTES, &[], false),
        getter("ShortName", SHORT_NAME, &[]),
        getter("ShortPath", SHORT_PATH, &[]),
        method("Copy", COPY, &["Destination", "OverWriteFiles"], 1),
        method("Delete", DELETE, &["Force"], 0),
        method("Move", MOVE, &["Destination"], 1),
        method(
            "OpenAsTextStream",
            OPEN_AS_TEXT_STREAM,
            &["IOMode", "Format"],
            0,
        ),
    ];

    pub(crate) fn new(fs: Rc<dyn ScriptFs>, abs: String) -> File {
        File { fs, abs }
    }

    fn call(
        &self,
        table: &ObjectTable,
        id: i32,
        flags: InvokeFlags,
        b: &Bound,
    ) -> Result<Variant, DispError> {
        if let Some(v) = common(&self.fs, &self.abs, false, table, id, flags, b)? {
            return Ok(v);
        }
        match id {
            SIZE => Ok(Variant::Double(
                stat_of(&*self.fs, &self.abs, false)?.len as f64,
            )),
            OPEN_AS_TEXT_STREAM => {
                let mode = match opt_long(b, 0, 1)? {
                    1 => Mode::Read,
                    2 => Mode::Write,
                    8 => Mode::Append,
                    _ => return Err(err(5)),
                };
                let unicode = match opt_long(b, 1, 0)? {
                    -1 => true,
                    0 | -2 => false,
                    _ => return Err(err(5)),
                };
                let ts = TextStream::open(self.fs.clone(), &self.abs, mode, false, unicode)?;
                Ok(obj(table, ts))
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

dispatch_impl!(File);

/// A `Folder` object.
pub struct Folder {
    fs: Rc<dyn ScriptFs>,
    abs: String,
}

impl Folder {
    pub(crate) const CLASS: &'static str = "Folder";
    pub(crate) const DEFS: &'static [Def] = &[
        property("Name", NAME, &[], false),
        getter("Path", PATH, &[]),
        getter("ParentFolder", PARENT_FOLDER, &[]),
        getter("Size", SIZE, &[]),
        getter("Type", TYPE, &[]),
        getter("DateCreated", DATE_CREATED, &[]),
        getter("DateLastAccessed", DATE_LAST_ACCESSED, &[]),
        getter("DateLastModified", DATE_LAST_MODIFIED, &[]),
        property("Attributes", ATTRIBUTES, &[], false),
        getter("ShortName", SHORT_NAME, &[]),
        getter("ShortPath", SHORT_PATH, &[]),
        getter("IsRootFolder", IS_ROOT_FOLDER, &[]),
        getter("Files", FILES, &[]),
        getter("SubFolders", SUB_FOLDERS, &[]),
        method("Copy", COPY, &["Destination", "OverWriteFiles"], 1),
        method("Delete", DELETE, &["Force"], 0),
        method("Move", MOVE, &["Destination"], 1),
        method(
            "CreateTextFile",
            CREATE_TEXT_FILE,
            &["FileName", "Overwrite", "Unicode"],
            1,
        ),
    ];

    pub(crate) fn new(fs: Rc<dyn ScriptFs>, abs: String) -> Folder {
        Folder { fs, abs }
    }

    fn call(
        &self,
        table: &ObjectTable,
        id: i32,
        flags: InvokeFlags,
        b: &Bound,
    ) -> Result<Variant, DispError> {
        if let Some(v) = common(&self.fs, &self.abs, true, table, id, flags, b)? {
            return Ok(v);
        }
        match id {
            SIZE => {
                stat_of(&*self.fs, &self.abs, true)?;
                Ok(Variant::Double(folder_size(&*self.fs, &self.abs)? as f64))
            }
            IS_ROOT_FOLDER => Ok(Variant::Boolean(parent_of(&self.abs).is_none())),
            FILES | SUB_FOLDERS => {
                let dirs = id == SUB_FOLDERS;
                let names: Vec<String> = list_kind(&*self.fs, &self.abs, dirs)?
                    .into_iter()
                    .map(|e| join(&self.abs, &e.name))
                    .collect();
                Ok(obj(
                    table,
                    Collection {
                        fs: self.fs.clone(),
                        dirs,
                        paths: names,
                    },
                ))
            }
            CREATE_TEXT_FILE => {
                let name = text_arg(b, 0)?;
                let overwrite = opt_bool(b, 1, true)?;
                let unicode = opt_bool(b, 2, false)?;
                let abs = self.fs.resolve(&join(&self.abs, &name))?;
                if !overwrite && self.fs.stat(&abs)?.is_some() {
                    return Err(err(58));
                }
                let ts = TextStream::open(self.fs.clone(), &abs, Mode::Write, true, unicode)?;
                Ok(obj(table, ts))
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

dispatch_impl!(Folder);

/// The `Files` or `SubFolders` collection: a snapshot of canonical paths.
pub struct Collection {
    fs: Rc<dyn ScriptFs>,
    dirs: bool,
    paths: Vec<String>,
}

impl Collection {
    pub(crate) const CLASS: &'static str = "Files";
    pub(crate) const DEFS: &'static [Def] =
        &[getter("Count", COUNT, &[]), getter("Item", ITEM, &["Key"])];

    fn make(&self, table: &ObjectTable, path: &str) -> Variant {
        if self.dirs {
            obj(table, Folder::new(self.fs.clone(), path.to_owned()))
        } else {
            obj(table, File::new(self.fs.clone(), path.to_owned()))
        }
    }

    fn call(
        &self,
        table: &ObjectTable,
        id: i32,
        _: InvokeFlags,
        b: &Bound,
    ) -> Result<Variant, DispError> {
        match id {
            COUNT => Ok(Variant::Long(self.paths.len() as i32)),
            ITEM => {
                // **unverified**: by name (case-insensitive), as the
                // collections are keyed by file or folder name.
                let key = text_arg(b, 0)?;
                let found = self
                    .paths
                    .iter()
                    .find(|p| last_component(p).eq_ignore_ascii_case(&key));
                match found {
                    Some(p) => Ok(self.make(table, p)),
                    None => Err(DispError::from(ErrInfo::standard(5).with_source(SOURCE))),
                }
            }
            _ => Err(DispError::MEMBER_NOT_FOUND),
        }
    }

    fn new_enum(&self, table: &ObjectTable) -> Result<Variant, DispError> {
        // Each element is created when it is served, with the one reference
        // its receiver owns.
        let fs = self.fs.clone();
        let dirs = self.dirs;
        let paths = self.paths.clone();
        Ok(EnumObject::with(table, paths.len(), move |t, i| {
            let c = Collection {
                fs: fs.clone(),
                dirs,
                paths: Vec::new(),
            };
            c.make(t, &paths[i])
        }))
    }
    fn on_terminate(&self, _: &ObjectTable) {}
    fn held(&self) -> Vec<ObjectHandle> {
        Vec::new()
    }
}

dispatch_impl!(Collection);
