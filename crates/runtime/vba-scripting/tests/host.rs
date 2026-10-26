//! `ScriptingHost`: policy, precedence of the inner host's factories and the
//! native classes' direct use through the automation call helpers.

use std::rc::Rc;
use vba_automation::{CallArgs, Final, ObjectTable, Step, call};
use vba_host::{Capability, HeadlessHost, Host, Policy, ScriptedHost};
use vba_scripting::{Dictionary, ScriptingHost};
use vba_value::{ObjectRef, Variant};

fn com() -> Policy {
    Policy::deny_all().grant(Capability::ComCreate)
}

fn handle(v: &Variant) -> vba_value::ObjectHandle {
    match v {
        Variant::Object(ObjectRef::Handle(h)) => *h,
        other => panic!("{other:?}"),
    }
}

#[test]
fn creation_needs_the_com_grant() {
    let h = ScriptingHost::new(Rc::new(HeadlessHost::new()));
    assert_eq!(
        h.create_object("Scripting.Dictionary", None)
            .unwrap_err()
            .number,
        70
    );
    let h = ScriptingHost::new(Rc::new(HeadlessHost::new().with_policy(com())));
    let d = h.create_object("scripting.DICTIONARY", None).unwrap();
    assert_eq!(
        h.objects().get(handle(&d)).unwrap().class_name(),
        "Dictionary"
    );
    // Unknown classes stay "no such class".
    assert_eq!(
        h.create_object("Other.Thing", None).unwrap_err().number,
        429
    );
    // The FileSystemObject needs a file system.
    assert_eq!(
        h.create_object("Scripting.FileSystemObject", None)
            .unwrap_err()
            .number,
        429
    );
}

#[test]
fn a_factory_of_the_inner_host_wins() {
    let inner = ScriptedHost::new()
        .with_policy(com())
        .with_factory("Scripting.Dictionary", |t| {
            Ok(Variant::Object(ObjectRef::Handle(t.insert_obj(
                vba_automation::testing::MemObject::new("MyDictionary"),
            ))))
        });
    let h = ScriptingHost::new(Rc::new(inner));
    let d = h.create_object("Scripting.Dictionary", None).unwrap();
    assert_eq!(
        h.objects().get(handle(&d)).unwrap().class_name(),
        "MyDictionary"
    );
}

#[test]
fn dictionary_through_the_call_helpers() {
    let table = ObjectTable::new();
    let d = Variant::Object(ObjectRef::Handle(table.insert_obj(Dictionary::new())));
    let mut add = [Step::member(
        "Add",
        CallArgs::values([Variant::from("k"), Variant::Long(7)]),
    )];
    call::eval_chain(&table, &d, &mut add, Final::Call).unwrap();
    // Default member with the key: d("k").
    let mut get = [Step::Default(CallArgs::values([Variant::from("k")]))];
    assert_eq!(
        call::eval_chain(&table, &d, &mut get, Final::Get).unwrap(),
        Variant::Long(7)
    );
    // Count is a property.
    let mut count = [Step::member("Count", CallArgs::none())];
    assert_eq!(
        call::eval_chain(&table, &d, &mut count, Final::Get).unwrap(),
        Variant::Long(1)
    );
    // A member that does not exist is error 438.
    let mut bad = [Step::member("Nope", CallArgs::none())];
    assert_eq!(
        call::eval_chain(&table, &d, &mut bad, Final::Get)
            .unwrap_err()
            .number,
        438
    );
}
