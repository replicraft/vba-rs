//! An embedding application with an in-memory table, seen from VBA.
//!
//! Run it with `cargo run -p vba-rs --example embedded_table`.
//!
//! What the host owns here: the table, the transaction and the module text.
//! What VBA sees: `Workspace.BeginTrans/CommitTrans/Rollback`, a `Customers`
//! recordset with `EOF`, `MoveNext`, `Fields` as the default member
//! (`rs!Name`) and `AddNew`, and `For Each` over a field-name collection.
//! The engine knows nothing about tables; everything goes through the
//! `Dispatch` contract. Every call has its own step, memory and time allowance.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use vba::automation::testing::MemObject;
use vba::automation::{
    DispError, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ObjectTable, ParamInfo,
};
use vba::{Arg, CallLimits, CallTarget, Engine, Error, ModuleKind, Project, Variant};

/// The host's data: rows of (name, balance), plus the open transaction.
#[derive(Default)]
struct Table {
    rows: RefCell<Vec<(String, i32)>>,
    /// A copy of `rows` taken by `BeginTrans`; restored by `Rollback`.
    snapshot: RefCell<Option<Vec<(String, i32)>>>,
    cursor: Cell<usize>,
}

const EOF: DispId = DispId(1);
const MOVE_NEXT: DispId = DispId(2);
const ADD_NEW: DispId = DispId(3);
const FIELDS: DispId = DispId::VALUE;

/// A recordset over the table: `Fields` is the default member, so `rs!Name`
/// and `rs("Balance") = 5` work.
struct Recordset(Rc<Table>);

impl Dispatch for Recordset {
    fn class_name(&self) -> String {
        "Recordset".into()
    }

    fn member_id(&self, name: &str) -> Option<DispId> {
        match name.to_ascii_lowercase().as_str() {
            "eof" => Some(EOF),
            "movenext" => Some(MOVE_NEXT),
            "addnew" => Some(ADD_NEW),
            "fields" => Some(FIELDS),
            _ => None,
        }
    }

    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        let (name, supports, params) = match id {
            EOF => ("EOF", InvokeFlags::PROPERTY_GET, vec![]),
            MOVE_NEXT => ("MoveNext", InvokeFlags::METHOD, vec![]),
            ADD_NEW => (
                "AddNew",
                InvokeFlags::METHOD,
                vec![
                    ParamInfo::required("Name").by_val(),
                    ParamInfo::optional("Balance")
                        .by_val()
                        .with_default(Variant::Long(0)),
                ],
            ),
            FIELDS => (
                "Fields",
                InvokeFlags::PROPERTY_GET | InvokeFlags::PROPERTY_PUT,
                vec![ParamInfo::required("Name").by_val()],
            ),
            _ => return None,
        };
        Some(MemberInfo::new(name, id, supports, params))
    }

    fn invoke(
        &self,
        _table: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let t = &self.0;
        let no_record = || DispError::raise(3021, "No current record.", "Customers");
        let text = |v: &Variant| String::try_from(v).map_err(|_| DispError::TYPE_MISMATCH);
        match id {
            EOF => Ok(Variant::Boolean(t.cursor.get() >= t.rows.borrow().len())),
            MOVE_NEXT => {
                t.cursor.set(t.cursor.get() + 1);
                Ok(Variant::Empty)
            }
            ADD_NEW => {
                let name = text(&params.args[0].value)?;
                let balance = params
                    .args
                    .get(1)
                    .and_then(|a| i32::try_from(&a.value).ok())
                    .unwrap_or(0);
                t.rows.borrow_mut().push((name, balance));
                Ok(Variant::Empty)
            }
            FIELDS => {
                let field = text(&params.args[0].value)?.to_ascii_lowercase();
                let mut rows = t.rows.borrow_mut();
                let row = rows.get_mut(t.cursor.get()).ok_or_else(no_record)?;
                if flags.contains(InvokeFlags::PROPERTY_PUT) {
                    let v = params
                        .put_value()
                        .cloned()
                        .ok_or(DispError::TYPE_MISMATCH)?;
                    match field.as_str() {
                        "balance" => {
                            row.1 = i32::try_from(&v).map_err(|_| DispError::TYPE_MISMATCH)?
                        }
                        _ => {
                            return Err(DispError::raise(3265, "Field is read-only.", "Customers"));
                        }
                    }
                    return Ok(Variant::Empty);
                }
                match field.as_str() {
                    "name" => Ok(Variant::from(row.0.as_str())),
                    "balance" => Ok(Variant::Long(row.1)),
                    _ => Err(DispError::raise(
                        3265,
                        "Item not found in this collection.",
                        "Customers",
                    )),
                }
            }
            _ => Err(DispError::MEMBER_NOT_FOUND),
        }
    }
}

/// `Workspace`: transactions are the host's, the VM only calls the methods.
fn workspace(t: &Rc<Table>) -> MemObject {
    let (b, c, r) = (t.clone(), t.clone(), t.clone());
    let fail = |n, what: &str| DispError::raise(n, what.to_owned(), "Workspace");
    MemObject::new("Workspace")
        .method("BeginTrans", vec![], move |_, _| {
            if b.snapshot.borrow().is_some() {
                return Err(fail(3035, "A transaction is already open."));
            }
            *b.snapshot.borrow_mut() = Some(b.rows.borrow().clone());
            Ok(Variant::Empty)
        })
        .method("CommitTrans", vec![], move |_, _| {
            c.snapshot
                .borrow_mut()
                .take()
                .map(|_| Variant::Empty)
                .ok_or_else(|| fail(3034, "No transaction to commit."))
        })
        .method("Rollback", vec![], move |_, _| {
            let saved = r.snapshot.borrow_mut().take();
            let saved = saved.ok_or_else(|| fail(3034, "No transaction to roll back."))?;
            *r.rows.borrow_mut() = saved;
            Ok(Variant::Empty)
        })
}

/// The VBA side, as the embedding application would store it.
const MODULES: [(&str, ModuleKind, &str); 1] = [(
    "Billing",
    ModuleKind::Standard,
    "\
Option Explicit

Public Function Charge(ByVal who As String, ByRef newBalance As Long) As Boolean
  On Error GoTo Fail
  Workspace.BeginTrans
  Customers.AddNew who, 100
  Customers.MoveNext
  Customers!Balance = Customers!Balance - 30
  newBalance = Customers!Balance
  Workspace.CommitTrans
  Charge = True
  Exit Function
Fail:
  Debug.Print \"charge failed: \" & Err.Number & \" \" & Err.Description & \" [\" & Err.Source & \"]\"
  Workspace.Rollback
End Function

Public Sub Report()
  Customers.MoveFirst
End Sub

Public Sub Runaway()
  Do
  Loop
End Sub
",
)];

fn main() -> Result<(), Error> {
    let table = Rc::new(Table::default());
    table.rows.borrow_mut().push(("ada".into(), 50));

    let (w, r) = (table.clone(), table.clone());
    let engine = Engine::new()
        .dispatch_global("Workspace", move || workspace(&w))
        .dispatch_global("Customers", move || Recordset(r.clone()));

    // 1. The module text comes from the host's own storage, not from a file.
    let project = Project::from_modules("Billing", MODULES)?;
    assert!(!engine.analyze(&project).has_errors());
    let mut inst = engine.load(&project)?;
    inst.trust(); // the embedding application's decision

    // 2. Each handler runs with its own allowance.
    let limits = CallLimits::new()
        .max_steps(100_000)
        .max_memory(4 * 1024 * 1024)
        .timeout(Duration::from_millis(500));

    // 3. A Public procedure by name, with a ByRef result.
    let result = inst.with_call_limits(limits, |i| {
        i.call(
            CallTarget::Project,
            "Charge",
            vec![Arg::val("bob"), Arg::by_ref(0)],
        )
    })?;
    println!(
        "charged: {:?}, new balance {:?}",
        result.value, result.byref[0]
    );
    assert_eq!(result.byref[0], Variant::Long(70));
    println!("rows: {:?}", table.rows.borrow());

    // 4. A VBA handler traps a host error and rolls back through the host.
    table.cursor.set(5); // the cursor is past the end: Balance fails with 3021
    let result = inst.with_call_limits(limits, |i| {
        i.call(
            CallTarget::Project,
            "Charge",
            vec![Arg::val("eve"), Arg::by_ref(0)],
        )
    })?;
    print!("{}", result.output);
    assert_eq!(result.value, Variant::Boolean(false));
    assert_eq!(
        table.rows.borrow().len(),
        2,
        "the rollback removed the new row"
    );
    println!("rows after rollback: {:?}", table.rows.borrow());

    // 5. An unhandled error (`Report` uses a member the object lacks, so the
    // host answers 438) reaches Rust with its number and source.
    let err = inst.run("Report", vec![]).unwrap_err();
    assert_eq!(err.kind(), "host");
    assert_eq!(err.error_info().map(|i| i.number), Some(438));
    println!("unhandled: {err}");

    // 6. A runaway handler is stopped by its allowance; the instance stays usable.
    let err = inst
        .with_call_limits(CallLimits::new().max_steps(10_000), |i| {
            i.run("Runaway", vec![])
        })
        .unwrap_err();
    assert_eq!(err.kind(), "limit");
    assert!(inst.is_usable());
    println!("runaway stopped ({}), instance still usable", err.kind());
    Ok(())
}
