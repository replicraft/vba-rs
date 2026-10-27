//! Small embedding features: UI hooks, host constants, VBA library enum
//! types and default-member arguments of the `Is*` functions.

use std::cell::RefCell;
use std::rc::Rc;

use vba::automation::testing::MemObject;
use vba::{Engine, HostResult, InputBoxRequest, MsgBoxRequest, Project, UiHooks, Variant};

struct Ui(Rc<RefCell<Vec<String>>>);

impl UiHooks for Ui {
    fn msg_box(&self, req: &MsgBoxRequest) -> HostResult<i32> {
        self.0.borrow_mut().push(req.prompt.clone());
        Ok(6)
    }
    fn input_box(&self, req: &InputBoxRequest) -> HostResult<Option<String>> {
        self.0.borrow_mut().push(req.prompt.clone());
        Ok(Some(req.default.to_uppercase()))
    }
}

fn run(engine: &Engine, code: &str) -> String {
    let mut inst = engine.load(&Project::from_source("M", code)).unwrap();
    inst.trust();
    inst.run("Main", vec![]).unwrap().output
}

#[test]
fn engine_ui_answers_msgbox_and_inputbox_and_the_default_stays_headless() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let engine = Engine::new().ui(Ui(seen.clone()));
    let out = run(
        &engine,
        "Public Sub Main()\n Debug.Print MsgBox(\"ok?\", vbYesNo) = vbYes\n Debug.Print InputBox(\"who\", , \"ann\")\nEnd Sub\n",
    );
    assert_eq!(out, "True\nANN\n");
    assert_eq!(*seen.borrow(), ["ok?", "who"]);

    let out = run(
        &Engine::new(),
        "Public Sub Main()\n On Error Resume Next\n MsgBox \"x\"\n Debug.Print Err.Number\nEnd Sub\n",
    );
    assert_eq!(out, " 70 \n");
}

#[test]
fn host_constants_fold_and_work_in_const_case_and_bounds() {
    let engine = Engine::new()
        .constant("acNormal", 0)
        .constants([("acDialog", 3), ("acHidden", 1)])
        .constant("AppTitle", "Demo");
    let out = run(
        &engine,
        "\
Option Explicit
Const Both As Long = acDialog Or acHidden
Const Title As String = AppTitle & \"!\"
Dim a(acNormal To acDialog) As Long
Public Sub Main()
  Select Case 3
    Case acDialog: Debug.Print \"dialog\"
  End Select
  Debug.Print Both; Title; UBound(a); TypeName(acDialog); TypeName(AppTitle)
End Sub
",
    );
    assert_eq!(out, "dialog\n 3 Demo! 3 LongString\n");
    // Under Option Explicit an unknown name is still an error.
    let report = engine.analyze(&Project::from_source(
        "M",
        "Option Explicit\nPublic Sub Main()\n Debug.Print acMissing\nEnd Sub\n",
    ));
    assert!(report.has_errors());
}

#[test]
fn vba_library_enum_types_are_valid_as_clauses() {
    let engine = Engine::new();
    let out = run(
        &engine,
        "\
Option Explicit
Function Pick(Optional cmp As VbCompareMethod = vbTextCompare) As VbMsgBoxResult
  Pick = vbNo
End Function
Public Sub Main()
  Dim style As VbMsgBoxStyle, day As VBA.VbDayOfWeek, t As VbVarType
  style = vbYesNo + vbQuestion
  day = vbMonday
  t = vbString
  Debug.Print style; day; t; Pick() = vbNo; TypeName(style)
End Sub
",
    );
    assert_eq!(out, " 36  2  8 TrueLong\n");
}

#[test]
fn is_functions_look_at_the_default_member_of_objects_that_have_one() {
    let engine = Engine::new()
        .dispatch_global("NullBox", || {
            MemObject::new("TextBox")
                .property("Value", Variant::Null)
                .default_member("Value")
        })
        .dispatch_global("EmptyBox", || {
            MemObject::new("TextBox")
                .property("Value", Variant::Empty)
                .default_member("Value")
        })
        .dispatch_global("TextBox", || {
            MemObject::new("TextBox")
                .property("Value", Variant::from("12"))
                .default_member("Value")
        })
        .dispatch_global("Plain", || {
            MemObject::new("Thing").property("Size", Variant::Long(1))
        });
    let out = run(
        &engine,
        "\
Public Sub Main()
  Debug.Print IsNull(NullBox); IsNull(EmptyBox); IsEmpty(EmptyBox); IsEmpty(NullBox)
  Debug.Print IsNumeric(TextBox); IsDate(TextBox); VarType(TextBox); VarType(NullBox)
  Debug.Print TypeName(NullBox); IsObject(NullBox)
  Debug.Print IsNull(Plain); VarType(Plain); IsNull(New Collection); IsNull(Nothing)
End Sub
",
    );
    assert_eq!(
        out,
        "TrueFalseTrueFalse\nTrueFalse 8  1 \nTextBoxTrue\nFalse 9 FalseFalse\n"
    );
}

/// An object whose methods write a fixed value into their `ByRef` argument.
fn out_engine() -> Engine {
    use vba::automation::ParamInfo;
    fn writer(o: MemObject, name: &str, v: Variant) -> MemObject {
        o.method(name, vec![ParamInfo::required("v")], move |_, c| {
            c.args[0].value = v.clone();
            Ok(Variant::Empty)
        })
    }
    Engine::new().dispatch_global("Out", || {
        let o = writer(MemObject::new("Out"), "Str", Variant::from("12"));
        let o = writer(o, "Bad", Variant::from("abc"));
        let o = writer(o, "Big", Variant::Long(70000));
        writer(o, "Dbl", Variant::Double(2.5))
    })
}

#[test]
fn byref_write_back_converts_to_the_declared_type_of_the_variable() {
    let out = run(
        &out_engine(),
        "\
Option Explicit
Private Type Rec
  n As Long
End Type
Dim modLong As Long
Dim arr(1 To 2) As Long
Public Sub Main()
  Dim n As Long, s As String, i As Integer, v As Variant, r As Rec
  Out.Str n
  Debug.Print TypeName(n); n
  Out.Dbl s
  Debug.Print TypeName(s); s
  Out.Dbl i
  Debug.Print TypeName(i); i
  Out.Str v
  Debug.Print TypeName(v); v
  Out.Str modLong
  Debug.Print TypeName(modLong); modLong
  Out.Str arr(2)
  Debug.Print TypeName(arr(2)); arr(2)
  Out.Str r.n
  Debug.Print TypeName(r.n); r.n
  On Error Resume Next
  Out.Bad n
  Debug.Print Err.Number; TypeName(n); n
  Err.Clear
  Out.Big i
  Debug.Print Err.Number; i
  Err.Clear
  Out.Bad arr(1)
  Debug.Print Err.Number; arr(1)
  Err.Clear
  Out.Bad r.n
  Debug.Print Err.Number; r.n
  Err.Clear
  Out.Bad modLong
  Debug.Print Err.Number; modLong
End Sub
",
    );
    // A value that does not convert is error 13 (6 for an overflow) at the
    // call, trappable, and leaves the variable as it was.
    assert_eq!(
        out,
        "Long 12 \nString2.5\nInteger 2 \nString12\nLong 12 \nLong 12 \nLong 12 \n 13 Long 12 \n 6  2 \n 13  0 \n 13  12 \n 13  12 \n"
    );
}
