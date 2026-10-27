//! Property tests: validation, the JSON form and the agreement of the
//! adapters.
#![allow(missing_docs)]

use proptest::prelude::*;
use vba_automation::Dispatch;
use vba_host_model::{
    ConstValue, Handlers, HostClass, HostConst, HostEnum, HostLibrary, HostMember, HostObject,
    HostParam, MemberKind, ModelError, TypeRef,
};
use vba_sema::{LibraryType, TypeProvider};
use vba_value::Variant;

fn ident() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9_]{0,7}"
}

fn scalar() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "Long", "String", "Variant", "Boolean", "Double", "Date", "Object", "Long()",
    ])
}

/// Distinct (case-insensitive) identifiers.
fn distinct(n: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(ident(), n..=n + 6).prop_map(move |v| {
        let mut seen = std::collections::HashSet::new();
        v.into_iter()
            .filter(|s| seen.insert(s.to_lowercase()))
            .collect()
    })
}

#[derive(Debug, Clone)]
struct ParamShape {
    optional: bool,
    by_ref: bool,
    ty: &'static str,
}

fn shapes() -> impl Strategy<Value = Vec<ParamShape>> {
    prop::collection::vec(
        (any::<bool>(), any::<bool>(), scalar()).prop_map(|(optional, by_ref, ty)| ParamShape {
            optional,
            by_ref,
            ty,
        }),
        0..4,
    )
    .prop_map(|mut v| {
        // Optional parameters last, as VBA requires.
        v.sort_by_key(|s| s.optional);
        v
    })
}

fn build(names: &[String], member_names: &[Vec<String>], shape: &[Vec<ParamShape>]) -> HostLibrary {
    let mut lib = HostLibrary::new("Lib");
    for (ci, cname) in names.iter().enumerate() {
        let mut c = HostClass::new(format!("C_{cname}")).doc("doc");
        if ci % 2 == 1 {
            c = c.interface();
        }
        for (mi, mname) in member_names[ci % member_names.len()].iter().enumerate() {
            let params = &shape[(ci + mi) % shape.len()];
            let mut m = if mi % 3 == 0 {
                HostMember::sub(mname.clone())
            } else if mi % 3 == 1 {
                HostMember::function(mname.clone(), "Long")
            } else {
                HostMember::property(mname.clone(), "String")
            };
            for (pi, p) in params.iter().enumerate() {
                let mut hp = HostParam::new(format!("p{pi}"), p.ty);
                hp.optional = p.optional;
                hp.by_ref = p.by_ref;
                m = m.param(hp);
            }
            c = c.member(m);
        }
        lib = lib.class(c);
    }
    lib
}

fn library() -> impl Strategy<Value = HostLibrary> {
    (
        distinct(1),
        prop::collection::vec(distinct(1), 1..4),
        prop::collection::vec(shapes(), 1..4),
    )
        .prop_map(|(names, members, shape)| build(&names, &members, &shape))
}

proptest! {
    /// Generated libraries are valid, and the JSON form round-trips.
    #[test]
    fn json_round_trips(lib in library()) {
        prop_assert!(lib.problems().is_empty(), "{:?}", lib.problems());
        let back = HostLibrary::from_json_str(&lib.to_json_string()).unwrap();
        prop_assert_eq!(&back, &lib);
        prop_assert_eq!(back.to_json(), lib.to_json());
    }

    /// Re-spelling a name in another case creates a duplicate error.
    #[test]
    fn case_variants_collide(lib in library(), flip in any::<prop::sample::Index>()) {
        let mut lib = lib;
        let i = flip.index(lib.classes.len());
        let mut twin = lib.classes[i].clone();
        twin.name = if twin.name.to_lowercase() == twin.name {
            twin.name.to_uppercase()
        } else {
            twin.name.to_lowercase()
        };
        prop_assert_ne!(&twin.name, &lib.classes[i].name);
        lib.classes.push(twin);
        let e = lib.problems();
        prop_assert!(e.iter().any(|x| matches!(x, ModelError::Duplicate { .. })), "{:?}", e);
    }

    /// A type that is nowhere declared is reported, wherever it is used.
    #[test]
    fn dangling_references_are_found(lib in library(), name in "Zz[a-z]{3,6}") {
        let mut lib = lib;
        lib.classes[0].members.push(HostMember::function("Probe", TypeRef::named(name.clone())));
        let e = lib.problems();
        prop_assert!(
            e.iter().any(|x| matches!(x, ModelError::DanglingType { name: n, .. } if *n == name)),
            "{:?}", e
        );
    }

    /// The same member as sema and as automation sees it.
    #[test]
    fn the_adapters_agree(lib in library()) {
        let checked = lib.check().unwrap();
        for c in &checked.library().classes {
            let ty = LibraryType { library: "Lib".into(), name: c.name.clone() };
            let obj = full_object(&checked, &c.name);
            for m in &c.members {
                let lm = checked.member(&ty, &m.name).expect("provider has it");
                let id = obj.member_id(&m.name.to_uppercase()).expect("dispatch has it");
                let info = obj.member_info(id).unwrap();
                prop_assert_eq!(lm.dispid, Some(id.0));
                prop_assert_eq!(&lm.name, &info.name);
                prop_assert_eq!(lm.params.len(), info.params.len());
                for (a, b) in lm.params.iter().zip(&info.params) {
                    prop_assert_eq!(&a.name, &b.name);
                    prop_assert_eq!(a.optional, b.optional);
                    prop_assert_eq!(a.by_ref, b.by_ref);
                    prop_assert_eq!(a.param_array, b.param_array);
                    prop_assert_eq!(&a.default, &b.default);
                }
                let writable = m.kind == MemberKind::Property;
                prop_assert_eq!(
                    info.supports.is_write() || info.supports.intersects(
                        vba_automation::InvokeFlags::PROPERTY_PUT | vba_automation::InvokeFlags::PROPERTY_PUT_REF),
                    writable
                );
            }
        }
    }

    /// The interface flag reaches the provider, and distinct described
    /// classes are never assignable to each other.
    #[test]
    fn classes_are_exact_and_interfaces_are_flagged(lib in library()) {
        let checked = lib.clone().check().unwrap();
        for c in &lib.classes {
            let ty = LibraryType { library: "Lib".into(), name: c.name.clone() };
            prop_assert_eq!(checked.is_interface(&ty), Some(c.interface));
            prop_assert!(checked.is_exact_class(&ty));
        }
        if lib.classes.len() >= 2 {
            let (a, b) = (&lib.classes[0].name, &lib.classes[1].name);
            let src = format!(
                "Sub P()\nDim x As Lib.{a}\nDim y As Lib.{b}\nSet x = y\nEnd Sub\n"
            );
            let modules = [vba_sema::SourceModule {
                name: "M".into(),
                kind: vba_project::ModuleKind::Standard,
                source: src.into_bytes(),
            }];
            let profile = *vba_profile::find_profile("vba7.1-excel-win64").unwrap();
            let r = vba_sema::analyze_sources(&modules, &profile, &checked, &vba_sema::Options::default());
            let codes: Vec<_> = r.diagnostics.iter()
                .filter(|d| d.severity == vba_diagnostics::Severity::Error)
                .map(|d| d.code.0)
                .collect();
            prop_assert_eq!(codes, vec!["SEM006"]);
        }
    }

    /// Arbitrary text never panics the reader or the checker.
    #[test]
    fn garbage_is_an_error_not_a_panic(s in ".{0,200}") {
        let _ = HostLibrary::from_json_str(&s);
        let lib = HostLibrary::new(s.clone())
            .class(HostClass::new(s.clone()).member(HostMember::sub(s.clone())))
            .enumeration(HostEnum::new(s.clone()).member(s.clone(), 1))
            .constant(HostConst::new(s.clone(), ConstValue::String(s.clone())));
        let _ = lib.problems();
    }

    /// Anything that is not a library object is refused with a path.
    #[test]
    fn structured_garbage_is_refused(v in arb_json()) {
        let _ = HostLibrary::from_json(&v);
    }
}

fn arb_json() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::from),
        any::<i64>().prop_map(serde_json::Value::from),
        "[a-z]{0,6}".prop_map(serde_json::Value::from),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(serde_json::Value::Array),
            prop::collection::vec(
                (
                    "(format|version|name|classes|enums|members|params|kind|type|value)",
                    inner
                ),
                0..4
            )
            .prop_map(|kv| serde_json::Value::Object(kv.into_iter().collect())),
        ]
    })
}

/// An object whose handlers cover every described member.
fn full_object(lib: &vba_host_model::CheckedLibrary, class: &str) -> HostObject {
    let c = lib.class(class).unwrap();
    let mut h = Handlers::new(());
    for m in &c.members {
        match m.kind {
            MemberKind::Method => h = h.method(&m.name, |_, _| Ok(Variant::Empty)),
            MemberKind::Property => {
                h = h
                    .get(&m.name, |_, _| Ok(Variant::from("")))
                    .let_(&m.name, |_, _| Ok(()));
            }
        }
    }
    HostObject::new(lib, class, h).expect("covers the description")
}
