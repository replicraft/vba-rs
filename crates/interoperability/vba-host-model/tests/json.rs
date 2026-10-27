//! The JSON form: round trip, strictness, stability.
#![allow(missing_docs)]

mod common;

use serde_json::json;
use vba_host_model::{HostLibrary, JsonError, TypeRef};

#[test]
fn the_sample_round_trips() {
    let lib = common::description();
    let text = lib.to_json_string();
    let back = HostLibrary::from_json_str(&text).expect("reads what it wrote");
    assert_eq!(back, lib);
    // And the text is stable: writing the copy gives the same text.
    assert_eq!(back.to_json_string(), text);
    back.check().expect("still valid");
}

#[test]
fn the_form_is_the_documented_one() {
    let lib = vba_host_model::HostLibrary::new("L")
        .doc("d")
        .class(
            vba_host_model::HostClass::new("C")
                .creatable()
                .enumerable("C")
                .member(
                    vba_host_model::HostMember::function("Item", "C")
                        .param(vba_host_model::HostParam::new("i", "Long").default_value(1))
                        .default_member(),
                ),
        )
        .enumeration(vba_host_model::HostEnum::new("E").member("A", 1))
        .constant(vba_host_model::HostConst::new("K", 2.5))
        .global(vba_host_model::HostGlobal::new("g", "C"));
    assert_eq!(
        lib.to_json(),
        json!({
            "format": "vba-host-library",
            "version": 1,
            "name": "L",
            "doc": "d",
            "classes": [{
                "name": "C",
                "creatable": true,
                "enumerable": "C",
                "members": [{
                    "name": "Item",
                    "kind": "method",
                    "type": "C",
                    "params": [{
                        "name": "i",
                        "type": "Long",
                        "optional": true,
                        "default": {"type": "long", "value": 1}
                    }],
                    "default": true
                }]
            }],
            "enums": [{"name": "E", "members": [{"name": "A", "value": 1}]}],
            "constants": [{"name": "K", "value": {"type": "double", "value": 2.5}}],
            "functions": [],
            "globals": [{"name": "g", "type": "C"}]
        })
    );
}

fn minimal() -> serde_json::Value {
    json!({"format": "vba-host-library", "version": 1, "name": "L",
           "classes": [], "enums": [], "constants": [], "functions": [], "globals": []})
}

#[test]
fn the_reader_is_strict() {
    assert!(HostLibrary::from_json(&minimal()).is_ok());

    let mut v = minimal();
    v["version"] = json!(2);
    assert_eq!(HostLibrary::from_json(&v), Err(JsonError::Version(2)));

    let mut v = minimal();
    v["format"] = json!("something-else");
    assert!(
        matches!(HostLibrary::from_json(&v), Err(JsonError::Shape { path, .. }) if path == "$.format")
    );

    let mut v = minimal();
    v["surprise"] = json!(1);
    assert!(
        matches!(HostLibrary::from_json(&v), Err(JsonError::Shape { path, .. }) if path == "$.surprise")
    );

    let mut v = minimal();
    v["classes"] = json!([{"name": "C", "members": [{"name": "M", "kind": "sub"}]}]);
    match HostLibrary::from_json(&v) {
        Err(JsonError::Shape { path, message }) => {
            assert_eq!(path, "$.classes[0].members[0].kind");
            assert!(message.contains("method"), "{message}");
        }
        other => panic!("{other:?}"),
    }

    let mut v = minimal();
    v["classes"] = json!([{"name": "C", "members": [
        {"name": "M", "kind": "method", "params": [{"name": "p", "type": 3}]}]}]);
    assert!(matches!(
        HostLibrary::from_json(&v),
        Err(JsonError::Shape { path, .. }) if path == "$.classes[0].members[0].params[0].type"
    ));

    let mut v = minimal();
    v["constants"] = json!([{"name": "K", "value": {"type": "long", "value": 3000000000u64}}]);
    assert!(matches!(
        HostLibrary::from_json(&v),
        Err(JsonError::Shape { .. })
    ));

    let mut v = minimal();
    v.as_object_mut().unwrap().remove("name");
    assert!(matches!(
        HostLibrary::from_json(&v),
        Err(JsonError::Shape { .. })
    ));

    assert!(matches!(
        HostLibrary::from_json_str("{"),
        Err(JsonError::Syntax(_))
    ));
    assert!(matches!(
        HostLibrary::from_json_str("[]"),
        Err(JsonError::Shape { .. })
    ));
}

#[test]
fn reading_does_not_validate() {
    let mut v = minimal();
    v["classes"] = json!([{"name": "C", "members": [
        {"name": "M", "kind": "method", "type": "Nowhere"}]}]);
    let lib = HostLibrary::from_json(&v).unwrap();
    assert_eq!(lib.problems().len(), 1);
    assert_eq!(
        lib.classes[0].members[0].ty,
        Some(TypeRef::Named("Nowhere".into()))
    );
}

#[test]
fn type_spellings_round_trip() {
    for t in [
        "Long",
        "string",
        "Variant()",
        "Thing",
        "Thing()",
        "Object",
        "LongLong",
    ] {
        let r = TypeRef::parse(t);
        assert_eq!(TypeRef::parse(&r.to_string()), r, "{t}");
    }
    assert_eq!(TypeRef::parse("long").to_string(), "Long");
}
