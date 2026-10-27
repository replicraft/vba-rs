//! Validation of descriptions: every rule has a typed error.
#![allow(missing_docs)]

use vba_host_model::{
    Access, ConstValue, HostClass, HostConst, HostEnum, HostEvent, HostGlobal, HostLibrary,
    HostMember, HostParam, ModelError, TypeRef,
};

fn errors(lib: HostLibrary) -> Vec<ModelError> {
    lib.problems()
}

fn base() -> HostLibrary {
    HostLibrary::new("L").class(HostClass::new("C").member(HostMember::sub("M")))
}

#[test]
fn a_valid_description_checks() {
    let lib = base().check().expect("valid");
    assert_eq!(lib.name(), "L");
    assert!(lib.class("c").is_some(), "lookup ignores case");
}

#[test]
fn names_must_be_identifiers() {
    let e = errors(HostLibrary::new("").class(HostClass::new("1Bad")));
    assert!(e.iter().any(|x| matches!(x, ModelError::EmptyName { .. })));
    assert!(
        e.iter()
            .any(|x| matches!(x, ModelError::InvalidName { name, .. } if name == "1Bad"))
    );
    let e = errors(base().class(HostClass::new("Has Space")));
    assert!(
        e.iter()
            .any(|x| matches!(x, ModelError::InvalidName { .. }))
    );
    let e = errors(base().class(HostClass::new("_Hidden")));
    assert!(
        e.iter()
            .any(|x| matches!(x, ModelError::InvalidName { .. }))
    );
    // Non-ASCII letters are identifiers in VBA.
    assert!(errors(HostLibrary::new("Bibliothek").class(HostClass::new("Größe"))).is_empty());
}

#[test]
fn types_may_not_take_intrinsic_names() {
    let e = errors(HostLibrary::new("L").class(HostClass::new("string")));
    assert!(
        e.iter()
            .any(|x| matches!(x, ModelError::ReservedName { .. }))
    );
    let e = errors(HostLibrary::new("L").enumeration(HostEnum::new("Long")));
    assert!(
        e.iter()
            .any(|x| matches!(x, ModelError::ReservedName { .. }))
    );
}

#[test]
fn duplicates_ignore_case() {
    // Types: a class and an enum share one namespace.
    let e = errors(
        HostLibrary::new("L")
            .class(HostClass::new("Thing"))
            .enumeration(HostEnum::new("THING")),
    );
    assert!(matches!(
        e.as_slice(),
        [ModelError::Duplicate { first, second, .. }] if first == "Thing" && second == "THING"
    ));
    // Members of a class.
    let e = errors(
        HostLibrary::new("L").class(
            HostClass::new("C")
                .member(HostMember::sub("Run"))
                .member(HostMember::property("rUN", "Long")),
        ),
    );
    assert!(matches!(e.as_slice(), [ModelError::Duplicate { .. }]));
    // Global names: constants, enum members, functions and objects share one.
    let e = errors(
        HostLibrary::new("L")
            .class(HostClass::new("App"))
            .constant(HostConst::new("Red", 1))
            .enumeration(HostEnum::new("Color").member("RED", 1))
            .function(HostMember::sub("red"))
            .global(HostGlobal::new("Red", "App")),
    );
    assert_eq!(
        e.iter()
            .filter(|x| matches!(x, ModelError::Duplicate { .. }))
            .count(),
        3
    );
    // A class and a global object of the same name are different namespaces.
    let ok = errors(
        HostLibrary::new("L")
            .class(HostClass::new("Application"))
            .global(HostGlobal::new("Application", "Application")),
    );
    assert!(ok.is_empty(), "{ok:?}");
    // Parameters, events, enum members.
    let e = errors(
        HostLibrary::new("L").class(
            HostClass::new("C")
                .member(
                    HostMember::sub("M")
                        .param(HostParam::new("a", "Long"))
                        .param(HostParam::new("A", "Long")),
                )
                .event(HostEvent::new("E"))
                .event(HostEvent::new("e")),
        ),
    );
    assert_eq!(e.len(), 2);
    let e =
        errors(HostLibrary::new("L").enumeration(HostEnum::new("E").member("A", 1).member("a", 2)));
    assert!(e.iter().any(|x| matches!(x, ModelError::Duplicate { .. })));
}

#[test]
fn dangling_type_references_are_found_everywhere() {
    let lib = HostLibrary::new("L")
        .class(
            HostClass::new("C")
                .enumerable("Missing1")
                .member(HostMember::function("F", "Missing2"))
                .member(HostMember::sub("G").param(HostParam::new("p", "Missing3()")))
                .event(HostEvent::new("E").param(HostParam::new("q", "Missing4"))),
        )
        .function(HostMember::function("Fn", "Missing5"))
        .global(HostGlobal::new("g", "Missing6"));
    let names: Vec<String> = errors(lib)
        .into_iter()
        .filter_map(|e| match e {
            ModelError::DanglingType { name, .. } => Some(name),
            _ => None,
        })
        .collect();
    for i in 1..=6 {
        assert!(names.contains(&format!("Missing{i}")), "{names:?}");
    }
}

#[test]
fn parameter_lists_follow_the_vba_rules() {
    let bad = |params: Vec<HostParam>| {
        let mut m = HostMember::sub("M");
        m.params = params;
        errors(HostLibrary::new("L").class(HostClass::new("C").member(m)))
    };
    // Required after optional.
    let e = bad(vec![
        HostParam::new("a", "Long").optional(),
        HostParam::new("b", "Long"),
    ]);
    assert!(
        matches!(e.as_slice(), [ModelError::RequiredAfterOptional { param, .. }] if param == "b")
    );
    // ParamArray not last.
    let e = bad(vec![
        HostParam::new("a", "Variant").param_array(),
        HostParam::new("b", "Long").optional(),
    ]);
    assert!(
        e.iter()
            .any(|x| matches!(x, ModelError::ParamArrayNotLast { .. }))
    );
    // ParamArray shape.
    let mut p = HostParam::new("a", "Long");
    p.param_array = true;
    let e = bad(vec![p]);
    assert!(
        e.iter()
            .any(|x| matches!(x, ModelError::ParamArrayShape { .. }))
    );
    // A default on a required parameter.
    let mut p = HostParam::new("a", "Long");
    p.default = Some(ConstValue::Long(1));
    let e = bad(vec![p]);
    assert!(matches!(
        e.as_slice(),
        [ModelError::DefaultNotOptional { .. }]
    ));
    // A default that does not fit the type.
    let e = bad(vec![HostParam::new("a", "Long").default_value("text")]);
    assert!(matches!(
        e.as_slice(),
        [ModelError::DefaultTypeMismatch { .. }]
    ));
    let e = bad(vec![HostParam::new("a", "Object").default_value(1)]);
    assert!(matches!(
        e.as_slice(),
        [ModelError::DefaultTypeMismatch { .. }]
    ));
    // Fine: an enum parameter with a numeric default, a Variant with Empty.
    let ok = HostLibrary::new("L")
        .enumeration(HostEnum::new("E").member("A", 1))
        .class(
            HostClass::new("C").member(
                HostMember::sub("M")
                    .param(HostParam::new("e", "E").default_value(1))
                    .param(HostParam::new("v", "Variant").default_value(ConstValue::Empty))
                    .param(HostParam::new("d", "Double").default_value(3)),
            ),
        );
    assert!(errors(ok).is_empty());
    // Not finite.
    let e = bad(vec![HostParam::new("a", "Double").default_value(f64::NAN)]);
    assert!(
        e.iter()
            .any(|x| matches!(x, ModelError::NonFiniteNumber { .. }))
    );
}

#[test]
fn global_functions_take_values() {
    let e =
        errors(HostLibrary::new("L").function(
            HostMember::function("F", "Long").param(HostParam::new("a", "Long").by_ref()),
        ));
    assert!(matches!(
        e.as_slice(),
        [ModelError::ByRefFunctionParam { .. }]
    ));
    let e = errors(HostLibrary::new("L").function(HostMember::property("P", "Long")));
    assert!(matches!(
        e.as_slice(),
        [ModelError::FunctionNotMethod { .. }]
    ));
}

#[test]
fn default_members_and_dispids() {
    let e = errors(
        HostLibrary::new("L").class(
            HostClass::new("C")
                .member(HostMember::property("A", "Long").default_member())
                .member(HostMember::property("B", "Long").default_member()),
        ),
    );
    assert!(matches!(
        e.as_slice(),
        [ModelError::MultipleDefaultMembers { .. }]
    ));
    let e = errors(
        HostLibrary::new("L")
            .class(HostClass::new("C").member(HostMember::sub("A").default_member())),
    );
    assert!(matches!(
        e.as_slice(),
        [ModelError::DefaultMemberIsSub { .. }]
    ));
    let e = errors(
        HostLibrary::new("L").class(
            HostClass::new("C")
                .member(HostMember::sub("A").dispid(7))
                .member(HostMember::sub("B").dispid(7)),
        ),
    );
    assert!(matches!(
        e.as_slice(),
        [ModelError::DuplicateDispId { id: 7, .. }]
    ));
    for id in [0, -4] {
        let e = errors(
            HostLibrary::new("L")
                .class(HostClass::new("C").member(HostMember::sub("A").dispid(id))),
        );
        assert!(
            matches!(e.as_slice(), [ModelError::ReservedDispId { .. }]),
            "{id}"
        );
    }
    // The default member may state its id, which must be 0.
    let ok = errors(HostLibrary::new("L").class(
        HostClass::new("C").member(HostMember::property("A", "Long").default_member().dispid(0)),
    ));
    assert!(ok.is_empty(), "{ok:?}");
    let e = errors(HostLibrary::new("L").class(
        HostClass::new("C").member(HostMember::property("A", "Long").default_member().dispid(3)),
    ));
    assert!(matches!(e.as_slice(), [ModelError::ReservedDispId { .. }]));
}

#[test]
fn automatic_ids_skip_the_fixed_ones() {
    let lib = HostLibrary::new("L")
        .class(
            HostClass::new("C")
                .member(HostMember::sub("A"))
                .member(HostMember::sub("B").dispid(2))
                .member(HostMember::sub("D"))
                .member(HostMember::property("V", "Long").default_member()),
        )
        .check()
        .unwrap();
    let ids: Vec<i32> = lib
        .class_members("C")
        .unwrap()
        .iter()
        .map(|m| m.id.0)
        .collect();
    assert_eq!(ids, [1, 2, 3, 0]);
}

#[test]
fn properties_need_a_type_and_report_every_problem() {
    let mut p = HostMember::sub("P");
    p.kind = vba_host_model::MemberKind::Property;
    let lib = HostLibrary::new("L")
        .class(HostClass::new("C").member(p).member(HostMember::sub("")))
        .class(HostClass::new("c"));
    let e = errors(lib.clone());
    assert!(
        e.iter()
            .any(|x| matches!(x, ModelError::PropertyWithoutType { .. }))
    );
    assert!(e.iter().any(|x| matches!(x, ModelError::EmptyName { .. })));
    assert!(e.iter().any(|x| matches!(x, ModelError::Duplicate { .. })));
    let err = lib.check().unwrap_err();
    assert!(
        err.to_string().contains("; "),
        "all problems in one message"
    );
    assert_eq!(err.0.len(), e.len());
}

#[test]
fn access_decides_the_invoke_kinds() {
    use vba_automation::InvokeFlags as F;
    let lib = HostLibrary::new("L")
        .class(HostClass::new("O"))
        .class(
            HostClass::new("C")
                .member(HostMember::property("Val", "Long"))
                .member(HostMember::property("Obj", "O"))
                .member(HostMember::property("Var", "Variant"))
                .member(HostMember::property("Ro", "Long").read_only())
                .member(HostMember::property("Wo", "Long").write_only())
                .member(HostMember::function("Fn", "Long")),
        )
        .check()
        .unwrap();
    let flags = |n: &str| lib.class_member("C", n).unwrap().supports;
    let read = F::METHOD | F::PROPERTY_GET;
    assert_eq!(flags("Val"), read | F::PROPERTY_PUT);
    assert_eq!(flags("Obj"), read | F::PROPERTY_PUT_REF);
    assert_eq!(flags("Var"), read | F::PROPERTY_PUT | F::PROPERTY_PUT_REF);
    assert_eq!(flags("Ro"), read);
    assert_eq!(flags("Wo"), F::PROPERTY_PUT);
    assert_eq!(flags("Fn"), read);
    let _ = Access::ReadOnly;
    let _ = TypeRef::Long;
}
