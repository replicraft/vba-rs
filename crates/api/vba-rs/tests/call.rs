//! `Instance::call`: ByRef write-back, named and omitted arguments,
//! `ParamArray`, results and errors.

use vba::runtime::Error as RunError;
use vba::{Arg, CallTarget, Engine, Error, Instance, Project, RunStatus, Variant};

fn load(src: &str) -> Instance {
    let mut inst = Engine::new()
        .load(&Project::from_source("Module1", src))
        .unwrap();
    inst.trust();
    inst
}

fn number(e: &Error) -> i32 {
    match e {
        Error::Run(RunError::Vba(r) | RunError::Host(r)) => r.number,
        other => panic!("not a VBA error: {other:?}"),
    }
}

const BEFORE_UPDATE: &str =
    "Public Sub BeforeUpdate(Cancel As Integer)\n  Cancel = True\nEnd Sub\n";

#[test]
fn byref_parameter_is_written_back() {
    let mut inst = load(BEFORE_UPDATE);
    let r = inst
        .call(
            CallTarget::Project,
            "BeforeUpdate",
            vec![Arg::by_ref(Variant::Integer(0))],
        )
        .unwrap();
    assert_eq!(r.status, RunStatus::Completed);
    assert_eq!(r.byref, vec![Variant::Integer(-1)]);
    assert_eq!(r.value, Variant::Empty);
}

#[test]
fn the_slot_value_is_converted_to_the_declared_type_both_ways() {
    let mut inst = load(BEFORE_UPDATE);
    // A Boolean goes in, the declared Integer comes out.
    let r = inst
        .call(
            CallTarget::Project,
            "BeforeUpdate",
            vec![Arg::by_ref(Variant::Boolean(false))],
        )
        .unwrap();
    assert_eq!(r.byref, vec![Variant::Integer(-1)]);

    // The handler leaves it alone: the converted initial value is reported.
    let mut inst = load("Public Sub Keep(n As Long)\nEnd Sub\n");
    let r = inst
        .call(
            CallTarget::Project,
            "Keep",
            vec![Arg::by_ref(Variant::from("12"))],
        )
        .unwrap();
    assert_eq!(r.byref, vec![Variant::Long(12)]);

    // A variant parameter keeps the caller's subtype.
    let mut inst = load("Public Sub Keep(v)\n  v = v & \"!\"\nEnd Sub\n");
    let r = inst
        .call(
            CallTarget::Project,
            "Keep",
            vec![Arg::by_ref(Variant::from("hi"))],
        )
        .unwrap();
    assert_eq!(r.byref, vec![Variant::from("hi!")]);
}

#[test]
fn an_unconvertible_slot_is_a_type_mismatch() {
    let mut inst = load("Public Sub Keep(n As Long)\nEnd Sub\n");
    let err = inst
        .call(
            CallTarget::Project,
            "Keep",
            vec![Arg::by_ref(Variant::from("abc"))],
        )
        .unwrap_err();
    assert_eq!(number(&err), 13);
    // The instance is still usable.
    let r = inst
        .call(
            CallTarget::Project,
            "Keep",
            vec![Arg::by_ref(Variant::Long(1))],
        )
        .unwrap();
    assert_eq!(r.byref, vec![Variant::Long(1)]);
}

#[test]
fn byval_parameters_do_not_write_back() {
    let mut inst = load("Public Sub P(ByVal a As Long, b As Long)\n  a = 99\n  b = 98\nEnd Sub\n");
    let r = inst
        .call(
            CallTarget::Project,
            "P",
            vec![Arg::by_ref(Variant::Long(1)), Arg::by_ref(Variant::Long(2))],
        )
        .unwrap();
    assert_eq!(r.byref, vec![Variant::Long(1), Variant::Long(98)]);
}

#[test]
fn a_by_value_argument_is_not_reported() {
    let mut inst = load("Public Sub P(a As Long, b As Long)\n  a = 99\n  b = 98\nEnd Sub\n");
    let r = inst
        .call(
            CallTarget::Project,
            "P",
            vec![Arg::val(1), Arg::by_ref(Variant::Long(2))],
        )
        .unwrap();
    assert_eq!(r.byref, vec![Variant::Long(98)]);
}

#[test]
fn function_result_and_output() {
    let mut inst = load(
        "Public Function Add(a As Long, b As Long, ByRef total As Long) As Long\n  total = a + b\n  Debug.Print \"adding\"\n  Add = total * 2\nEnd Function\n",
    );
    let r = inst
        .call(
            CallTarget::Project,
            "Add",
            vec![Arg::val(2), Arg::val(3), Arg::by_ref(Variant::Long(0))],
        )
        .unwrap();
    assert_eq!(r.value, Variant::Long(10));
    assert_eq!(r.byref, vec![Variant::Long(5)]);
    assert_eq!(r.output, "adding\n");
    // The next call only reports its own output.
    let r = inst
        .call(
            CallTarget::Project,
            "Add",
            vec![Arg::val(1), Arg::val(1), Arg::by_ref(Variant::Long(0))],
        )
        .unwrap();
    assert_eq!(r.output, "adding\n");
}

const OPT: &str = "Public Function F(a As Long, Optional b As Long = 7, Optional c As String = \"d\", Optional v) As String\n  F = a & \"/\" & b & \"/\" & c & \"/\" & IsMissing(v)\nEnd Function\n";

#[test]
fn optional_missing_and_named_arguments() {
    let mut inst = load(OPT);
    let call = |inst: &mut Instance, args: Vec<Arg>| {
        inst.call(CallTarget::Project, "F", args).unwrap().value
    };
    assert_eq!(
        call(&mut inst, vec![Arg::val(1)]),
        Variant::from("1/7/d/True")
    );
    assert_eq!(
        call(&mut inst, vec![Arg::val(1), Arg::missing(), Arg::val("x")]),
        Variant::from("1/7/x/True")
    );
    assert_eq!(
        call(
            &mut inst,
            vec![
                Arg::val(1),
                Arg::named("c", Arg::val("z")),
                Arg::named("B", Arg::val(5))
            ]
        ),
        Variant::from("1/5/z/True")
    );
    assert_eq!(
        call(
            &mut inst,
            vec![Arg::named("v", Arg::val(0)), Arg::named("a", Arg::val(3))]
        ),
        Variant::from("3/7/d/False")
    );
}

#[test]
fn a_named_byref_argument_writes_back() {
    let mut inst = load("Public Sub P(Optional a As Long, Optional b As Long)\n  b = 5\nEnd Sub\n");
    let r = inst
        .call(
            CallTarget::Project,
            "P",
            vec![Arg::named("b", Arg::by_ref(Variant::Long(0)))],
        )
        .unwrap();
    assert_eq!(r.byref, vec![Variant::Long(5)]);
}

#[test]
fn param_array_collects_the_rest() {
    let mut inst = load(
        "Public Function Sum(ParamArray n()) As Long\n  Dim i, t As Long\n  For i = LBound(n) To UBound(n)\n    t = t + n(i)\n  Next\n  Sum = t\nEnd Function\n",
    );
    let r = inst
        .call(
            CallTarget::Project,
            "Sum",
            vec![Arg::val(1), Arg::val(2), Arg::val(3)],
        )
        .unwrap();
    assert_eq!(r.value, Variant::Long(6));
    let r = inst.call(CallTarget::Project, "Sum", vec![]).unwrap();
    assert_eq!(r.value, Variant::Long(0));
}

#[test]
fn argument_count_and_name_errors() {
    let mut inst = load("Public Sub P(a As Long, Optional b As Long)\nEnd Sub\n");
    let too_many = inst
        .call(
            CallTarget::Project,
            "P",
            vec![Arg::val(1), Arg::val(2), Arg::val(3)],
        )
        .unwrap_err();
    assert_eq!(number(&too_many), 450);
    let required = inst.call(CallTarget::Project, "P", vec![]).unwrap_err();
    assert_eq!(number(&required), 449);
    let unknown = inst
        .call(
            CallTarget::Project,
            "P",
            vec![Arg::val(1), Arg::named("zzz", Arg::val(2))],
        )
        .unwrap_err();
    assert_eq!(number(&unknown), 448);
    let twice = inst
        .call(
            CallTarget::Project,
            "P",
            vec![Arg::val(1), Arg::named("a", Arg::val(2))],
        )
        .unwrap_err();
    assert_eq!(number(&twice), 446);
    // Positional after named is a misuse of the API, not a VBA error.
    let order = inst
        .call(
            CallTarget::Project,
            "P",
            vec![Arg::named("b", Arg::val(2)), Arg::val(1)],
        )
        .unwrap_err();
    assert!(
        matches!(order, Error::Run(RunError::NotCallable(..))),
        "{order:?}"
    );
    // A conversion failure of a by-value argument is error 13 as well.
    let mismatch = inst
        .call(CallTarget::Project, "P", vec![Arg::val("abc")])
        .unwrap_err();
    assert_eq!(number(&mismatch), 13);
}

#[test]
fn no_leak_of_temporary_slots_across_failed_calls() {
    let mut inst = load("Public Sub P(a As Long)\n  Err.Raise 5\nEnd Sub\n");
    for _ in 0..3 {
        let err = inst
            .call(
                CallTarget::Project,
                "P",
                vec![Arg::by_ref(Variant::Long(1))],
            )
            .unwrap_err();
        assert_eq!(number(&err), 5);
    }
    let mut inst = load("Public Sub P(a As Long)\n  a = 4\nEnd Sub\n");
    let r = inst
        .call(
            CallTarget::Project,
            "P",
            vec![Arg::by_ref(Variant::Long(1))],
        )
        .unwrap();
    assert_eq!(r.byref, vec![Variant::Long(4)]);
}

#[test]
fn stop_and_end_report_their_status() {
    let mut inst = load(
        "Public Sub S(a As Long)\n  a = 3\n  Stop\n  a = 4\nEnd Sub\nPublic Sub E(a As Long)\n  a = 3\n  End\nEnd Sub\n",
    );
    let r = inst
        .call(
            CallTarget::Project,
            "S",
            vec![Arg::by_ref(Variant::Long(1))],
        )
        .unwrap();
    assert_eq!(r.status, RunStatus::Stopped);
    assert_eq!(r.byref, vec![Variant::Long(3)]);
    let r = inst
        .call(
            CallTarget::Project,
            "E",
            vec![Arg::by_ref(Variant::Long(1))],
        )
        .unwrap();
    assert_eq!(r.status, RunStatus::Ended);
}

#[test]
fn values_round_trip_through_calls() {
    let mut inst = load(
        "Public Function Echo(v As Variant) As Variant\n  Echo = v\nEnd Function\nPublic Function Kind(v As Variant) As String\n  Kind = TypeName(v)\nEnd Function\nPublic Function Total(v As Variant) As Long\n  Dim i, t As Long\n  For i = LBound(v) To UBound(v)\n    t = t + v(i)\n  Next\n  Total = t\nEnd Function\n",
    );
    let echo = |inst: &mut Instance, v: Variant| {
        inst.call(CallTarget::Project, "Echo", vec![Arg::val(v)])
            .unwrap()
            .value
    };
    let arr = echo(&mut inst, Variant::from(vec![1_i32, 2, 3]));
    assert_eq!(arr.array_bounds(), Some((0, 2)));
    let items: Vec<i32> = arr
        .to_vec()
        .unwrap()
        .iter()
        .map(|v| i32::try_from(v).unwrap())
        .collect();
    assert_eq!(items, vec![1, 2, 3]);
    let one = Variant::array_one_based(vec![Variant::from(10), Variant::from(20)]);
    assert_eq!(echo(&mut inst, one.clone()).array_bounds(), Some((1, 2)));
    let total = inst
        .call(CallTarget::Project, "Total", vec![Arg::val(one)])
        .unwrap();
    assert_eq!(total.value, Variant::Long(30));
    let date = Variant::date_from_ymd_hms(2024, 2, 29, 13, 30, 0).unwrap();
    assert_eq!(echo(&mut inst, date.clone()), date);
    assert_eq!(echo(&mut inst, Variant::from(None::<i32>)), Variant::Null);
    let kind = |inst: &mut Instance, v: Variant| {
        String::try_from(
            inst.call(CallTarget::Project, "Kind", vec![Arg::val(v)])
                .unwrap()
                .value,
        )
        .unwrap()
    };
    assert_eq!(kind(&mut inst, Variant::from(1.5_f64)), "Double");
    assert_eq!(kind(&mut inst, Variant::from(true)), "Boolean");
    assert_eq!(kind(&mut inst, date), "Date");
    assert_eq!(kind(&mut inst, Variant::nothing()), "Nothing");
}
