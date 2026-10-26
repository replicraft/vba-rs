//! An embedding application as seen through the facade: a transaction
//! scope object, a recordset-like object with a default member, a
//! collection that `For Each` can walk, and host errors that reach
//! `Err.Number`, `Err.Description` and `Err.Source`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use vba::automation::testing::MemObject;
use vba::automation::{DispError, ParamInfo};
use vba::{Arg, CallResult, CallTarget, Engine, Error, Project, Variant};

#[derive(Default)]
struct Store {
    /// Committed rows: (name, qty).
    rows: RefCell<Vec<(String, i32)>>,
    /// Rows staged inside the open transaction, if one is open.
    staged: RefCell<Option<Vec<(String, i32)>>>,
    log: RefCell<Vec<String>>,
    cursor: Cell<usize>,
}

impl Store {
    /// The rows the next read sees: staged ones inside a transaction.
    fn visible(&self) -> Vec<(String, i32)> {
        match self.staged.borrow().as_ref() {
            Some(rows) => rows.clone(),
            None => self.rows.borrow().clone(),
        }
    }
}

fn no_transaction(what: &str) -> DispError {
    DispError::raise(3034, format!("No transaction to {what}"), "DAO.Workspace")
}

fn workspace(store: &Rc<Store>) -> MemObject {
    let (b, c, r) = (store.clone(), store.clone(), store.clone());
    MemObject::new("Workspace")
        .method("BeginTrans", vec![], move |_, _| {
            if b.staged.borrow().is_some() {
                return Err(DispError::raise(
                    3035,
                    "Transaction already open",
                    "DAO.Workspace",
                ));
            }
            *b.staged.borrow_mut() = Some(b.rows.borrow().clone());
            b.log.borrow_mut().push("begin".into());
            Ok(Variant::Empty)
        })
        .method("CommitTrans", vec![], move |_, _| {
            let Some(rows) = c.staged.borrow_mut().take() else {
                return Err(no_transaction("commit"));
            };
            *c.rows.borrow_mut() = rows;
            c.log.borrow_mut().push("commit".into());
            Ok(Variant::Empty)
        })
        .method("Rollback", vec![], move |_, _| {
            if r.staged.borrow_mut().take().is_none() {
                return Err(no_transaction("roll back"));
            }
            r.log.borrow_mut().push("rollback".into());
            Ok(Variant::Empty)
        })
}

fn table(store: &Rc<Store>) -> MemObject {
    let (add, eof, next, count, get, put) = (
        store.clone(),
        store.clone(),
        store.clone(),
        store.clone(),
        store.clone(),
        store.clone(),
    );
    let no_row = || DispError::raise(3021, "No current record.", "DAO.Recordset");
    MemObject::new("Recordset")
        .method(
            "AddRow",
            vec![
                ParamInfo::required("Name").by_val(),
                ParamInfo::required("Qty").by_val(),
            ],
            move |_, call| {
                let name = String::try_from(&call.args[0].value).unwrap();
                let qty = i32::try_from(&call.args[1].value).unwrap();
                match add.staged.borrow_mut().as_mut() {
                    Some(rows) => rows.push((name, qty)),
                    None => add.rows.borrow_mut().push((name, qty)),
                }
                Ok(Variant::Empty)
            },
        )
        .computed("EOF", move |_| {
            Variant::Boolean(eof.cursor.get() >= eof.visible().len())
        })
        .method("MoveNext", vec![], move |_, _| {
            next.cursor.set(next.cursor.get() + 1);
            Ok(Variant::Empty)
        })
        .computed("RecordCount", move |_| {
            Variant::Long(count.visible().len() as i32)
        })
        // The default member: `rs!Name` and `rs("Name")` address the current row.
        .indexed_property(
            "Fields",
            vec![ParamInfo::required("Field").by_val()],
            move |_, call| {
                let key = String::try_from(&call.args[0].value).unwrap();
                let rows = get.visible();
                let row = rows.get(get.cursor.get()).ok_or_else(no_row)?;
                match key.to_lowercase().as_str() {
                    "name" => Ok(Variant::from(row.0.as_str())),
                    "qty" => Ok(Variant::Long(row.1)),
                    _ => Err(DispError::raise(
                        3265,
                        "Item not found in this collection.",
                        "DAO.Fields",
                    )),
                }
            },
            Some(Rc::new(move |_, call| {
                let key = String::try_from(&call.args[0].value).unwrap();
                let v = call.value.clone().unwrap();
                let i = put.cursor.get();
                let mut staged = put.staged.borrow_mut();
                let mut committed = put.rows.borrow_mut();
                let rows = staged.as_mut().unwrap_or(&mut committed);
                let row = rows.get_mut(i).ok_or_else(no_row)?;
                if key.eq_ignore_ascii_case("qty") {
                    row.1 = i32::try_from(&v).map_err(|_| DispError::TYPE_MISMATCH)?;
                }
                Ok(Variant::Empty)
            })),
        )
        .default_member("Fields")
}

fn engine(store: &Rc<Store>) -> Engine {
    let (s1, s2, s3) = (store.clone(), store.clone(), store.clone());
    Engine::new()
        .dispatch_global("DBWorkspace", move || workspace(&s1))
        .dispatch_global("Orders", move || table(&s2))
        .dispatch_global("Names", move || {
            let items = s3
                .visible()
                .iter()
                .map(|r| Variant::from(r.0.as_str()))
                .collect();
            MemObject::new("Names").collection(items)
        })
}

fn run(store: &Rc<Store>, code: &str, proc: &str, args: Vec<Arg>) -> Result<CallResult, Error> {
    let project = Project::from_source("Module1", code);
    let mut inst = engine(store).load(&project).unwrap();
    inst.trust();
    inst.call(CallTarget::Project, proc, args)
}

fn store_with(rows: &[(&str, i32)]) -> Rc<Store> {
    let s = Store::default();
    *s.rows.borrow_mut() = rows.iter().map(|(n, q)| (n.to_string(), *q)).collect();
    Rc::new(s)
}

#[test]
fn a_recordset_like_object_supports_loops_and_default_members() {
    let store = store_with(&[("bolt", 10), ("nut", 20)]);
    let code = "\
Public Sub Main()
  Do While Not Orders.EOF
    Debug.Print Orders!Name; Orders(\"Qty\")
    Orders!Qty = Orders!Qty + 1
    Orders.MoveNext
  Loop
  Debug.Print Orders.RecordCount
End Sub
";
    let out = run(&store, code, "Main", vec![]).unwrap();
    assert_eq!(out.output, "bolt 10 \nnut 20 \n 2 \n");
    assert_eq!(
        *store.rows.borrow(),
        vec![("bolt".into(), 11), ("nut".into(), 21)]
    );
}

#[test]
fn for_each_walks_a_host_collection() {
    let store = store_with(&[("bolt", 10), ("nut", 20), ("washer", 5)]);
    let code = "\
Public Function Joined() As String
  Dim n As Variant, acc As String
  For Each n In Names
    acc = acc & n & \",\"
  Next
  Joined = acc
  Debug.Print Names.Count; Names(2)
End Function
";
    let out = run(&store, code, "Joined", vec![]).unwrap();
    assert_eq!(out.value, Variant::from("bolt,nut,washer,"));
    assert_eq!(out.output, " 3 nut\n");
}

#[test]
fn a_transaction_commits_through_host_methods() {
    let store = store_with(&[("bolt", 10)]);
    let code = "\
Public Sub Main()
  DBWorkspace.BeginTrans
  Orders.AddRow \"nut\", 20
  Debug.Print Orders.RecordCount
  DBWorkspace.CommitTrans
End Sub
";
    let out = run(&store, code, "Main", vec![]).unwrap();
    assert_eq!(out.output, " 2 \n");
    assert_eq!(store.rows.borrow().len(), 2);
    assert_eq!(*store.log.borrow(), ["begin", "commit"]);
}

#[test]
fn on_error_maps_host_errors_to_err_and_lets_vba_roll_back() {
    let store = store_with(&[("bolt", 10)]);
    let code = "\
Public Function Transfer() As Boolean
  On Error GoTo Fail
  DBWorkspace.BeginTrans
  Orders.AddRow \"nut\", 20
  Debug.Print Orders!NoSuchField
  DBWorkspace.CommitTrans
  Transfer = True
  Exit Function
Fail:
  Debug.Print Err.Number & \"|\" & Err.Description & \"|\" & Err.Source
  DBWorkspace.Rollback
End Function
";
    let out = run(&store, code, "Transfer", vec![]).unwrap();
    assert_eq!(out.value, Variant::Boolean(false));
    assert_eq!(
        out.output,
        "3265|Item not found in this collection.|DAO.Fields\n"
    );
    assert_eq!(
        store.rows.borrow().len(),
        1,
        "the staged row never reached the committed rows"
    );
    assert_eq!(*store.log.borrow(), ["begin", "rollback"]);
}

#[test]
fn an_unhandled_host_error_reaches_the_embedder_with_its_details() {
    let store = store_with(&[]);
    let err = run(
        &store,
        "Public Sub Main()\n  DBWorkspace.CommitTrans\nEnd Sub\n",
        "Main",
        vec![],
    )
    .unwrap_err();
    assert_eq!(err.kind(), "host");
    let Error::Run(run) = &err else {
        panic!("{err:?}")
    };
    let info = run.error_info().unwrap();
    assert_eq!(info.number, 3034);
    assert_eq!(info.description, "No transaction to commit");
    assert_eq!(info.source, "DAO.Workspace");
    assert_eq!(run.report().unwrap().frames[0].procedure, "Main");
}

#[test]
fn err_raise_resume_next_and_err_clear_round_trip_with_host_calls() {
    let store = store_with(&[]);
    let code = "\
Public Sub Main()
  On Error Resume Next
  DBWorkspace.Rollback
  Debug.Print Err.Number; Err.Source
  Err.Clear
  Debug.Print Err.Number
  Err.Raise vbObjectError + 513, \"App.Rules\", \"Quantity must be positive\"
  Debug.Print Err.Number - vbObjectError; Err.Description
End Sub
";
    let out = run(&store, code, "Main", vec![]).unwrap();
    assert_eq!(
        out.output,
        " 3034 DAO.Workspace\n 0 \n 513 Quantity must be positive\n"
    );
}

#[test]
fn a_nested_begin_is_the_hosts_error_and_state_stays_consistent() {
    let store = store_with(&[]);
    let code = "\
Public Sub Main()
  DBWorkspace.BeginTrans
  DBWorkspace.BeginTrans
End Sub
";
    let err = run(&store, code, "Main", vec![]).unwrap_err();
    let Error::Run(run) = &err else { panic!() };
    assert_eq!(run.error_info().unwrap().number, 3035);
    // The VM rolled nothing back: the host's staged work is the host's to discard.
    assert!(store.staged.borrow().is_some());
}

#[test]
fn byref_arguments_come_back_from_a_public_procedure() {
    let store = store_with(&[]);
    let code = "\
Public Sub Split2(ByVal s As String, ByRef head As String, ByRef n As Long)
  head = Left$(s, 1)
  n = Len(s)
End Sub
";
    let r = run(
        &store,
        code,
        "Split2",
        vec![Arg::val("abc"), Arg::by_ref(""), Arg::by_ref(0)],
    )
    .unwrap();
    assert_eq!(r.byref, vec![Variant::from("a"), Variant::Long(3)]);
}
