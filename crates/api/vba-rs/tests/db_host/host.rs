//! A generic stand-in for a database application that embeds vba-rs: a
//! table store with recordsets, forms and reports with controls, `DoCmd`,
//! `TempVars`, domain functions and constants. It is written only with the
//! public facade (`vba::*`, `vba::automation`) the way an adapter would be,
//! and it models the object-model vocabulary a user's VBA code uses without
//! claiming compatibility with any real product.
//!
//! Error numbers and messages below are the stand-in's own choices, marked
//! **(unverified)** where they imitate a familiar message.

#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use vba::automation::testing::MemObject;
use vba::automation::{
    ConnectionPoint, DispError, DispId, DispParams, Dispatch, EventSource, InvokeFlags, MemberInfo,
    ObjectHandle, ObjectRef, ObjectTable, ParamInfo, VariantEnum,
};
use vba::{
    Arg, CallTarget, Engine, Error, HostContext, InputBoxRequest, Instance, MsgBoxRequest, UiHooks,
    Variant,
};

pub fn txt(v: &Variant) -> String {
    match v {
        Variant::Null | Variant::Empty => String::new(),
        other => String::try_from(other).unwrap_or_default(),
    }
}

fn err(number: i32, description: &str, source: &str) -> DispError {
    DispError::raise(number, description, source)
}

// ---------------------------------------------------------------- the store

#[derive(Default, Clone)]
pub struct Table {
    pub fields: Vec<String>,
    pub rows: Vec<Vec<Variant>>,
}

/// A form or report definition: its controls and their initial values.
#[derive(Clone)]
pub struct FormDef {
    pub controls: Vec<(String, Variant)>,
    pub record_source: Option<String>,
}

#[derive(Default)]
pub struct Db {
    pub tables: RefCell<BTreeMap<String, Table>>,
    pub defs: RefCell<BTreeMap<String, FormDef>>,
    pub forms: RefCell<Vec<Rc<FormState>>>,
    pub reports: RefCell<Vec<Rc<FormState>>>,
    pub temp: RefCell<Vec<(String, Variant)>>,
    pub active: Cell<Option<ObjectRef>>,
    pub log: RefCell<Vec<String>>,
    pub messages: RefCell<Vec<String>>,
    pub answers: RefCell<Vec<i32>>,
    pub inputs: RefCell<Vec<Option<String>>>,
}

impl Db {
    pub fn new() -> Rc<Db> {
        Rc::new(Db::default())
    }

    pub fn add_table(&self, name: &str, fields: &[&str], rows: Vec<Vec<Variant>>) {
        self.tables.borrow_mut().insert(
            name.to_lowercase(),
            Table {
                fields: fields.iter().map(|f| f.to_string()).collect(),
                rows,
            },
        );
    }

    pub fn add_form(&self, name: &str, source: Option<&str>, controls: &[(&str, Variant)]) {
        self.defs.borrow_mut().insert(
            name.to_lowercase(),
            FormDef {
                controls: controls
                    .iter()
                    .map(|(n, v)| (n.to_string(), v.clone()))
                    .collect(),
                record_source: source.map(str::to_owned),
            },
        );
    }

    pub fn rows(&self, table: &str) -> Vec<Vec<Variant>> {
        self.tables.borrow()[&table.to_lowercase()].rows.clone()
    }

    pub fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    fn note(&self, what: impl Into<String>) {
        self.log.borrow_mut().push(what.into());
    }

    /// Every control name of every form: the names a form module may use
    /// unqualified.
    pub fn control_names(&self) -> Vec<String> {
        self.defs
            .borrow()
            .values()
            .flat_map(|d| d.controls.iter().map(|(n, _)| n.clone()))
            .collect()
    }
}

// ---------------------------------------------------------------- UI hooks

/// `MsgBox` / `InputBox` backed by queued answers; prompts are recorded.
pub struct TestUi(pub Rc<Db>);

impl UiHooks for TestUi {
    fn msg_box(&self, req: &MsgBoxRequest) -> vba::HostResult<i32> {
        self.0
            .messages
            .borrow_mut()
            .push(format!("{}|{}|{}", req.title, req.prompt, req.buttons));
        let mut a = self.0.answers.borrow_mut();
        Ok(if a.is_empty() { 1 } else { a.remove(0) })
    }

    fn input_box(&self, req: &InputBoxRequest) -> vba::HostResult<Option<String>> {
        self.0
            .messages
            .borrow_mut()
            .push(format!("input:{}|{}", req.prompt, req.default));
        let mut a = self.0.inputs.borrow_mut();
        Ok(if a.is_empty() { None } else { a.remove(0) })
    }
}

// --------------------------------------------------------------- recordsets

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Idle,
    AddNew,
    Edit,
}

pub struct RsInner {
    db: Rc<Db>,
    table: String,
    pos: Cell<usize>,
    mode: Cell<Mode>,
    buf: RefCell<Vec<Variant>>,
    closed: Cell<bool>,
    pub cp: ConnectionPoint,
}

impl RsInner {
    fn len(&self) -> usize {
        self.db.tables.borrow()[&self.table].rows.len()
    }

    fn eof(&self) -> bool {
        self.pos.get() >= self.len()
    }

    fn column(&self, name: &Variant) -> Result<usize, DispError> {
        let t = self.db.tables.borrow();
        let fields = &t[&self.table].fields;
        match name {
            Variant::String(_) => {
                let n = txt(name);
                fields
                    .iter()
                    .position(|f| f.eq_ignore_ascii_case(&n))
                    // (unverified) wording
                    .ok_or_else(|| err(3265, "Item not found in this collection.", "DAO.Fields"))
            }
            other => {
                let i = i32::try_from(other).map_err(|_| DispError::TYPE_MISMATCH)?;
                usize::try_from(i)
                    .ok()
                    .filter(|i| *i < fields.len())
                    .ok_or_else(|| err(3265, "Item not found in this collection.", "DAO.Fields"))
            }
        }
    }

    fn read(&self, col: usize) -> Result<Variant, DispError> {
        if self.mode.get() != Mode::Idle {
            return Ok(self.buf.borrow()[col].clone());
        }
        let t = self.db.tables.borrow();
        t[&self.table]
            .rows
            .get(self.pos.get())
            .map(|r| r[col].clone())
            .ok_or_else(|| err(3021, "No current record.", "DAO.Recordset"))
    }

    fn write(&self, col: usize, v: Variant) -> Result<(), DispError> {
        if self.mode.get() == Mode::Idle {
            // (unverified) wording
            return Err(err(
                3188,
                "Couldn't update; currently locked.",
                "DAO.Recordset",
            ));
        }
        self.buf.borrow_mut()[col] = v;
        Ok(())
    }
}

/// `Recordset`: `Fields` is the default member, so `rs!Name` and
/// `rs("Name")` work, and so does the chain through `Field.Value`.
pub fn recordset(db: &Rc<Db>, table: &str) -> MemObject {
    let rs = Rc::new(RsInner {
        db: db.clone(),
        table: table.to_lowercase(),
        pos: Cell::new(0),
        mode: Cell::new(Mode::Idle),
        buf: RefCell::default(),
        closed: Cell::new(false),
        cp: ConnectionPoint::new("RecordsetEvents"),
    });
    let ncols = db.tables.borrow()[&rs.table].fields.len();
    let (a, b, c, d, e, f, g, h, i, j, k) = (
        rs.clone(),
        rs.clone(),
        rs.clone(),
        rs.clone(),
        rs.clone(),
        rs.clone(),
        rs.clone(),
        rs.clone(),
        rs.clone(),
        rs.clone(),
        rs.clone(),
    );
    let move_to = |rs: Rc<RsInner>, f: fn(&RsInner) -> usize| {
        move |_: &ObjectTable, _: &mut vba::automation::testing::Call| {
            if rs.len() == 0 {
                return Err(err(3021, "No current record.", "DAO.Recordset"));
            }
            rs.pos.set(f(&rs));
            Ok(Variant::Empty)
        }
    };
    MemObject::new("Recordset")
        .computed("EOF", move |_| Variant::Boolean(a.eof()))
        .computed("BOF", move |_| Variant::Boolean(b.len() == 0))
        .computed("RecordCount", move |_| Variant::Long(c.len() as i32))
        .method("MoveNext", vec![], move |_, _| {
            if d.eof() {
                return Err(err(3021, "No current record.", "DAO.Recordset"));
            }
            d.pos.set(d.pos.get() + 1);
            Ok(Variant::Empty)
        })
        .method("MoveFirst", vec![], move_to(rs.clone(), |_| 0))
        .method(
            "MoveLast",
            vec![],
            move_to(rs.clone(), |r: &RsInner| r.len() - 1),
        )
        .method("AddNew", vec![], move |_, _| {
            *e.buf.borrow_mut() = vec![Variant::Null; ncols];
            e.mode.set(Mode::AddNew);
            Ok(Variant::Empty)
        })
        .method("Edit", vec![], move |_, _| {
            if f.eof() {
                return Err(err(3021, "No current record.", "DAO.Recordset"));
            }
            let row = f.db.tables.borrow()[&f.table].rows[f.pos.get()].clone();
            *f.buf.borrow_mut() = row;
            f.mode.set(Mode::Edit);
            Ok(Variant::Empty)
        })
        .method("Update", vec![], move |_, _| {
            let mode = g.mode.replace(Mode::Idle);
            let row = g.buf.borrow().clone();
            let mut t = g.db.tables.borrow_mut();
            let rows = &mut t.get_mut(&g.table).expect("table").rows;
            match mode {
                Mode::AddNew => {
                    rows.push(row);
                    g.pos.set(rows.len() - 1);
                }
                Mode::Edit => rows[g.pos.get()] = row,
                // (unverified) wording
                Mode::Idle => {
                    return Err(err(
                        3020,
                        "Update or CancelUpdate without AddNew or Edit.",
                        "DAO.Recordset",
                    ));
                }
            }
            g.db.note(format!("update {}", g.table));
            Ok(Variant::Empty)
        })
        .method("CancelUpdate", vec![], move |_, _| {
            h.mode.set(Mode::Idle);
            Ok(Variant::Empty)
        })
        .method("Close", vec![], move |_, _| {
            i.closed.set(true);
            Ok(Variant::Empty)
        })
        .indexed_property(
            "Fields",
            vec![ParamInfo::required("Index").by_val()],
            move |t, call| {
                let col = j.column(&call.args[0].value)?;
                let field = Field { rs: j.clone(), col };
                Ok(Variant::Object(ObjectRef::Handle(t.insert_obj(field))))
            },
            None,
        )
        .default_member("Fields")
        .computed("Name", move |_| Variant::from(k.table.as_str()))
}

/// `Field`: `Value` is the default member, readable and writable.
pub struct Field {
    rs: Rc<RsInner>,
    col: usize,
}

impl Dispatch for Field {
    fn class_name(&self) -> String {
        "Field".into()
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        match name.to_lowercase().as_str() {
            "value" => Some(DispId::VALUE),
            "name" => Some(DispId(2)),
            _ => None,
        }
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        let (n, f) = match id {
            DispId::VALUE => (
                "Value",
                InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT,
            ),
            DispId(2) => ("Name", InvokeFlags::PROPERTY_GET),
            _ => return None,
        };
        Some(MemberInfo::new(n, id, f, vec![]))
    }
    fn invoke(
        &self,
        _: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        if id == DispId(2) {
            let t = self.rs.db.tables.borrow();
            return Ok(Variant::from(t[&self.rs.table].fields[self.col].as_str()));
        }
        if flags.is_write() {
            let v = params.put_value().cloned().unwrap_or(Variant::Empty);
            self.rs.write(self.col, v)?;
            Ok(Variant::Empty)
        } else {
            self.rs.read(self.col)
        }
    }
}

/// `Database` (`CurrentDb`): `OpenRecordset`, `Execute`, `Name`.
pub fn database(db: &Rc<Db>) -> MemObject {
    let (d1, d2) = (db.clone(), db.clone());
    MemObject::new("Database")
        .computed("Name", |_| Variant::from("current.db"))
        .method(
            "OpenRecordset",
            vec![
                ParamInfo::required("Name").by_val(),
                ParamInfo::optional("Type").by_val(),
                ParamInfo::optional("Options").by_val(),
                ParamInfo::optional("LockEdits").by_val(),
            ],
            move |t, call| {
                let name = txt(&call.args[0].value);
                // Accept a table name or "SELECT * FROM <table>".
                let table = name
                    .to_lowercase()
                    .strip_prefix("select * from ")
                    .map(str::to_owned)
                    .unwrap_or_else(|| name.to_lowercase());
                if !d1.tables.borrow().contains_key(&table) {
                    // (unverified) wording
                    return Err(err(
                        3078,
                        &format!("Cannot find the input table or query '{name}'."),
                        "DAO.Database",
                    ));
                }
                d1.note(format!("open {table}"));
                let rs = recordset(&d1, &table);
                Ok(Variant::Object(ObjectRef::Handle(t.insert_obj(rs))))
            },
        )
        .method(
            "Execute",
            vec![
                ParamInfo::required("Query").by_val(),
                ParamInfo::optional("Options").by_val(),
            ],
            move |_, call| {
                let q = txt(&call.args[0].value);
                let lower = q.to_lowercase();
                let Some(table) = lower.strip_prefix("delete from ") else {
                    return Err(err(3131, "Syntax error in the statement.", "DAO.Database"));
                };
                match d2.tables.borrow_mut().get_mut(table.trim()) {
                    Some(t) => t.rows.clear(),
                    None => return Err(err(3078, "Cannot find the input table.", "DAO.Database")),
                }
                d2.note(format!("execute {q}"));
                Ok(Variant::Empty)
            },
        )
}

// --------------------------------------------------------- forms and reports

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Form,
    Report,
}

impl Kind {
    /// The event source name of the module-level handlers (`Form_Load`,
    /// `Report_Open`).
    pub fn source(self) -> &'static str {
        match self {
            Kind::Form => "Form",
            Kind::Report => "Report",
        }
    }
}

pub struct Ctl {
    pub name: String,
    pub value: RefCell<Variant>,
    pub visible: Cell<bool>,
    pub enabled: Cell<bool>,
    pub caption: RefCell<String>,
    pub cp: ConnectionPoint,
}

pub struct FormState {
    pub db: Rc<Db>,
    pub name: String,
    pub kind: Kind,
    pub ctls: Vec<Rc<Ctl>>,
    pub props: RefCell<BTreeMap<String, Variant>>,
    pub object: Cell<Option<ObjectRef>>,
    /// The handle of each control object, created on first use so that the
    /// same control is the same object (and `WithEvents` sees one source).
    handles: RefCell<Vec<Option<ObjectHandle>>>,
    pub rs: RefCell<Option<Variant>>,
    pub pos: Cell<usize>,
}

impl FormState {
    pub fn ctl(&self, name: &str) -> Option<(usize, &Rc<Ctl>)> {
        self.ctls
            .iter()
            .enumerate()
            .find(|(_, c)| c.name.eq_ignore_ascii_case(name))
    }

    pub fn value(&self, name: &str) -> Variant {
        self.ctl(name)
            .map(|(_, c)| c.value.borrow().clone())
            .unwrap_or(Variant::Empty)
    }

    /// The object of control `i`, created on first use. The returned handle
    /// is borrowed: the form owns the table's initial reference.
    fn control_handle(
        self: &Rc<Self>,
        table: &ObjectTable,
        i: usize,
    ) -> Result<ObjectHandle, DispError> {
        let mut handles = self.handles.borrow_mut();
        if let Some(h) = handles[i] {
            return Ok(h);
        }
        let h = table.insert_obj(Control {
            ctl: self.ctls[i].clone(),
            form: self.clone(),
        });
        handles[i] = Some(h);
        Ok(h)
    }

    pub fn module(&self) -> String {
        match self.kind {
            Kind::Form => format!("Form_{}", self.name),
            Kind::Report => format!("Report_{}", self.name),
        }
    }

    /// Copies the current record of the record source into the controls.
    fn load_record(&self, table: &str) {
        let t = self.db.tables.borrow();
        let tb = &t[&table.to_lowercase()];
        let row = tb.rows.get(self.pos.get());
        for (i, f) in tb.fields.iter().enumerate() {
            if let Some((_, c)) = self.ctl(f) {
                *c.value.borrow_mut() = row.map(|r| r[i].clone()).unwrap_or(Variant::Null);
            }
        }
    }
}

fn open_state(db: &Rc<Db>, name: &str, kind: Kind, open_args: Variant) -> Option<Rc<FormState>> {
    let def = db.defs.borrow().get(&name.to_lowercase())?.clone();
    let ctls = def
        .controls
        .iter()
        .map(|(n, v)| {
            Rc::new(Ctl {
                name: n.clone(),
                value: RefCell::new(v.clone()),
                visible: Cell::new(true),
                enabled: Cell::new(true),
                caption: RefCell::new(n.clone()),
                cp: ConnectionPoint::new("ControlEvents"),
            })
        })
        .collect::<Vec<_>>();
    let mut props = BTreeMap::new();
    props.insert("name".to_owned(), Variant::from(name));
    props.insert("caption".to_owned(), Variant::from(name));
    props.insert("openargs".to_owned(), open_args);
    props.insert("dirty".to_owned(), Variant::Boolean(false));
    props.insert(
        "recordsource".to_owned(),
        def.record_source
            .as_deref()
            .map_or(Variant::Null, Variant::from),
    );
    let n = ctls.len();
    Some(Rc::new(FormState {
        db: db.clone(),
        name: name.to_owned(),
        kind,
        ctls,
        props: RefCell::new(props),
        object: Cell::new(None),
        handles: RefCell::new(vec![None; n]),
        rs: RefCell::new(None),
        pos: Cell::new(0),
    }))
}

/// The `Me` of a form or report module. Controls are reachable as `Me.Ctl`,
/// `Me!Ctl`, `Me("Ctl")`, `Me.Controls("Ctl")` and, inside the module,
/// as bare names (the engine is told the names with `host_members`).
pub struct FormHost {
    pub st: Rc<FormState>,
}

const FIXED: [&str; 8] = [
    "Name",
    "Caption",
    "OpenArgs",
    "Dirty",
    "RecordSource",
    "Recordset",
    "Controls",
    "Requery",
];
const CTL0: i32 = 1000;

impl Dispatch for FormHost {
    fn class_name(&self) -> String {
        match self.st.kind {
            Kind::Form => "Form".into(),
            Kind::Report => "Report".into(),
        }
    }

    fn reentrant(&self) -> bool {
        true
    }

    fn member_id(&self, name: &str) -> Option<DispId> {
        if let Some(i) = FIXED.iter().position(|n| n.eq_ignore_ascii_case(name)) {
            return Some(DispId(i as i32 + 1));
        }
        if name.eq_ignore_ascii_case("Item") {
            return Some(DispId::VALUE);
        }
        self.st.ctl(name).map(|(i, _)| DispId(CTL0 + i as i32))
    }

    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        let rw = InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT;
        let (n, f, p) = match id.0 {
            i if i == DispId::VALUE.0 => (
                "Item",
                InvokeFlags::PROPERTY_GET,
                vec![ParamInfo::required("Index").by_val()],
            ),
            i if (1..=FIXED.len() as i32).contains(&i) => {
                let n = FIXED[i as usize - 1];
                match n {
                    "Controls" => (
                        n,
                        InvokeFlags::PROPERTY_GET,
                        vec![ParamInfo::optional("Index").by_val()],
                    ),
                    "Requery" => (n, InvokeFlags::METHOD, vec![]),
                    "Name" => (n, InvokeFlags::PROPERTY_GET, vec![]),
                    "Recordset" => (
                        n,
                        InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT_REF,
                        vec![],
                    ),
                    _ => (n, rw, vec![]),
                }
            }
            i if i >= CTL0 => {
                let c = self.st.ctls.get((i - CTL0) as usize)?;
                return Some(MemberInfo::new(
                    c.name.clone(),
                    id,
                    InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT,
                    vec![],
                ));
            }
            _ => return None,
        };
        Some(MemberInfo::new(n, id, f, p))
    }

    fn invoke(
        &self,
        table: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let st = &self.st;
        if id == DispId(7) && params.args.is_empty() {
            // `Me.Controls` without an index is the collection.
            let c = Collection::of_controls(st);
            return Ok(Variant::Object(ObjectRef::Handle(table.insert_obj(c))));
        }
        // Controls: by index in the id space, or by name through Item/Controls.
        let ctl_index = if id.0 >= CTL0 {
            Some((id.0 - CTL0) as usize)
        } else if id == DispId::VALUE || id == DispId(7) {
            let key = &params.args.first().ok_or(DispError::BAD_PARAM_COUNT)?.value;
            let n = txt(key);
            let found = match key {
                Variant::String(_) => st.ctl(&n).map(|(i, _)| i),
                other => i32::try_from(other)
                    .ok()
                    .and_then(|i| usize::try_from(i).ok())
                    .filter(|i| *i < st.ctls.len()),
            };
            match found {
                Some(i) => Some(i),
                // (unverified) wording
                None => {
                    return Err(err(
                        2465,
                        &format!("Can't find the field '{n}' referred to in your expression."),
                        "Form",
                    ));
                }
            }
        } else {
            None
        };
        if let Some(i) = ctl_index {
            let ctl = st.ctls.get(i).ok_or(DispError::MEMBER_NOT_FOUND)?;
            let by_id = id.0 >= CTL0;
            if by_id && flags.is_write() {
                // `Me.Ctl = v` assigns the default member (Value) of the control.
                *ctl.value.borrow_mut() = params.put_value().cloned().unwrap_or(Variant::Empty);
                return Ok(Variant::Empty);
            }
            let h = st.control_handle(table, i)?;
            // The form keeps its own reference; the caller gets one.
            table.add_ref(h)?;
            return Ok(Variant::Object(ObjectRef::Handle(h)));
        }
        let name = FIXED[id.0 as usize - 1].to_lowercase();
        match name.as_str() {
            "requery" => {
                st.db.note(format!("requery {}", st.name));
                Ok(Variant::Empty)
            }
            "recordset" => {
                if flags.contains(InvokeFlags::PROPERTY_PUT_REF) {
                    let v = params.put_value().cloned().unwrap_or(Variant::Empty);
                    table.add_ref_value(&v);
                    if let Some(old) = st.rs.replace(Some(v)) {
                        table.release_value(&old);
                    }
                    Ok(Variant::Empty)
                } else {
                    let v = st
                        .rs
                        .borrow()
                        .clone()
                        .unwrap_or(Variant::Object(ObjectRef::Nothing));
                    table.add_ref_value(&v);
                    Ok(v)
                }
            }
            _ => {
                if flags.is_write() {
                    let v = params.put_value().cloned().unwrap_or(Variant::Empty);
                    st.props.borrow_mut().insert(name, v);
                    Ok(Variant::Empty)
                } else {
                    Ok(st.props.borrow()[&name].clone())
                }
            }
        }
    }

    fn held_references(&self) -> Vec<ObjectHandle> {
        let mut v: Vec<ObjectHandle> = self.st.handles.borrow().iter().flatten().copied().collect();
        if let Some(Variant::Object(ObjectRef::Handle(h))) = self.st.rs.borrow().as_ref() {
            v.push(*h);
        }
        v
    }
}

/// A control object. `Value` is the default member; the control raises
/// `Click` to `WithEvents` sinks when VBA calls `Click` (the host would do
/// this for user input).
pub struct Control {
    ctl: Rc<Ctl>,
    form: Rc<FormState>,
}

impl Dispatch for Control {
    fn class_name(&self) -> String {
        "TextBox".into()
    }
    fn reentrant(&self) -> bool {
        true
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        match name.to_lowercase().as_str() {
            "value" | "text" => Some(DispId::VALUE),
            "visible" => Some(DispId(2)),
            "enabled" => Some(DispId(3)),
            "caption" => Some(DispId(4)),
            "name" => Some(DispId(5)),
            "setfocus" => Some(DispId(6)),
            "click" => Some(DispId(7)),
            "parent" => Some(DispId(8)),
            _ => None,
        }
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        let rw = InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT;
        let (n, f) = match id.0 {
            i if i == DispId::VALUE.0 => ("Value", rw),
            2 => ("Visible", rw),
            3 => ("Enabled", rw),
            4 => ("Caption", rw),
            5 => ("Name", InvokeFlags::PROPERTY_GET),
            6 => ("SetFocus", InvokeFlags::METHOD),
            7 => ("Click", InvokeFlags::METHOD),
            8 => ("Parent", InvokeFlags::PROPERTY_GET),
            _ => return None,
        };
        Some(MemberInfo::new(n, id, f, vec![]))
    }
    fn invoke(
        &self,
        table: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let c = &self.ctl;
        let put = || params.put_value().cloned().unwrap_or(Variant::Empty);
        match id.0 {
            i if i == DispId::VALUE.0 => {
                if flags.is_write() {
                    *c.value.borrow_mut() = put();
                    self.form
                        .props
                        .borrow_mut()
                        .insert("dirty".into(), Variant::Boolean(true));
                    Ok(Variant::Empty)
                } else {
                    Ok(c.value.borrow().clone())
                }
            }
            2 if flags.is_write() => {
                c.visible
                    .set(bool::try_from(&put()).map_err(|_| DispError::TYPE_MISMATCH)?);
                Ok(Variant::Empty)
            }
            2 => Ok(Variant::Boolean(c.visible.get())),
            3 if flags.is_write() => {
                c.enabled
                    .set(bool::try_from(&put()).map_err(|_| DispError::TYPE_MISMATCH)?);
                Ok(Variant::Empty)
            }
            3 => Ok(Variant::Boolean(c.enabled.get())),
            4 if flags.is_write() => {
                *c.caption.borrow_mut() = txt(&put());
                Ok(Variant::Empty)
            }
            4 => Ok(Variant::from(c.caption.borrow().as_str())),
            5 => Ok(Variant::from(c.name.as_str())),
            6 => {
                self.form.db.note(format!("focus {}", c.name));
                Ok(Variant::Empty)
            }
            7 => {
                c.cp.fire(table, "Click", &mut [])?;
                Ok(Variant::Empty)
            }
            8 => {
                let v = self
                    .form
                    .object
                    .get()
                    .map_or(Variant::Object(ObjectRef::Nothing), Variant::Object);
                table.add_ref_value(&v);
                Ok(v)
            }
            _ => Err(DispError::MEMBER_NOT_FOUND),
        }
    }
    fn event_source(&self) -> Option<&dyn EventSource> {
        Some(&self.ctl.cp)
    }
}

// ----------------------------------------------- collections (Forms/Reports)

/// A host collection over a live list: `Count`, `Item(name or 0-based
/// index)` as the default member (so `Forms!Orders` works) and `For Each`.
#[allow(clippy::type_complexity)]
pub struct Collection {
    class: &'static str,
    /// The live items as (name, object); the objects are borrowed, the
    /// collection adds the reference it hands out.
    list: Box<dyn Fn(&ObjectTable) -> Result<Vec<(String, Variant)>, DispError>>,
}

impl Collection {
    /// `Forms` or `Reports`.
    pub fn of_forms(db: &Rc<Db>, kind: Kind) -> Collection {
        let db = db.clone();
        Collection {
            class: if kind == Kind::Form {
                "Forms"
            } else {
                "Reports"
            },
            list: Box::new(move |_| {
                let list = match kind {
                    Kind::Form => db.forms.borrow().clone(),
                    Kind::Report => db.reports.borrow().clone(),
                };
                Ok(list
                    .iter()
                    .map(|f| {
                        let o = f
                            .object
                            .get()
                            .map_or(Variant::Object(ObjectRef::Nothing), Variant::Object);
                        (f.name.clone(), o)
                    })
                    .collect())
            }),
        }
    }

    /// `Me.Controls`.
    pub fn of_controls(st: &Rc<FormState>) -> Collection {
        let st = st.clone();
        Collection {
            class: "Controls",
            list: Box::new(move |t| {
                (0..st.ctls.len())
                    .map(|i| {
                        let h = st.control_handle(t, i)?;
                        Ok((
                            st.ctls[i].name.clone(),
                            Variant::Object(ObjectRef::Handle(h)),
                        ))
                    })
                    .collect()
            }),
        }
    }
}

struct Snapshot {
    items: Vec<Variant>,
    pos: Cell<usize>,
}

impl VariantEnum for Snapshot {
    fn next(&self, table: &ObjectTable) -> Result<Option<Variant>, DispError> {
        let p = self.pos.get();
        self.pos.set(p + 1);
        Ok(self.items.get(p).map(|v| {
            table.add_ref_value(v);
            v.clone()
        }))
    }
    fn reset(&self) -> Result<(), DispError> {
        self.pos.set(0);
        Ok(())
    }
}

impl Dispatch for Snapshot {
    fn class_name(&self) -> String {
        "IEnumVARIANT".into()
    }
    fn member_id(&self, _: &str) -> Option<DispId> {
        None
    }
    fn member_info(&self, _: DispId) -> Option<MemberInfo> {
        None
    }
    fn invoke(
        &self,
        _: &ObjectTable,
        _: DispId,
        _: InvokeFlags,
        _: &mut DispParams,
    ) -> Result<Variant, DispError> {
        Err(DispError::MEMBER_NOT_FOUND)
    }
    fn enumerator(&self) -> Option<&dyn VariantEnum> {
        Some(self)
    }
    fn held_references(&self) -> Vec<ObjectHandle> {
        Vec::new()
    }
}

impl Dispatch for Collection {
    fn class_name(&self) -> String {
        self.class.into()
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        match name.to_lowercase().as_str() {
            "count" => Some(DispId(1)),
            "item" => Some(DispId::VALUE),
            _ => None,
        }
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        match id.0 {
            1 => Some(MemberInfo::new(
                "Count",
                id,
                InvokeFlags::PROPERTY_GET,
                vec![],
            )),
            i if i == DispId::VALUE.0 => Some(MemberInfo::new(
                "Item",
                id,
                InvokeFlags::PROPERTY_GET | InvokeFlags::METHOD,
                vec![ParamInfo::required("Index").by_val()],
            )),
            _ => None,
        }
    }
    fn invoke(
        &self,
        table: &ObjectTable,
        id: DispId,
        _: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let items = (self.list)(table)?;
        match id {
            DispId(1) => Ok(Variant::Long(items.len() as i32)),
            DispId::NEWENUM => {
                let vals = items.into_iter().map(|(_, v)| v).collect();
                let e = Snapshot {
                    items: vals,
                    pos: Cell::new(0),
                };
                Ok(Variant::Object(ObjectRef::Handle(table.insert_obj(e))))
            }
            DispId::VALUE => {
                let key = &params.args.first().ok_or(DispError::BAD_PARAM_COUNT)?.value;
                let found = match key {
                    Variant::String(_) => {
                        let n = txt(key);
                        items.iter().find(|(name, _)| name.eq_ignore_ascii_case(&n))
                    }
                    other => {
                        let i = i32::try_from(other).map_err(|_| DispError::TYPE_MISMATCH)?;
                        usize::try_from(i).ok().and_then(|i| items.get(i))
                    }
                };
                match found {
                    Some((_, v)) => {
                        table.add_ref_value(v);
                        Ok(v.clone())
                    }
                    // (unverified) wording
                    None => Err(err(
                        2450,
                        &format!("Can't find the {} referred to.", self.class),
                        self.class,
                    )),
                }
            }
            _ => Err(DispError::MEMBER_NOT_FOUND),
        }
    }
}

// ------------------------------------------------------------------ DoCmd

fn bind_args(info: &[ParamInfo], params: &DispParams) -> Result<vba::automation::Bound, DispError> {
    vba::automation::bind(info, params)
}

/// Opens a form: creates the VBA instance with the host `Me`, shows it in
/// `Forms`, makes it active and raises `Open`, `Load` and `Current`.
fn open_form(
    ctx: &HostContext,
    db: &Rc<Db>,
    name: &str,
    kind: Kind,
    open_args: Variant,
) -> Result<(), DispError> {
    let list = |db: &Db| match kind {
        Kind::Form => db.forms.borrow().clone(),
        Kind::Report => db.reports.borrow().clone(),
    };
    if list(db).iter().any(|f| f.name.eq_ignore_ascii_case(name)) {
        return Ok(());
    }
    let Some(st) = open_state(db, name, kind, open_args) else {
        // (unverified) wording
        return Err(err(
            2102,
            &format!(
                "The form name '{name}' is misspelled or refers to a form that doesn't exist."
            ),
            "DoCmd",
        ));
    };
    let obj = ctx
        .create_object(&st.module(), Some(Rc::new(FormHost { st: st.clone() })))
        .map_err(DispError::from)?;
    st.object.set(Some(obj));
    let cancelled = |r: Result<vba::EventOutcome, Error>| -> Result<bool, DispError> {
        let o = r.map_err(DispError::from)?;
        Ok(o.byref.first().is_some_and(|v| *v != Variant::Integer(0)))
    };
    db.note(format!("open {} begin", st.name));
    if cancelled(ctx.raise_event_on(
        obj,
        st.kind.source(),
        "Open",
        vec![Arg::by_ref(Variant::Integer(0))],
    ))? {
        ctx.destroy_object(obj).map_err(DispError::from)?;
        db.note(format!("open {} cancelled", st.name));
        return Err(err(2501, "The OpenForm action was canceled.", "DoCmd"));
    }
    match kind {
        Kind::Form => db.forms.borrow_mut().push(st.clone()),
        Kind::Report => db.reports.borrow_mut().push(st.clone()),
    }
    if kind == Kind::Form {
        db.active.set(Some(obj));
    }
    if let Some(Variant::String(src)) = st.props.borrow().get("recordsource").cloned() {
        st.load_record(&src.to_string_lossy());
    }
    ctx.raise_event_on(obj, st.kind.source(), "Load", vec![])
        .map_err(DispError::from)?;
    ctx.raise_event_on(obj, st.kind.source(), "Current", vec![])
        .map_err(DispError::from)?;
    db.note(format!("open {} end", st.name));
    Ok(())
}

pub const AC_FORM: i32 = 2;
pub const AC_REPORT: i32 = 3;

pub fn docmd(db: &Rc<Db>) -> MemObject {
    let (d1, d2, d3, d4) = (db.clone(), db.clone(), db.clone(), db.clone());
    let ctx = |t: &ObjectTable| {
        HostContext::current(t).ok_or_else(|| err(5, "DoCmd needs a running session", "DoCmd"))
    };
    MemObject::new("DoCmd")
        .reentrant()
        .method(
            "OpenForm",
            vec![
                ParamInfo::required("FormName").by_val(),
                ParamInfo::optional("View")
                    .by_val()
                    .with_default(Variant::Long(0)),
                ParamInfo::optional("FilterName").by_val(),
                ParamInfo::optional("WhereCondition").by_val(),
                ParamInfo::optional("DataMode").by_val(),
                ParamInfo::optional("WindowMode").by_val(),
                ParamInfo::optional("OpenArgs").by_val(),
            ],
            move |t, call| {
                let args = if call.args[6].is_missing() {
                    Variant::Null
                } else {
                    call.args[6].value.clone()
                };
                open_form(&ctx(t)?, &d1, &txt(&call.args[0].value), Kind::Form, args)?;
                Ok(Variant::Empty)
            },
        )
        .method(
            "OpenReport",
            vec![
                ParamInfo::required("ReportName").by_val(),
                ParamInfo::optional("View")
                    .by_val()
                    .with_default(Variant::Long(0)),
            ],
            move |t, call| {
                open_form(
                    &ctx(t)?,
                    &d2,
                    &txt(&call.args[0].value),
                    Kind::Report,
                    Variant::Null,
                )?;
                Ok(Variant::Empty)
            },
        )
        .method(
            "Close",
            vec![
                ParamInfo::optional("ObjectType")
                    .by_val()
                    .with_default(Variant::Long(AC_FORM)),
                ParamInfo::optional("ObjectName").by_val(),
                ParamInfo::optional("Save").by_val(),
            ],
            move |t, call| {
                let kind = if call.args[0].value == Variant::Long(AC_REPORT) {
                    Kind::Report
                } else {
                    Kind::Form
                };
                let name = txt(&call.args[1].value);
                let found = {
                    let list = match kind {
                        Kind::Form => d3.forms.borrow(),
                        Kind::Report => d3.reports.borrow(),
                    };
                    list.iter()
                        .find(|f| f.name.eq_ignore_ascii_case(&name))
                        .cloned()
                };
                let Some(st) = found else {
                    // (unverified) wording
                    return Err(err(
                        2489,
                        &format!("The object '{name}' isn't open."),
                        "DoCmd",
                    ));
                };
                let c = ctx(t)?;
                let obj = st.object.get().expect("open form has an object");
                let o = c
                    .raise_event_on(
                        obj,
                        st.kind.source(),
                        "Unload",
                        vec![Arg::by_ref(Variant::Integer(0))],
                    )
                    .map_err(DispError::from)?;
                if o.byref[0] != Variant::Integer(0) {
                    return Err(err(2501, "The Close action was canceled.", "DoCmd"));
                }
                c.raise_event_on(obj, st.kind.source(), "Close", vec![])
                    .map_err(DispError::from)?;
                match kind {
                    Kind::Form => d3.forms.borrow_mut().retain(|f| !Rc::ptr_eq(f, &st)),
                    Kind::Report => d3.reports.borrow_mut().retain(|f| !Rc::ptr_eq(f, &st)),
                }
                if d3.active.get() == Some(obj) {
                    d3.active.set(None);
                }
                c.destroy_object(obj).map_err(DispError::from)?;
                d3.note(format!("closed {name}"));
                Ok(Variant::Empty)
            },
        )
        .method(
            "SetWarnings",
            vec![ParamInfo::required("WarningsOn").by_val()],
            move |_, call| {
                d4.note(format!("warnings {}", txt(&call.args[0].value)));
                Ok(Variant::Empty)
            },
        )
}

// ------------------------------------------------------------ Screen, etc.

pub fn screen(db: &Rc<Db>) -> MemObject {
    let d = db.clone();
    MemObject::new("Screen").method("ActiveForm", vec![], move |t, _| match d.active.get() {
        Some(o) => {
            let v = Variant::Object(o);
            t.add_ref_value(&v);
            Ok(v)
        }
        // (unverified) number and wording
        None => Err(err(
            2475,
            "You entered an expression that requires a form to be the active window.",
            "Screen",
        )),
    })
}

/// `TempVars`: `Add`, `Item` (default member, `TempVars!x` and
/// `TempVars("x")`), `Count`, `Remove`, `RemoveAll`.
pub fn tempvars(db: &Rc<Db>) -> MemObject {
    let (a, b, c, d, e) = (db.clone(), db.clone(), db.clone(), db.clone(), db.clone());
    MemObject::new("TempVars")
        .method(
            "Add",
            vec![
                ParamInfo::required("Name").by_val(),
                ParamInfo::required("Value").by_val(),
            ],
            move |_, call| {
                let n = txt(&call.args[0].value);
                let mut t = a.temp.borrow_mut();
                let v = call.args[1].value.clone();
                match t.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(&n)) {
                    Some(slot) => slot.1 = v,
                    None => t.push((n, v)),
                }
                Ok(Variant::Empty)
            },
        )
        .indexed_property(
            "Item",
            vec![ParamInfo::required("Index").by_val()],
            move |t, call| {
                let n = txt(&call.args[0].value);
                if !b
                    .temp
                    .borrow()
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case(&n))
                {
                    return Err(DispError::Hresult(
                        vba::raw::automation::hresult::DISP_E_BADINDEX,
                    ));
                }
                Ok(Variant::Object(ObjectRef::Handle(t.insert_obj(TempVar {
                    db: b.clone(),
                    name: n,
                }))))
            },
            None,
        )
        .default_member("Item")
        .computed(
            "Count",
            move |_| Variant::Long(c.temp.borrow().len() as i32),
        )
        .method(
            "Remove",
            vec![ParamInfo::required("Index").by_val()],
            move |_, call| {
                let n = txt(&call.args[0].value);
                d.temp
                    .borrow_mut()
                    .retain(|(k, _)| !k.eq_ignore_ascii_case(&n));
                Ok(Variant::Empty)
            },
        )
        .method("RemoveAll", vec![], move |_, _| {
            e.temp.borrow_mut().clear();
            Ok(Variant::Empty)
        })
}

/// One `TempVars` item: `Value` is the default member.
pub struct TempVar {
    db: Rc<Db>,
    name: String,
}

impl Dispatch for TempVar {
    fn class_name(&self) -> String {
        "TempVar".into()
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        match name.to_lowercase().as_str() {
            "value" => Some(DispId::VALUE),
            "name" => Some(DispId(2)),
            _ => None,
        }
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        match id.0 {
            i if i == DispId::VALUE.0 => Some(MemberInfo::new(
                "Value",
                id,
                InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT,
                vec![],
            )),
            2 => Some(MemberInfo::new(
                "Name",
                id,
                InvokeFlags::PROPERTY_GET,
                vec![],
            )),
            _ => None,
        }
    }
    fn invoke(
        &self,
        _: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        if id == DispId(2) {
            return Ok(Variant::from(self.name.as_str()));
        }
        let mut t = self.db.temp.borrow_mut();
        let slot = t
            .iter_mut()
            .find(|(k, _)| k.eq_ignore_ascii_case(&self.name))
            .ok_or(DispError::MEMBER_NOT_FOUND)?;
        if flags.is_write() {
            slot.1 = params.put_value().cloned().unwrap_or(Variant::Empty);
            Ok(Variant::Empty)
        } else {
            Ok(slot.1.clone())
        }
    }
}

/// `CurrentProject`-like data.
pub fn current_project() -> MemObject {
    MemObject::new("CurrentProject")
        .computed("Name", |_| Variant::from("Sales.proj"))
        .computed("Path", |_| Variant::from("/data"))
        .computed("FullName", |_| Variant::from("/data/Sales.proj"))
}

// --------------------------------------------------------------- Application

/// `Application`: `Run` (dynamic call of a VBA procedure by name),
/// `TryGetOption` (a `ByRef Variant` out-parameter), `Name`.
pub fn application() -> MemObject {
    MemObject::new("Application")
        .reentrant()
        .computed("Name", |_| Variant::from("StandIn"))
        .method(
            "Run",
            vec![
                ParamInfo::required("Procedure").by_val(),
                ParamInfo::param_array("Args"),
            ],
            |t, call| {
                let ctx = HostContext::current(t)
                    .ok_or_else(|| err(5, "Run needs a running session", "Application"))?;
                let args = call
                    .rest
                    .iter()
                    .map(|a| Arg::val(a.value.clone()))
                    .collect();
                let r = ctx
                    .call(CallTarget::Project, &txt(&call.args[0].value), args)
                    .map_err(DispError::from)?;
                Ok(r.value)
            },
        )
        .method(
            "TryGetOption",
            vec![
                ParamInfo::required("Name").by_val(),
                ParamInfo::required("Value"),
            ],
            |_, call| {
                let known = txt(&call.args[0].value).eq_ignore_ascii_case("Theme");
                if known {
                    call.args[1].value = Variant::from("dark");
                }
                Ok(Variant::Boolean(known))
            },
        )
}

// ------------------------------------------------------- domain functions

fn compare(a: &Variant, op: &str, b: &Variant) -> bool {
    use std::cmp::Ordering;
    let ord = match (a, b) {
        (Variant::String(_), Variant::String(_)) => {
            txt(a).to_lowercase().cmp(&txt(b).to_lowercase())
        }
        _ => match (f64::try_from(a), f64::try_from(b)) {
            (Ok(x), Ok(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
            _ => txt(a).to_lowercase().cmp(&txt(b).to_lowercase()),
        },
    };
    match op {
        "=" => ord == Ordering::Equal,
        "<>" => ord != Ordering::Equal,
        "<" => ord == Ordering::Less,
        ">" => ord == Ordering::Greater,
        "<=" => ord != Ordering::Greater,
        _ => ord != Ordering::Less,
    }
}

/// Rows of `domain` matching a single `[Field] <op> literal` criterion.
fn matching(
    db: &Db,
    domain: &str,
    criteria: &Variant,
) -> Result<(Vec<String>, Vec<Vec<Variant>>), DispError> {
    let t = db.tables.borrow();
    let Some(tb) = t.get(&domain.to_lowercase()) else {
        // (unverified) wording
        return Err(err(2001, "The domain is not a table or query.", "Domain"));
    };
    let crit = txt(criteria);
    if crit.trim().is_empty() {
        return Ok((tb.fields.clone(), tb.rows.clone()));
    }
    let ops = ["<>", "<=", ">=", "=", "<", ">"];
    let (op, pos) = ops
        .iter()
        .filter_map(|o| crit.find(o).map(|p| (*o, p)))
        .min_by_key(|(_, p)| *p)
        .ok_or_else(|| err(3075, "Syntax error in criteria.", "Domain"))?;
    let field = crit[..pos].trim().trim_matches(['[', ']']);
    let lit = crit[pos + op.len()..].trim();
    let value = if let Some(s) = lit.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        Variant::from(s)
    } else {
        Variant::Double(
            lit.parse::<f64>()
                .map_err(|_| err(3075, "Syntax error in criteria.", "Domain"))?,
        )
    };
    let col = tb
        .fields
        .iter()
        .position(|f| f.eq_ignore_ascii_case(field))
        .ok_or_else(|| err(3075, "Unknown field in criteria.", "Domain"))?;
    let rows = tb
        .rows
        .iter()
        .filter(|r| compare(&r[col], op, &value))
        .cloned()
        .collect();
    Ok((tb.fields.clone(), rows))
}

fn domain_params() -> Vec<ParamInfo> {
    vec![
        ParamInfo::required("Expr").by_val(),
        ParamInfo::required("Domain").by_val(),
        ParamInfo::optional("Criteria")
            .by_val()
            .with_default(Variant::Null),
    ]
}

/// A global function: an object with one method that is also its default
/// member, so `Name(args)` calls it.
fn function(
    name: &str,
    params: Vec<ParamInfo>,
    f: impl Fn(&ObjectTable, &mut vba::automation::testing::Call) -> Result<Variant, DispError>
    + 'static,
) -> MemObject {
    MemObject::new(name)
        .method(name, params, f)
        .default_member(name)
}

pub fn dlookup(db: &Rc<Db>) -> MemObject {
    let db = db.clone();
    function("DLookup", domain_params(), move |_, call| {
        let (fields, rows) = matching(&db, &txt(&call.args[1].value), &call.args[2].value)?;
        let want = txt(&call.args[0].value);
        let want = want.trim_matches(['[', ']']);
        let col = fields
            .iter()
            .position(|f| f.eq_ignore_ascii_case(want))
            .ok_or_else(|| err(3075, "Unknown field.", "DLookup"))?;
        Ok(rows.first().map_or(Variant::Null, |r| r[col].clone()))
    })
}

pub fn dcount(db: &Rc<Db>) -> MemObject {
    let db = db.clone();
    function("DCount", domain_params(), move |_, call| {
        let (_, rows) = matching(&db, &txt(&call.args[1].value), &call.args[2].value)?;
        Ok(Variant::Long(rows.len() as i32))
    })
}

pub fn nz() -> MemObject {
    function(
        "Nz",
        vec![
            ParamInfo::required("Value").by_val(),
            ParamInfo::optional("ValueIfNull").by_val(),
        ],
        |t, call| {
            let v = if call.args[0].value != Variant::Null {
                call.args[0].value.clone()
            } else if call.args[1].is_missing() {
                Variant::from("")
            } else {
                call.args[1].value.clone()
            };
            // A result that is an object carries one reference for the
            // caller; the argument it came from is only borrowed.
            t.add_ref_value(&v);
            Ok(v)
        },
    )
}

/// `Eval`: integer sums such as `"1+2+3"`, or a quoted string.
pub fn eval() -> MemObject {
    function(
        "Eval",
        vec![ParamInfo::required("Expr").by_val()],
        |_, call| {
            let e = txt(&call.args[0].value);
            let mut sum = 0i32;
            for part in e.split('+') {
                sum += part
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| err(2425, "Eval: unsupported expression.", "Eval"))?;
            }
            Ok(Variant::Long(sum))
        },
    )
}

/// Enum members and constants of the embedding application's libraries.
pub fn constants() -> Vec<(&'static str, i32)> {
    vec![
        ("acViewNormal", 0),
        ("acViewDesign", 1),
        ("acViewPreview", 2),
        ("acForm", AC_FORM),
        ("acReport", AC_REPORT),
        ("dbOpenTable", 1),
        ("dbOpenDynaset", 2),
        ("dbOpenSnapshot", 4),
        ("dbFailOnError", 128),
    ]
}

// ----------------------------------------------------------- the whole host

/// The engine of the stand-in: every global, constant, the UI and the names
/// the form modules may use unqualified.
pub fn engine(db: &Rc<Db>) -> Engine {
    let g = |db: &Rc<Db>| db.clone();
    let (d1, d2, d3, d4, d5, d6, d7, d8) = (g(db), g(db), g(db), g(db), g(db), g(db), g(db), g(db));
    let mut names: Vec<String> = db.control_names();
    names.extend(FIXED.iter().map(|s| s.to_string()));
    Engine::new()
        .constants(constants())
        .host_members(names)
        .ui(TestUi(db.clone()))
        .dispatch_global("Application", application)
        .dispatch_global("DoCmd", move || docmd(&d1))
        .dispatch_global("CurrentDb", move || database(&d2))
        .dispatch_global("Screen", move || screen(&d3))
        .dispatch_global("TempVars", move || tempvars(&d4))
        .dispatch_global("CurrentProject", current_project)
        .dispatch_global("Nz", nz)
        .dispatch_global("DLookup", move || dlookup(&d5))
        .dispatch_global("DCount", move || dcount(&d6))
        .dispatch_global("Eval", eval)
        .dispatch_global("Forms", move || Collection::of_forms(&d7, Kind::Form))
        .dispatch_global("Reports", move || Collection::of_forms(&d8, Kind::Report))
}

/// Loads `modules` (name, kind, source) into a trusted instance of the
/// stand-in engine.
pub fn load(db: &Rc<Db>, modules: &[(&str, vba::ModuleKind, &str)]) -> Instance {
    let mut p = vba::Project::new();
    for (n, k, s) in modules {
        p.add_module(n, *k, s).expect("module");
    }
    let mut inst = engine(db).load(&p).expect("loads");
    inst.trust();
    inst
}

/// The types the VBA code of the stand-in may name in `As` clauses:
/// opaque (members stay late bound) except that controls have a `Click`
/// event, which `WithEvents` needs to know.
pub fn libraries() -> (vba::raw::sema::StaticLibrary, vba::raw::sema::StaticLibrary) {
    use vba::raw::sema::{LibEvent, StaticLibrary};
    let dao = StaticLibrary::new("DAO")
        .with_opaque_type("Database")
        .with_opaque_type("Recordset")
        .with_opaque_type("Field");
    let access = StaticLibrary::new("Access")
        .with_opaque_type("Form")
        .with_opaque_type("Report")
        .with_opaque_type("Control")
        .with_events(
            "Control",
            vec![LibEvent {
                name: "Click".into(),
                params: vec![],
            }],
        );
    (dao, access)
}

/// Like [`load`], with the typed libraries above.
pub fn load_typed(db: &Rc<Db>, modules: &[(&str, vba::ModuleKind, &str)]) -> Instance {
    let mut p = vba::Project::new();
    for (n, k, s) in modules {
        p.add_module(n, *k, s).expect("module");
    }
    let (dao, access) = libraries();
    let mut inst = engine(db)
        .load_with(&p, |b| b.extra_provider(dao).extra_provider(access))
        .expect("loads");
    inst.trust();
    inst
}

// ------------------------------------------------- host-initiated "UI" events

/// Raises a `Cancel`-carrying event and tells whether the handler cancelled.
pub fn raise_cancelable(
    inst: &mut Instance,
    form: ObjectRef,
    source: &str,
    event: &str,
) -> Result<bool, Error> {
    let o = inst.raise_event_on(form, source, event, vec![Arg::by_ref(Variant::Integer(0))])?;
    Ok(o.byref[0] != Variant::Integer(0))
}

/// The user changes a control: `BeforeUpdate(Cancel)` of the control, then
/// the value, then `AfterUpdate`. Returns whether the change was applied.
pub fn user_edit(
    inst: &mut Instance,
    st: &Rc<FormState>,
    ctl: &str,
    value: Variant,
) -> Result<bool, Error> {
    let obj = st.object.get().expect("open");
    // The pending value is visible to the handler through the control.
    let old = st.value(ctl);
    let (_, c) = st.ctl(ctl).expect("control");
    *c.value.borrow_mut() = value;
    if raise_cancelable(inst, obj, ctl, "BeforeUpdate")? {
        *c.value.borrow_mut() = old;
        return Ok(false);
    }
    st.props
        .borrow_mut()
        .insert("dirty".into(), Variant::Boolean(true));
    inst.raise_event_on(obj, ctl, "AfterUpdate", vec![])?;
    Ok(true)
}

/// The user saves the record: form `BeforeUpdate(Cancel)`; if not
/// cancelled the controls are written to the record source and
/// `AfterUpdate` runs. The host owns this "transaction".
pub fn save_record(inst: &mut Instance, st: &Rc<FormState>) -> Result<bool, Error> {
    let obj = st.object.get().expect("open");
    if raise_cancelable(inst, obj, "Form", "BeforeUpdate")? {
        st.db.note(format!("save {} cancelled", st.name));
        return Ok(false);
    }
    if let Some(Variant::String(src)) = st.props.borrow().get("recordsource").cloned() {
        let mut t = st.db.tables.borrow_mut();
        let tb = t
            .get_mut(&src.to_string_lossy().to_lowercase())
            .expect("table");
        let fields = tb.fields.clone();
        let row = fields.iter().map(|f| st.value(f)).collect::<Vec<_>>();
        match tb.rows.get_mut(st.pos.get()) {
            Some(slot) => *slot = row,
            None => tb.rows.push(row),
        }
    }
    st.props
        .borrow_mut()
        .insert("dirty".into(), Variant::Boolean(false));
    inst.raise_event_on(obj, "Form", "AfterUpdate", vec![])?;
    st.db.note(format!("save {} committed", st.name));
    Ok(true)
}
