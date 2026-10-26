//! A host base object for document-like modules in tests: it accepts every
//! member name, journals what is done to it and remembers what was
//! assigned. Reading a member whose name looks like a control (or one of a
//! few well-known application objects) yields a child object that journals
//! under a dotted path; any other member reads as `<name>` unless a value
//! was assigned to it.

#![allow(dead_code, missing_docs)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vba_automation::{
    DispError, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ObjectTable,
};
use vba_value::{ObjectRef, Variant};

pub struct HostMock {
    journal: Rc<RefCell<Vec<String>>>,
    path: String,
    names: RefCell<Vec<String>>,
    /// Assigned values by full path, shared by the whole tree.
    values: Rc<RefCell<HashMap<String, Variant>>>,
}

impl HostMock {
    pub fn new(journal: Rc<RefCell<Vec<String>>>) -> Self {
        Self::at(journal, String::new(), Rc::default())
    }

    fn at(
        journal: Rc<RefCell<Vec<String>>>,
        path: String,
        values: Rc<RefCell<HashMap<String, Variant>>>,
    ) -> Self {
        Self {
            journal,
            path,
            names: RefCell::new(Vec::new()),
            values,
        }
    }
}

fn is_object_like(name: &str) -> bool {
    const PREFIXES: [&str; 14] = [
        "lbl", "txt", "cmd", "cbo", "lst", "tab", "chk", "fra", "sub", "pge", "ctl", "img", "opt",
        "frm",
    ];
    const NAMES: [&str; 6] = [
        "docmd",
        "currentdb",
        "application",
        "screen",
        "forms",
        "reports",
    ];
    PREFIXES.iter().any(|p| name.starts_with(p)) || NAMES.contains(&name)
}

impl Dispatch for HostMock {
    fn class_name(&self) -> String {
        "HostBase".into()
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        let key = name.to_ascii_lowercase();
        let mut names = self.names.borrow_mut();
        let i = match names.iter().position(|n| *n == key) {
            Some(i) => i,
            None => {
                names.push(key);
                names.len() - 1
            }
        };
        Some(DispId(i as i32 + 1))
    }
    fn member_info(&self, _id: DispId) -> Option<MemberInfo> {
        None
    }
    fn invoke(
        &self,
        table: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let name = match usize::try_from(id.0 - 1) {
            Ok(i) => self.names.borrow()[i].clone(),
            Err(_) => "(default)".to_owned(),
        };
        let args: Vec<&Variant> = params.args.iter().map(|a| &a.value).collect();
        if flags.is_write() {
            let v = params.put_value().cloned().unwrap_or(Variant::Empty);
            self.journal
                .borrow_mut()
                .push(format!("put {}{name}{args:?} = {v:?}", self.path));
            self.values
                .borrow_mut()
                .insert(format!("{}{name}", self.path), v);
            return Ok(Variant::Empty);
        }
        self.journal
            .borrow_mut()
            .push(format!("get {}{name}{args:?}", self.path));
        if let Some(v) = self.values.borrow().get(&format!("{}{name}", self.path)) {
            return Ok(v.clone());
        }
        if is_object_like(&name) {
            let child = HostMock::at(
                self.journal.clone(),
                format!("{}{name}.", self.path),
                self.values.clone(),
            );
            let h = table.insert_obj(child);
            return Ok(Variant::Object(ObjectRef::Handle(h)));
        }
        Ok(Variant::String(format!("<{name}>").as_str().into()))
    }
}
