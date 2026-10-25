//! Table-driven resolution and invocation tests.

use vba_automation::call::{self, eval_chain, let_coerce};
use vba_automation::testing::MemObject;
use vba_automation::{
    Arg, CallArgs, DispId, DispParams, Final, InvokeFlags, ObjectTable, ParamInfo, Step,
};
use vba_value::{ObjectHandle, ObjectRef, Variant};

fn obj(h: ObjectHandle) -> Variant {
    Variant::Object(ObjectRef::Handle(h))
}

fn s(x: &str) -> Variant {
    Variant::from(x)
}

/// A default-`Item(Key)` object keyed by case-insensitive strings.
fn keyed(class: &str, items: Vec<(&str, Variant)>) -> MemObject {
    let items: Vec<(String, Variant)> =
        items.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    MemObject::new(class)
        .method("Item", vec![ParamInfo::required("Key")], move |t, c| {
            let Variant::String(k) = &c.args[0].value else {
                return Err(vba_automation::DispError::TYPE_MISMATCH);
            };
            let k = k.to_string_lossy();
            let v = items
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&k))
                .map(|(_, v)| v.clone())
                .ok_or(vba_automation::DispError::Hresult(
                    vba_automation::hresult::DISP_E_BADINDEX,
                ))?;
            t.add_ref_value(&v);
            Ok(v)
        })
        .default_member("Item")
}

fn field(name: &str, value: Variant) -> MemObject {
    MemObject::new("Field")
        .property("Name", s(name))
        .property("Value", value)
        .default_member("Value")
}

/// Recordset -> Fields (default, parameterless) -> Item(Key) -> Field -> Value.
fn recordset(table: &ObjectTable) -> Variant {
    let name = table.insert_obj(field("Name", s("Alice")));
    let age = table.insert_obj(field("Age", Variant::Long(30)));
    let fields = table.insert_obj(keyed(
        "Fields",
        vec![("Name", obj(name)), ("Age", obj(age))],
    ));
    let rs = MemObject::new("Recordset")
        .computed("Fields", move |t| {
            t.add_ref(fields).unwrap();
            obj(fields)
        })
        .default_member("Fields");
    obj(table.insert_obj(rs))
}

fn get(table: &ObjectTable, root: &Variant, mut steps: Vec<Step>) -> Result<Variant, i32> {
    eval_chain(table, root, &mut steps, Final::Get).map_err(|e| e.number)
}

fn value_of(table: &ObjectTable, root: &Variant, steps: Vec<Step>) -> Variant {
    let v = get(table, root, steps).unwrap();
    let r = let_coerce(table, &v).unwrap();
    table.release_value(&v);
    r
}

#[test]
fn default_member_chaining_forms() {
    let t = ObjectTable::new();
    let rs = recordset(&t);
    let forms: Vec<(&str, Vec<Step>)> = vec![
        ("rs!Name", vec![Step::bang("Name")]),
        ("rs!NAME", vec![Step::bang("NAME")]),
        (
            "rs(\"Name\")",
            vec![Step::Default(CallArgs::values([s("Name")]))],
        ),
        (
            "rs.Fields(\"name\")",
            vec![Step::member("fields", CallArgs::values([s("name")]))],
        ),
        (
            "rs.Fields.Item(\"Name\")",
            vec![
                Step::member("Fields", CallArgs::none()),
                Step::member("Item", CallArgs::values([s("Name")])),
            ],
        ),
        (
            "rs.Fields(\"Name\").Value",
            vec![
                Step::member("Fields", CallArgs::values([s("Name")])),
                Step::member("Value", CallArgs::none()),
            ],
        ),
        (
            "rs!Name.Value",
            vec![Step::bang("Name"), Step::member("Value", CallArgs::none())],
        ),
    ];
    for (label, steps) in forms {
        assert_eq!(value_of(&t, &rs, steps), s("Alice"), "{label}");
    }
    assert_eq!(
        value_of(&t, &rs, vec![Step::bang("age")]),
        Variant::Long(30)
    );
}

#[test]
fn forms_orders_total() {
    let t = ObjectTable::new();
    let total = t.insert_obj(field("Total", Variant::Double(12.5)));
    let orders = t.insert_obj(keyed("Form", vec![("Total", obj(total))]));
    let forms = t.insert_obj(keyed("Forms", vec![("Orders", obj(orders))]));
    let root = obj(forms);
    let steps = vec![Step::bang("Orders"), Step::bang("Total")];
    assert_eq!(value_of(&t, &root, steps), Variant::Double(12.5));
}

#[test]
fn let_through_default_members() {
    let t = ObjectTable::new();
    let rs = recordset(&t);
    // rs!Name = "Bob"  and  rs.Fields("Age") = 31
    eval_chain(&t, &rs, &mut [Step::bang("Name")], Final::Let(s("Bob"))).unwrap();
    eval_chain(
        &t,
        &rs,
        &mut [Step::member("Fields", CallArgs::values([s("Age")]))],
        Final::Let(Variant::Long(31)),
    )
    .unwrap();
    assert_eq!(value_of(&t, &rs, vec![Step::bang("Name")]), s("Bob"));
    assert_eq!(
        value_of(&t, &rs, vec![Step::bang("Age")]),
        Variant::Long(31)
    );
}

#[test]
fn let_coerces_object_values() {
    let t = ObjectTable::new();
    let holder = t.insert_obj(MemObject::new("Holder").property("X", Variant::Empty));
    let f = t.insert_obj(field("F", Variant::Long(7)));
    // Let h.X = f  stores f's default value, not the object.
    eval_chain(
        &t,
        &obj(holder),
        &mut [Step::member("X", CallArgs::none())],
        Final::Let(obj(f)),
    )
    .unwrap();
    let v = get(&t, &obj(holder), vec![Step::member("X", CallArgs::none())]).unwrap();
    assert_eq!(v, Variant::Long(7));
    // Set h.X = f stores the object.
    eval_chain(
        &t,
        &obj(holder),
        &mut [Step::member("X", CallArgs::none())],
        Final::Set(obj(f)),
    )
    .unwrap();
    let v = get(&t, &obj(holder), vec![Step::member("X", CallArgs::none())]).unwrap();
    assert_eq!(v, obj(f));
}

#[test]
fn let_vs_set_support() {
    let t = ObjectTable::new();
    let o = t.insert_obj(
        MemObject::new("Thing")
            .value_property("Name", s("n"))
            .object_property("Target"),
    );
    let other = t.insert_obj(field("F", Variant::Long(1)));
    let root = obj(o);
    let name = || vec![Step::member("Name", CallArgs::none())];
    let target = || vec![Step::member("Target", CallArgs::none())];
    // Set on a Let-only property: member exists but no PutRef -> 438.
    assert_eq!(
        eval_chain(&t, &root, &mut name(), Final::Set(obj(other))).map_err(|e| e.number),
        Err(438)
    );
    // Set with a non-object -> 424.
    assert_eq!(
        eval_chain(&t, &root, &mut target(), Final::Set(Variant::Long(1))).map_err(|e| e.number),
        Err(424)
    );
    // Let on a Set-only property: its current value is Nothing, which is not
    // an object to assign through -> 451.
    assert_eq!(
        eval_chain(&t, &root, &mut target(), Final::Let(Variant::Long(1))).map_err(|e| e.number),
        Err(451)
    );
    // Set then Let-through-default works: Target = other; Target = 5 sets F.
    eval_chain(&t, &root, &mut target(), Final::Set(obj(other))).unwrap();
    eval_chain(&t, &root, &mut target(), Final::Let(Variant::Long(5))).unwrap();
    assert_eq!(let_coerce(&t, &obj(other)).unwrap(), Variant::Long(5));
}

#[test]
fn error_table() {
    let t = ObjectTable::new();
    let rs = recordset(&t);
    let leaf = t.insert_obj(MemObject::new("Leaf").property("P", Variant::Long(1)));
    let nothing_holder =
        t.insert_obj(MemObject::new("H").property("N", Variant::Object(ObjectRef::Nothing)));
    let cases: Vec<(&str, Variant, Vec<Step>, i32)> = vec![
        (
            "unknown member",
            rs.clone(),
            vec![Step::member("Nope", CallArgs::none())],
            438,
        ),
        ("no default member", obj(leaf), vec![Step::bang("x")], 438),
        (
            "unknown named arg",
            rs.clone(),
            vec![Step::member(
                "Fields",
                CallArgs::none().with_named("zzz", Arg::val(1)),
            )],
            448,
        ),
        (
            "surplus arg on non-object result",
            obj(leaf),
            vec![Step::member("P", CallArgs::values([Variant::Long(1)]))],
            450,
        ),
        (
            "missing required arg",
            rs.clone(),
            vec![
                Step::bang("Name"),
                Step::member("Value", CallArgs::values([Variant::Long(1)])),
            ],
            450,
        ),
        (
            "Nothing intermediate",
            obj(nothing_holder),
            vec![
                Step::member("N", CallArgs::none()),
                Step::member("Z", CallArgs::none()),
            ],
            91,
        ),
        (
            "non-object intermediate",
            obj(leaf),
            vec![
                Step::member("P", CallArgs::none()),
                Step::member("Z", CallArgs::none()),
            ],
            424,
        ),
        ("bad key", rs.clone(), vec![Step::bang("missing")], 9),
        (
            "type mismatch",
            rs.clone(),
            vec![Step::Default(CallArgs::values([Variant::Long(1)]))],
            13,
        ),
    ];
    for (label, root, steps, want) in cases {
        assert_eq!(get(&t, &root, steps), Err(want), "{label}");
    }
}

#[test]
fn error_source_defaults_to_class_name() {
    let t = ObjectTable::new();
    let rs = recordset(&t);
    let e = eval_chain(
        &t,
        &rs,
        &mut [Step::member("Nope", CallArgs::none())],
        Final::Get,
    )
    .unwrap_err();
    assert_eq!(e.source, "Recordset");
    assert_eq!(
        e.description,
        "Object doesn't support this property or method"
    );
}

#[test]
fn raised_exception_maps_to_err() {
    let t = ObjectTable::new();
    let o = t.insert_obj(MemObject::new("Svc").method("Fail", vec![], |_, _| {
        Err(vba_automation::DispError::Exception(
            vba_automation::ExcepInfo {
                scode: vba_automation::hresult::VB_OBJECT_ERROR + 1001,
                source: "Svc.Module".into(),
                description: "boom".into(),
                help_file: "svc.hlp".into(),
                help_context: 42,
                code: 0,
            },
        ))
    }));
    let e = call::invoke_member(&t, o, "fail", &mut CallArgs::none(), &Final::Call).unwrap_err();
    assert_eq!(e.number, vba_automation::hresult::VB_OBJECT_ERROR + 1001);
    assert_eq!(e.source, "Svc.Module");
    assert_eq!(e.description, "boom");
    assert_eq!(e.help_file, "svc.hlp");
    assert_eq!(e.help_context, 42);
}

fn add_object() -> MemObject {
    // Add(a, Optional b = 10, Optional c) -> a*10000 + b*100 + (c or 0)
    MemObject::new("Calc")
        .method(
            "Add",
            vec![
                ParamInfo::required("a"),
                ParamInfo::optional("b").with_default(Variant::Long(10)),
                ParamInfo::optional("c"),
            ],
            |_, c| {
                let n = |a: &Arg| match a.value {
                    Variant::Long(v) => v,
                    _ => 0,
                };
                let c3 = if c.args[2].is_missing() {
                    0
                } else {
                    n(&c.args[2])
                };
                Ok(Variant::Long(
                    n(&c.args[0]) * 10000 + n(&c.args[1]) * 100 + c3,
                ))
            },
        )
        .method("Sum", vec![ParamInfo::param_array("vals")], |_, c| {
            Ok(Variant::Long(
                c.rest
                    .iter()
                    .map(|a| match a.value {
                        Variant::Long(v) => v,
                        _ => 0,
                    })
                    .sum(),
            ))
        })
        .method(
            "Swap",
            vec![ParamInfo::required("x"), ParamInfo::required("y")],
            |_, c| {
                let (a, b) = (c.args[0].value.clone(), c.args[1].value.clone());
                c.args[0].value = b;
                c.args[1].value = a;
                Ok(Variant::Empty)
            },
        )
        .method(
            "Clobber",
            vec![ParamInfo::required("v").by_val(), ParamInfo::required("w")],
            |_, c| {
                c.args[0].value = Variant::Long(-1);
                c.args[1].value = Variant::Long(-2);
                Ok(Variant::Empty)
            },
        )
}

#[test]
fn optional_named_and_paramarray() {
    let t = ObjectTable::new();
    let o = t.insert_obj(add_object());
    let l = Variant::Long;
    let call = |args: CallArgs| {
        call::invoke_member(&t, o, "ADD", &mut args.clone(), &Final::Get).map_err(|e| e.number)
    };
    assert_eq!(call(CallArgs::values([l(1)])), Ok(l(11000)));
    assert_eq!(call(CallArgs::values([l(1), l(2)])), Ok(l(10200)));
    assert_eq!(call(CallArgs::values([l(1), l(2), l(3)])), Ok(l(10203)));
    assert_eq!(
        call(CallArgs::values([l(1)]).with_named("c", Arg::val(l(9)))),
        Ok(l(11009))
    );
    assert_eq!(
        call(
            CallArgs::none()
                .with_named("B", Arg::val(l(5)))
                .with_named("a", Arg::val(l(2)))
        ),
        Ok(l(20500))
    );
    // Omitted positional argument (`Add 1, , 3`).
    assert_eq!(
        call(CallArgs::positional(vec![
            Arg::val(l(1)),
            Arg::missing(),
            Arg::val(l(3))
        ])),
        Ok(l(11003))
    );
    assert_eq!(call(CallArgs::none()), Err(449));
    assert_eq!(call(CallArgs::values([l(1), l(2), l(3), l(4)])), Err(450));
    assert_eq!(
        call(CallArgs::values([l(1)]).with_named("a", Arg::val(l(1)))),
        Err(448)
    );
    let sum = |n: usize| {
        call::invoke_member(
            &t,
            o,
            "sum",
            &mut CallArgs::values((0..n as i32).map(Variant::Long)),
            &Final::Get,
        )
        .unwrap()
    };
    assert_eq!(sum(0), l(0));
    assert_eq!(sum(5), l(10));
}

#[test]
fn byref_write_back() {
    let t = ObjectTable::new();
    let o = t.insert_obj(add_object());
    let mut a = CallArgs::positional(vec![Arg::by_ref(1i32), Arg::by_ref(2i32)]);
    call::invoke_member(&t, o, "Swap", &mut a, &Final::Call).unwrap();
    assert_eq!(a.positional[0].value, Variant::Long(2));
    assert_eq!(a.positional[1].value, Variant::Long(1));

    // By-value caller arguments are untouched.
    let mut a = CallArgs::positional(vec![Arg::val(1i32), Arg::val(2i32)]);
    call::invoke_member(&t, o, "Swap", &mut a, &Final::Call).unwrap();
    assert_eq!(a.positional[0].value, Variant::Long(1));

    // A ByVal parameter never writes back, even for a ByRef argument.
    let mut a = CallArgs::positional(vec![Arg::by_ref(5i32), Arg::by_ref(6i32)]);
    call::invoke_member(&t, o, "Clobber", &mut a, &Final::Call).unwrap();
    assert_eq!(a.positional[0].value, Variant::Long(5));
    assert_eq!(a.positional[1].value, Variant::Long(-2));

    // ByRef through a named argument.
    let mut a = CallArgs::none()
        .with_named("y", Arg::by_ref(1i32))
        .with_named("x", Arg::by_ref(2i32));
    call::invoke_member(&t, o, "Swap", &mut a, &Final::Call).unwrap();
    assert_eq!(a.named[0].1.value, Variant::Long(2));
    assert_eq!(a.named[1].1.value, Variant::Long(1));
}

#[test]
fn flags_and_ids() {
    let t = ObjectTable::new();
    let o = t.insert_obj(
        MemObject::new("E")
            .method("Eval", vec![ParamInfo::required("expr")], |_, c| {
                Ok(c.args[0].value.clone())
            })
            .with_dispid("Eval", DispId::EVALUATE)
            .value_property("V", Variant::Long(1)),
    );
    assert_eq!(call::evaluate(&t, o, "A1").unwrap(), s("A1"));

    let d = t.get(o).unwrap();
    let ids = d.get_ids_of_names(&["eval", "EXPR"]).unwrap();
    assert_eq!(ids, vec![DispId::EVALUATE, DispId(0)]);
    assert_eq!(
        d.get_ids_of_names(&["eval", "nope"]).unwrap_err().hresult(),
        vba_automation::hresult::DISP_E_UNKNOWNNAME
    );
    // V has no PropertyPutRef.
    let mut dp = DispParams::new();
    dp.named.push((DispId::PROPERTYPUT, Arg::val(2i32)));
    let id = d.member_id("v").unwrap();
    assert_eq!(
        d.invoke(&t, id, InvokeFlags::PROPERTY_PUT_REF, &mut dp)
            .unwrap_err()
            .hresult(),
        vba_automation::hresult::DISP_E_MEMBERNOTFOUND
    );
    assert!(d.invoke(&t, id, InvokeFlags::PROPERTY_PUT, &mut dp).is_ok());
    assert_eq!(
        d.invoke(&t, id, InvokeFlags::PROPERTY_GET, &mut DispParams::new())
            .unwrap(),
        Variant::Long(2)
    );
}

#[test]
fn for_each_protocol() {
    let t = ObjectTable::new();
    let a = t.insert_obj(field("A", Variant::Long(1)));
    let coll =
        t.insert_obj(MemObject::new("Col").collection(vec![Variant::Long(10), obj(a), s("x")]));
    let before = t.ref_count(a).unwrap();
    let mut it = vba_automation::ForEach::start(&t, &obj(coll)).unwrap();
    let mut seen = Vec::new();
    while let Some(v) = it.next_item().unwrap() {
        seen.push(v);
    }
    assert_eq!(seen, vec![Variant::Long(10), obj(a), s("x")]);
    // Object elements come with a reference the receiver owns.
    assert_eq!(t.ref_count(a).unwrap(), before + 1);
    t.release_value(&seen[1]);
    drop(it);
    assert_eq!(t.live_count(), 2); // field a + collection

    // Item(2) and Count through the call helpers.
    let v = call::invoke_default(
        &t,
        coll,
        &mut CallArgs::values([Variant::Long(3)]),
        &Final::Get,
    )
    .unwrap();
    assert_eq!(v, s("x"));
    assert_eq!(
        call::invoke_member(&t, coll, "count", &mut CallArgs::none(), &Final::Get).unwrap(),
        Variant::Long(3)
    );

    // Errors.
    let plain = t.insert_obj(MemObject::new("Plain"));
    assert_eq!(
        vba_automation::ForEach::start(&t, &obj(plain))
            .err()
            .unwrap()
            .number,
        438
    );
    assert_eq!(
        vba_automation::ForEach::start(&t, &Variant::Object(ObjectRef::Nothing))
            .err()
            .unwrap()
            .number,
        91
    );
    assert_eq!(
        vba_automation::ForEach::start(&t, &Variant::Long(1))
            .err()
            .unwrap()
            .number,
        438
    );
}
