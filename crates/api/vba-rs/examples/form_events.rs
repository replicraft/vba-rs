//! A form-like host: the user edits a form, the host prepares the update,
//! raises `BeforeUpdate(Cancel)` on the form's VBA module, and then commits
//! or discards in its own storage depending on what the handler left in
//! `Cancel`.
//!
//! Run it with `cargo run -p vba-rs --example form_events`.
//!
//! What the host owns here: the record store, the pending (uncommitted)
//! changes and the transaction. The VM never opens or ends a transaction;
//! the VBA code only reads and writes the form through the host object `Me`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use vba::automation::{
    DispError, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ObjectRef, ObjectTable,
    ParamInfo,
};
use vba::{Arg, Engine, Error, ModuleKind, Project, Variant};

/// The VBA side: a form module as it would be stored in the project.
const FORM_ORDERS: &str = "\
Option Explicit

Private Sub Form_BeforeUpdate(Cancel As Integer)
    If IsNull(Me.CustomerName) Then
        MsgText = \"Customer name is required\"
        Cancel = True
    ElseIf Len(Me.CustomerName) < 3 Then
        MsgText = \"Customer name is too short: \" & Me.CustomerName
        Cancel = True
    End If
    Debug.Print \"BeforeUpdate: Cancel=\" & Cancel
End Sub

Private Sub Form_AfterUpdate()
    Debug.Print \"AfterUpdate: saved \" & Me.CustomerName
End Sub

Private Sub Form_Error(DataErr As Integer, Response As Integer)
    Debug.Print \"Form_Error \" & DataErr
End Sub
";

/// Host side: the stored row, the form's edit buffer and a log.
#[derive(Default)]
struct Database {
    /// Committed rows by key.
    rows: BTreeMap<u32, String>,
    /// Text the VBA code left for the host to show ("MsgText").
    message: String,
}

/// The `Me` of the form: a property bag over the edit buffer. Unknown names
/// are not members (error 438 for the VBA code).
struct FormObject {
    buffer: RefCell<BTreeMap<String, Variant>>,
    db: Rc<RefCell<Database>>,
}

const NAMES: [&str; 3] = ["CustomerName", "Dirty", "MsgText"];

impl Dispatch for FormObject {
    fn class_name(&self) -> String {
        "Form".into()
    }

    fn member_id(&self, name: &str) -> Option<DispId> {
        NAMES
            .iter()
            .position(|n| n.eq_ignore_ascii_case(name))
            .map(|i| DispId(i as i32 + 1))
    }

    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        let name = NAMES.get((id.0 - 1) as usize)?;
        Some(MemberInfo {
            name: (*name).into(),
            id,
            supports: InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT,
            params: Vec::<ParamInfo>::new(),
        })
    }

    fn invoke(
        &self,
        _table: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let name = NAMES
            .get((id.0 - 1) as usize)
            .ok_or(DispError::MEMBER_NOT_FOUND)?;
        if flags.is_write() {
            let v = params.put_value().cloned().unwrap_or(Variant::Empty);
            if *name == "MsgText" {
                self.db.borrow_mut().message = String::try_from(&v).unwrap_or_default();
            } else {
                self.buffer.borrow_mut().insert((*name).into(), v);
            }
            Ok(Variant::Empty)
        } else {
            Ok(self
                .buffer
                .borrow()
                .get(*name)
                .cloned()
                .unwrap_or(Variant::Null))
        }
    }
}

/// What the host does when the user leaves the record.
fn save(
    inst: &mut vba::Instance,
    form: ObjectRef,
    obj: &FormObject,
    key: u32,
) -> Result<bool, Error> {
    // 1. The host prepares the update: it starts its own transaction and
    //    stages the new value. The VM knows nothing about it.
    let staged = obj.buffer.borrow().get("CustomerName").cloned();
    println!("host: begin transaction, stage row {key}");

    // 2. Raise BeforeUpdate with a ByRef Cancel and read it back.
    let outcome = inst.raise_event_on(
        form,
        "Form",
        "BeforeUpdate",
        vec![Arg::by_ref(Variant::Integer(0))],
    )?;
    print!("{}", outcome.output);
    let cancel = outcome.byref[0] != Variant::Integer(0);

    // 3. Commit or discard in the host's storage.
    if cancel {
        println!("host: ROLLBACK ({})", obj.db.borrow().message.clone());
        Ok(false)
    } else {
        let text = staged
            .as_ref()
            .and_then(|v| String::try_from(v).ok())
            .unwrap_or_default();
        obj.db.borrow_mut().rows.insert(key, text);
        println!("host: COMMIT");
        let after = inst.raise_event_on(form, "Form", "AfterUpdate", vec![])?;
        print!("{}", after.output);
        Ok(true)
    }
}

fn main() -> Result<(), Error> {
    // Load the project: nothing runs yet, and the VBA code is untrusted.
    let mut project = Project::new();
    project
        .add_module("Form_Orders", ModuleKind::Document, FORM_ORDERS)
        .expect("a new module");
    let engine = Engine::new().host_members(["MsgText"]);
    let report = engine.analyze(&project);
    assert!(!report.has_errors(), "{:?}", report.rendered());

    let mut inst = engine.load(&project)?;
    inst.trust(); // the embedder decides the code may run

    // The form's host object is the instance's `Me`.
    let db = Rc::new(RefCell::new(Database::default()));
    let obj = Rc::new(FormObject {
        buffer: RefCell::new(BTreeMap::new()),
        db: db.clone(),
    });
    let form = inst.create_object("Form_Orders", Some(obj.clone()))?;

    // Which handlers does the form define? A host can look.
    for m in inst.modules().iter().filter(|m| m.class_like) {
        let handlers: Vec<&str> = m
            .procedures
            .iter()
            .filter(|p| p.name.starts_with("Form_"))
            .map(|p| p.name.as_str())
            .collect();
        println!("module {} defines {}", m.name, handlers.join(", "));
    }

    // The user clears the customer name and tries to save.
    println!("\n-- user leaves the field empty");
    obj.buffer
        .borrow_mut()
        .insert("Dirty".into(), Variant::Boolean(true));
    assert!(!save(&mut inst, form, &obj, 1)?);

    // Then types a name that is too short.
    println!("\n-- user types \"Al\"");
    obj.buffer
        .borrow_mut()
        .insert("CustomerName".into(), Variant::from("Al"));
    assert!(!save(&mut inst, form, &obj, 1)?);

    // Finally a valid one.
    println!("\n-- user types \"Alice\"");
    obj.buffer
        .borrow_mut()
        .insert("CustomerName".into(), Variant::from("Alice"));
    assert!(save(&mut inst, form, &obj, 1)?);
    println!("\ncommitted rows: {:?}", db.borrow().rows);

    // A VBA error inside a handler comes back with its call stack.
    println!("\n-- a handler that fails");
    let mut broken = Project::new();
    broken
        .add_module(
            "Form_Broken",
            ModuleKind::Document,
            "Private Sub Form_BeforeUpdate(Cancel As Integer)\n    Helper\nEnd Sub\nPrivate Sub Helper()\n    Dim n As Long\n    n = 1 \\ 0\nEnd Sub\n",
        )
        .expect("a new module");
    let mut inst2 = engine.load(&broken)?;
    inst2.trust();
    let form2 = inst2.create_object("Form_Broken", Some(obj.clone()))?;
    match inst2.raise_event_on(
        form2,
        "Form",
        "BeforeUpdate",
        vec![Arg::by_ref(Variant::Integer(0))],
    ) {
        Ok(_) => unreachable!("the handler divides by zero"),
        Err(Error::Run(e)) => println!("{}", e.detail()),
        Err(e) => return Err(e),
    }

    inst.destroy_object(form)?;
    inst.shutdown()?;
    Ok(())
}
