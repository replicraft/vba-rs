//! Description -> `Dispatch` behavior.
#![allow(missing_docs)]

mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::*;
use vba_automation::call::{eval_chain, invoke_early, invoke_member};
use vba_automation::{
    Arg, CallArgs, DispError, DispId, DispParams, Dispatch, EventSink, Final, ForEach, InvokeFlags,
    ObjectTable, Step,
};
use vba_host_model::{BindError, CallKind, Handlers, HostObject};
use vba_value::{ObjectHandle, ObjectRef, Variant};

fn setup() -> (ObjectTable, ObjectHandle) {
    let table = ObjectTable::new();
    let lib = library();
    let rs = table.insert_obj(open_recordset(&lib, "Fruit"));
    (table, rs)
}

fn root(h: ObjectHandle) -> Variant {
    Variant::Object(ObjectRef::Handle(h))
}

fn hresult_of(e: &DispError) -> i32 {
    e.hresult()
}

#[test]
fn member_info_is_the_description() {
    let lib = library();
    let db = database(&lib);
    let id = db.member_id("openrecordset").expect("case-insensitive");
    let info = db.member_info(id).unwrap();
    assert_eq!(info.name, "OpenRecordset");
    assert_eq!(info.params.len(), 2);
    assert!(!info.params[0].optional);
    assert!(info.params[1].optional);
    assert_eq!(info.params[1].default, Some(Variant::Long(2)));
    assert!(!info.params[1].by_ref, "described ByVal");
    assert_eq!(
        info.supports,
        InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET
    );
    // Named parameters resolve to their index.
    let ids = db
        .get_ids_of_names(&["OpenRecordset", "kind", "NAME"])
        .unwrap();
    assert_eq!(ids[1], DispId(1));
    assert_eq!(ids[2], DispId(0));
    assert!(db.get_ids_of_names(&["OpenRecordset", "nope"]).is_err());
}

#[test]
fn ids_are_stable_and_the_default_member_is_zero() {
    let lib = library();
    let field = lib.class_member("Field", "Value").unwrap();
    assert_eq!(field.id, DispId::VALUE);
    let a = lib.class_member("Recordset", "Fields").unwrap().id;
    let b = lib.class_member("Recordset", "EOF").unwrap().id;
    assert_ne!(a, b);
    assert_eq!(library().class_member("Recordset", "Fields").unwrap().id, a);
}

#[test]
fn optional_defaults_and_named_arguments() {
    let (table, rs) = setup();
    let mut a = CallArgs::none();
    // The default NumRows is 1.
    let v = invoke_member(&table, rs, "GetRows", &mut a, &Final::Get).unwrap();
    assert_eq!(v, Variant::Long(1));
    let mut a = CallArgs::none().with_named("numrows", Arg::val(9));
    let v = invoke_member(&table, rs, "GetRows", &mut a, &Final::Get).unwrap();
    assert_eq!(v, Variant::Long(2), "limited by the two rows");
}

#[test]
fn arguments_are_converted_to_the_declared_type() {
    let (table, rs) = setup();
    // A numeric String converts to Long.
    let mut a = CallArgs::values([Variant::from("2")]);
    let v = invoke_member(&table, rs, "GetRows", &mut a, &Final::Get).unwrap();
    assert_eq!(v, Variant::Long(2));
    // A String that is not a number is a type mismatch.
    let mut a = CallArgs::values([Variant::from("many")]);
    let e = invoke_member(&table, rs, "GetRows", &mut a, &Final::Get).unwrap_err();
    assert_eq!(e.number, 13);
}

#[test]
fn too_many_and_missing_arguments() {
    let (table, rs) = setup();
    let mut a = CallArgs::values([Variant::Long(1), Variant::Long(2)]);
    let e = invoke_member(&table, rs, "GetRows", &mut a, &Final::Get).unwrap_err();
    assert_eq!(e.number, 450);
    let mut a = CallArgs::none();
    let e = invoke_member(&table, rs, "FindFirst", &mut a, &Final::Get).unwrap_err();
    assert_eq!(e.number, 449);
}

#[test]
fn an_object_of_the_wrong_class_is_a_type_mismatch() {
    let table = ObjectTable::new();
    let lib = library();
    let db = table.insert_obj(database(&lib));
    let app = table.insert_obj(application(&lib));
    let rs = table.insert_obj(open_recordset(&lib, "t"));
    let mut a = CallArgs::values([root(rs)]);
    invoke_member(&table, db, "Attach", &mut a, &Final::Call).expect("a Recordset fits");
    let mut a = CallArgs::values([Variant::Object(ObjectRef::Nothing)]);
    invoke_member(&table, db, "Attach", &mut a, &Final::Call).expect("Nothing fits");
    let mut a = CallArgs::values([root(app)]);
    let e = invoke_member(&table, db, "Attach", &mut a, &Final::Call).unwrap_err();
    assert_eq!(e.number, 13);
    let mut a = CallArgs::values([Variant::Long(1)]);
    let e = invoke_member(&table, db, "Attach", &mut a, &Final::Call).unwrap_err();
    assert_eq!(e.number, 13);
}

#[test]
fn byref_parameters_write_back_only_for_byref_variables() {
    let (table, rs) = setup();
    let mut a = CallArgs::positional(vec![Arg::val("pear"), Arg::by_ref(false)]);
    let v = invoke_member(&table, rs, "Lookup", &mut a, &Final::Get).unwrap();
    assert_eq!(v, Variant::Long(5));
    assert_eq!(
        a.positional[1].value,
        Variant::Boolean(true),
        "written back, converted to the declared Boolean"
    );

    // A by-value argument is never changed.
    let mut a = CallArgs::positional(vec![Arg::val("pear"), Arg::val(false)]);
    invoke_member(&table, rs, "Lookup", &mut a, &Final::Get).unwrap();
    assert_eq!(a.positional[1].value, Variant::Boolean(false));

    // An untouched ByRef variable keeps its own value (no converted copy).
    let mut a = CallArgs::positional(vec![Arg::by_ref("pear"), Arg::by_ref(0i16)]);
    invoke_member(&table, rs, "Lookup", &mut a, &Final::Get).unwrap();
    assert_eq!(a.positional[0].value, Variant::from("pear"));
}

#[test]
fn param_array_collects_the_rest() {
    let (table, rs) = setup();
    let mut a = CallArgs::values([
        Variant::from("-"),
        Variant::from("a"),
        Variant::Long(2),
        Variant::from("c"),
    ]);
    let v = invoke_member(&table, rs, "Join", &mut a, &Final::Get).unwrap();
    assert_eq!(v, Variant::from("a-2-c"));
}

#[test]
fn early_and_late_binding_share_one_path() {
    let (table, rs) = setup();
    let lib = library();
    let sig = lib.class_member("Recordset", "Lookup").unwrap();
    let mut late = CallArgs::positional(vec![Arg::val("apple"), Arg::by_ref(false)]);
    let a = invoke_member(&table, rs, "Lookup", &mut late, &Final::Get).unwrap();
    let mut early = CallArgs::positional(vec![Arg::val("apple"), Arg::by_ref(false)]);
    let b = invoke_early(&table, rs, sig.id, "Lookup", &mut early, &Final::Get)
        .expect("the object accepts the id")
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(late, early);
    // A wrong id for the name is refused without calling.
    let wrong = lib.class_member("Recordset", "EOF").unwrap().id;
    let mut x = CallArgs::none();
    assert!(invoke_early(&table, rs, wrong, "Lookup", &mut x, &Final::Get).is_none());
}

#[test]
fn read_only_and_unknown_members_are_refused() {
    let (table, rs) = setup();
    let mut a = CallArgs::none();
    let e = invoke_member(
        &table,
        rs,
        "RecordCount",
        &mut a,
        &Final::Let(Variant::Long(3)),
    )
    .unwrap_err();
    // No Property Let and the value is not an object: VBA error 451.
    assert_eq!(e.number, 451);
    let mut a = CallArgs::none();
    let e = invoke_member(&table, rs, "Nope", &mut a, &Final::Get).unwrap_err();
    assert_eq!(e.number, 438);
    // Property get through the Method flag combination works.
    let mut a = CallArgs::none();
    assert_eq!(
        invoke_member(&table, rs, "RecordCount", &mut a, &Final::Get).unwrap(),
        Variant::Long(2)
    );
}

#[test]
fn default_member_chaining_follows_the_description() {
    let (table, rs) = setup();
    // rs.Fields("Name").Value
    let mut steps = [
        Step::member("Fields", CallArgs::values([Variant::from("Name")])),
        Step::member("Value", CallArgs::none()),
    ];
    let v = eval_chain(&table, &root(rs), &mut steps, Final::Get).unwrap();
    assert_eq!(v, Variant::from("apple"));
    // rs.Fields("Qty") = 10 assigns through the default member of Field.
    let mut steps = [Step::member(
        "Fields",
        CallArgs::values([Variant::from("Qty")]),
    )];
    eval_chain(&table, &root(rs), &mut steps, Final::Let(Variant::Long(10))).unwrap();
    let mut steps = [
        Step::member("Fields", CallArgs::values([Variant::from("Qty")])),
        Step::member("Value", CallArgs::none()),
    ];
    let v = eval_chain(&table, &root(rs), &mut steps, Final::Get).unwrap();
    assert_eq!(v, Variant::Long(10), "the default member's value");
}

#[test]
fn for_each_uses_the_described_enumerator() {
    let (table, rs) = setup();
    let mut fields_steps = [Step::member("Fields", CallArgs::none())];
    let fields = eval_chain(&table, &root(rs), &mut fields_steps, Final::Get).unwrap();
    let mut fe = ForEach::start(&table, &fields).expect("Fields is enumerable");
    let mut names = Vec::new();
    while let Some(item) = fe.next_item().unwrap() {
        let mut s = [Step::member("Name", CallArgs::none())];
        names.push(eval_chain(&table, &item, &mut s, Final::Get).unwrap());
        table.release_value(&item);
    }
    assert_eq!(names, [Variant::from("Name"), Variant::from("Qty")]);
    // A class that is not described as enumerable has no _NewEnum.
    let mut fe = ForEach::start(&table, &root(rs));
    assert!(fe.is_err(), "Recordset is not enumerable");
    let _ = &mut fe;
}

#[derive(Default)]
struct Counter {
    seen: Cell<u32>,
}

impl EventSink for Counter {
    fn handles(&self, event: &str) -> bool {
        event.eq_ignore_ascii_case("updated")
    }
    fn on_event(&self, _: &ObjectTable, _: &str, args: &mut [Arg]) -> Result<Variant, DispError> {
        self.seen.set(self.seen.get() + 1);
        args[0].value = Variant::Integer(-1);
        Ok(Variant::Empty)
    }
}

#[test]
fn described_events_reach_sinks() {
    let (table, rs) = setup();
    let obj = table.get(rs).unwrap();
    let source = obj.event_source().expect("the class has events");
    assert_eq!(source.event_interface(), "RecordsetEvents");
    let sink = Rc::new(Counter::default());
    source.advise(sink.clone()).unwrap();
    let mut a = CallArgs::none();
    invoke_member(&table, rs, "MoveNext", &mut a, &Final::Call).unwrap();
    assert_eq!(sink.seen.get(), 1);
    // Another class has no connection point.
    let lib = library();
    assert!(database(&lib).event_source().is_none());
}

#[test]
fn an_implementation_is_checked_against_the_description() {
    let lib = library();
    let none = || Handlers::new(());
    // A described member without a handler.
    let e = HostObject::new(
        &lib,
        "Database",
        none().get("Name", |_, _| Ok(Variant::Empty)),
    )
    .err()
    .unwrap();
    assert!(
        matches!(&e, BindError::Unimplemented { member, .. } if member == "Attach" || member == "OpenRecordset"),
        "{e}"
    );
    // A handler for a member that is not described.
    let e = HostObject::new(
        &lib,
        "Database",
        none().method("Drop", |_, _| Ok(Variant::Empty)),
    )
    .err()
    .unwrap();
    assert!(matches!(e, BindError::UnknownMember { .. }), "{e}");
    // A handler for a kind of call the member does not support.
    let e = HostObject::new(&lib, "Database", none().let_("Name", |_, _| Ok(())))
        .err()
        .unwrap();
    assert!(
        matches!(
            e,
            BindError::KindNotSupported {
                kind: CallKind::Let,
                ..
            }
        ),
        "{e}"
    );
    // An unknown class.
    let e = HostObject::new(&lib, "Nope", none()).err().unwrap();
    assert!(matches!(e, BindError::UnknownClass(_)));
}

#[test]
fn a_result_of_the_wrong_type_is_caught() {
    let lib = library();
    let bad = HostObject::new(
        &lib,
        "Application",
        Handlers::new(())
            .get("Name", |_, _| Ok(Variant::from("x")))
            // Declared As Database but returns a String.
            .method("CurrentDb", |_, _| Ok(Variant::from("oops"))),
    )
    .unwrap();
    let table = ObjectTable::new();
    let h = table.insert_obj(bad);
    let mut a = CallArgs::none();
    let e = invoke_member(&table, h, "CurrentDb", &mut a, &Final::Get).unwrap_err();
    assert_eq!(e.number, 13);
}

#[test]
fn global_functions_bind_like_members() {
    let rt = runtime();
    let table = ObjectTable::new();
    let f = rt.functions();
    // Nz(Null, "n") returns the replacement; Nz(5) returns 5.
    let v = f
        .call(
            &table,
            "nz",
            &[Some(Variant::Null), Some(Variant::from("n"))],
        )
        .unwrap()
        .unwrap();
    assert_eq!(v, Variant::from("n"));
    let v = f
        .call(&table, "Nz", &[Some(Variant::Long(5)), None])
        .unwrap()
        .unwrap();
    assert_eq!(v, Variant::Long(5));
    // A required argument omitted.
    let e = f
        .call(&table, "DLookup", &[None, Some("t".into())])
        .unwrap_err();
    assert_eq!(hresult_of(&e), DispError::PARAM_NOT_OPTIONAL.hresult());
    // Too many arguments.
    let args: Vec<Option<Variant>> = (0..4).map(|i| Some(Variant::Long(i))).collect();
    let e = f.call(&table, "DLookup", &args).unwrap_err();
    assert_eq!(hresult_of(&e), DispError::BAD_PARAM_COUNT.hresult());
    // Arguments are converted: a Long Expr becomes a String.
    let v = f
        .call(
            &table,
            "DLookup",
            &[Some(Variant::Long(7)), Some("t".into()), None],
        )
        .unwrap()
        .unwrap();
    assert_eq!(v, Variant::from("7!"));
    // Not a function of the library.
    assert!(f.call(&table, "Nope", &[]).unwrap().is_none());
}

#[test]
fn the_runtime_reports_what_is_missing() {
    let lib = library();
    let missing = vba_host_model::HostRuntime::new(&lib)
        .function("Nz", |_| Ok(Variant::Null))
        .function("Bogus", |_| Ok(Variant::Null))
        .build()
        .err()
        .unwrap();
    assert!(missing.contains(&BindError::UnknownFunction("Bogus".into())));
    assert!(missing.contains(&BindError::NoFactory("Application".into())));
    assert!(missing.contains(&BindError::NoFactory("Database".into())));
    assert!(
        missing
            .iter()
            .any(|e| matches!(e, BindError::Unimplemented { member, .. } if member == "DLookup"))
    );
}

#[test]
fn factories_make_objects_in_a_table() {
    let rt = runtime();
    let table = ObjectTable::new();
    let globals = rt.globals();
    let (name, make) = &globals[0];
    assert_eq!(name, "Application");
    let v = make(&table).unwrap();
    let mut s = [Step::member("Name", CallArgs::none())];
    assert_eq!(
        eval_chain(&table, &v, &mut s, Final::Get).unwrap(),
        Variant::from("TestApp")
    );
    let classes = rt.classes();
    assert_eq!(classes[0].0, "Database");
}

#[test]
fn invoke_flags_decide_the_kind_of_call() {
    let lib = library();
    let obj = application(&lib);
    let table = ObjectTable::new();
    let id = obj.member_id("Name").unwrap();
    let mut p = DispParams::new();
    // A write to a read-only property.
    p.named.push((DispId::PROPERTYPUT, Arg::val("x")));
    let e = obj
        .invoke(&table, id, InvokeFlags::PROPERTY_PUT, &mut p)
        .unwrap_err();
    assert_eq!(e, DispError::MEMBER_NOT_FOUND);
}
