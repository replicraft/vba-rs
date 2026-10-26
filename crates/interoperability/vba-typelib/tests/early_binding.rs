//! Early binding end to end: type library -> sema -> IR -> VM -> host object.
//!
//! The same programs run with a statically typed receiver (`Dim d As
//! Scripting.Dictionary`, bound by member id) and with an `Object` receiver
//! (bound by name); the output must be identical. Errors that early binding
//! reports at compile time are checked against the run-time errors of late
//! binding.
#![cfg(feature = "automation")]

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use common::builder::build;
use common::libs;
use vba_automation::testing::MemObject;
use vba_automation::{
    DispError, DispId, DispParams, Dispatch, InvokeFlags, MemberInfo, ObjectTable,
};
use vba_diagnostics::{Severity, render_plain};
use vba_host::{Capability, HeadlessHost, Host, Policy, ScriptedHost};
use vba_project::ModuleKind;
use vba_scripting::ScriptingHost;
use vba_sema::{Analysis, LibraryType, SourceModule};
use vba_typelib::automation::InterfaceTable;
use vba_typelib::{parse_msft, sema::TypelibProvider};
use vba_value::{ObjectRef, Variant};
use vba_vm::{BindStats, BufferSink, NoServices, Vm, VmConfig};

fn provider() -> TypelibProvider {
    TypelibProvider::new()
        .with_library(parse_msft(&build(&libs::scripting())).unwrap())
        .with_library(parse_msft(&build(&libs::calc())).unwrap())
}

fn com() -> Policy {
    Policy::deny_all().grant(Capability::ComCreate)
}

fn analyse(src: &str) -> Analysis {
    vba_sema::analyze_sources(
        &[SourceModule {
            name: "M".into(),
            kind: ModuleKind::Standard,
            source: src.as_bytes().to_vec(),
        }],
        vba_profile::default_profile(),
        &provider(),
        &vba_sema::Options::default(),
    )
}

/// `CODE@line` of errors.
fn errors(a: &Analysis) -> Vec<String> {
    a.diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| {
            let f = a.sources.get(d.file).expect("file");
            format!("{}@{}", d.code.0, f.line_col(d.span.start).line + 1)
        })
        .collect()
}

struct Ran {
    out: String,
    error: Option<String>,
    stats: BindStats,
}

fn run_on(src: &str, host: Rc<dyn Host>) -> Ran {
    let a = analyse(src);
    let rendered: Vec<String> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| render_plain(d, &a.sources))
        .collect();
    assert!(
        rendered.is_empty(),
        "compile errors:\n{}",
        rendered.join("\n")
    );
    let ir = vba_ir::lower(&a).expect("lowering");
    vba_ir::verify(&ir).expect("verified IR");
    let sink = BufferSink::new();
    let mut vm = Vm::new(
        Rc::new(ir),
        host,
        Box::new(NoServices),
        Box::new(vba_stdlib::FixedHost::default()),
        Box::new(sink.clone()),
        VmConfig::default(),
    )
    .expect("vm");
    let main = vm.find_procedure("Main").expect("Main");
    let result = vm.run_procedure(main, vec![]);
    Ran {
        out: sink.contents(),
        error: result.err().map(|e| e.to_string()),
        stats: vm.bind_stats(),
    }
}

/// The VBA error number of a dispatch error.
fn number(e: DispError) -> i32 {
    e.into_err_info("test").number
}

fn scripting_host() -> Rc<dyn Host> {
    ScriptingHost::new(Rc::new(HeadlessHost::new().with_policy(com())))
}

// ---- a calculator host object driven by the dispatch table -----------------

struct Calculator {
    table: Rc<InterfaceTable>,
    total: Cell<i32>,
    mode: Cell<i32>,
    calls: RefCell<Vec<String>>,
}

impl Dispatch for Calculator {
    fn class_name(&self) -> String {
        "Calculator".into()
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        self.table.member_id(name)
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        self.table.member_info(id)
    }
    fn accepts_dispid(&self, id: DispId, name: &str) -> bool {
        self.table.accepts_dispid(id, name)
    }
    fn get_ids_of_names(&self, names: &[&str]) -> Result<Vec<DispId>, DispError> {
        self.table.get_ids_of_names(names)
    }
    fn invoke(
        &self,
        _table: &ObjectTable,
        id: DispId,
        flags: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let mut b = self.table.bind_args(id, flags, params)?;
        let long = |v: &Variant| match v {
            Variant::Long(n) => *n,
            other => panic!("the table converts to the declared type, got {other:?}"),
        };
        let name = self.table.by_id(id).unwrap().name.clone();
        self.calls.borrow_mut().push(name);
        let r = match id.0 {
            0 => Variant::Long(long(&b.slots[0].value) * 10),
            1 => {
                let (x, y) = (long(&b.slots[0].value), long(&b.slots[1].value));
                Variant::Long(
                    x.checked_add(y)
                        .ok_or_else(|| DispError::raise(6, "Overflow", "Calculator"))?,
                )
            }
            2 => {
                let Variant::Double(v) = b.slots[0].value else {
                    panic!("Double expected: {:?}", b.slots[0].value)
                };
                let Variant::Double(f) = b.slots[1].value else {
                    panic!("Double expected: {:?}", b.slots[1].value)
                };
                b.slots[0].value = Variant::Double(v * f);
                Variant::Empty
            }
            3 => {
                let Variant::String(s) = &b.slots[0].value else {
                    panic!("String expected")
                };
                Variant::from(format!("Hello, {}", s.to_string_lossy()).as_str())
            }
            4 if flags.is_write() => {
                self.total.set(long(b.value.as_ref().unwrap()));
                Variant::Empty
            }
            4 => Variant::Long(self.total.get()),
            5 => Variant::from("1.0"),
            6 if flags.is_write() => {
                self.mode.set(long(b.value.as_ref().unwrap()));
                Variant::Empty
            }
            6 => Variant::Long(self.mode.get()),
            7 => b.slots[0].value.clone(),
            8 => {
                let Variant::String(sep) = &b.slots[0].value else {
                    panic!("String expected")
                };
                let parts: Vec<String> = b
                    .rest
                    .iter()
                    .map(|a| {
                        vba_value::convert::to_vba_string(&a.value)
                            .unwrap()
                            .to_string_lossy()
                    })
                    .collect();
                Variant::from(parts.join(&sep.to_string_lossy()).as_str())
            }
            11 => {
                let (x, y) = (long(&b.slots[0].value), long(&b.slots[1].value));
                if y == 0 {
                    return Err(DispError::raise(11, "Division by zero", "Calculator"));
                }
                b.slots[2].value = Variant::Long(x % y);
                Variant::Long(x / y)
            }
            10 => {
                let Variant::Array(a) = &b.slots[0].value else {
                    panic!("array expected")
                };
                Variant::Long(a.iter().map(long).sum())
            }
            _ => return Err(DispError::MEMBER_NOT_FOUND),
        };
        b.write_back(params);
        self.table.coerce_result(id, r)
    }
}

/// The calculators a host has created.
type Made = Rc<RefCell<Vec<Rc<Calculator>>>>;

fn calc_host() -> (Rc<dyn Host>, Made) {
    let p = provider();
    let table = Rc::new(
        p.interface_table(&LibraryType {
            library: "Calc".into(),
            name: "Calculator".into(),
        })
        .expect("table"),
    );
    let made: Made = Rc::default();
    let m = made.clone();
    let host = ScriptedHost::new()
        .with_policy(com())
        .with_factory("Calc.Calculator", move |t| {
            let c = Rc::new(Calculator {
                table: table.clone(),
                total: Cell::new(0),
                mode: Cell::new(0),
                calls: RefCell::default(),
            });
            m.borrow_mut().push(c.clone());
            let h = t.insert(c);
            Ok(Variant::Object(ObjectRef::Handle(h)))
        });
    (Rc::new(host), made)
}

/// Runs `body` (the statements of `Main`, using `d`) twice: with `d` typed as
/// `typed` and as `Object`, on a fresh host each time. Returns both runs.
fn both(
    decl: &str,
    typed: &str,
    create: &str,
    body: &str,
    host: impl Fn() -> Rc<dyn Host>,
) -> (Ran, Ran) {
    let program = |ty: &str| {
        format!(
            "Option Explicit\nSub Main()\nDim d As {ty}\nSet d = CreateObject(\"{create}\")\n{decl}{body}\nEnd Sub\n"
        )
    };
    (
        run_on(&program(typed), host()),
        run_on(&program("Object"), host()),
    )
}

fn assert_same(early: &Ran, late: &Ran) {
    assert_eq!(early.out, late.out, "early and late output differ");
    assert_eq!(early.error, late.error);
}

#[test]
fn dictionary_early_and_late_agree() {
    let body = r#"
d.Add "a", 1
d.Add "b", "two"
d("c") = 3.5
d.Item("d") = Empty
Debug.Print d.Count, d.Exists("a"), d.Exists("zz")
Debug.Print d("a") + 1, d.Item("b"), d("c")
Dim k As Variant
For Each k In d.Keys
Debug.Print k;
Next
Debug.Print
Dim v As Variant
For Each v In d.Items
Debug.Print TypeName(v);
Next
Debug.Print
d.Key("a") = "A"
Debug.Print d.Exists("a"), d.Exists("A"), d("A")
d.Remove "b"
Debug.Print d.Count
d.RemoveAll
Debug.Print d.Count
d.CompareMode = 1
d.Add "Key", 7
Debug.Print d.Exists("KEY"), d.CompareMode
d("Other") = d("key") * 2
Debug.Print d("OTHER")
"#;
    let (early, late) = both(
        "",
        "Scripting.Dictionary",
        "Scripting.Dictionary",
        body,
        scripting_host,
    );
    assert_eq!(early.error, None, "{}", early.out);
    assert_same(&early, &late);
    assert!(early.out.contains("abcd"), "{}", early.out);
    // Every call on the typed variable went by id; none on the Object.
    assert!(early.stats.early >= 20, "{:?}", early.stats);
    assert_eq!(early.stats.by_name, 0, "{:?}", early.stats);
    assert_eq!(late.stats.early, 0, "{:?}", late.stats);
    assert!(late.stats.by_name >= 20, "{:?}", late.stats);
}

#[test]
fn dictionary_errors_agree() {
    let body = r#"
On Error Resume Next
d.Add "a", 1
d.Add "a", 2
Debug.Print Err.Number, Err.Source
Err.Clear
d.Remove "zz"
Debug.Print Err.Number
Err.Clear
d.CompareMode = "abc"
Debug.Print Err.Number
Err.Clear
d.Key("zz") = "y"
Debug.Print Err.Number
Err.Clear
d.CompareMode = 1
Debug.Print Err.Number
Debug.Print d.Count
"#;
    let (early, late) = both(
        "",
        "Scripting.Dictionary",
        "Scripting.Dictionary",
        body,
        scripting_host,
    );
    assert_eq!(early.error, None);
    assert_same(&early, &late);
    assert!(early.out.contains("457"), "{}", early.out);
    assert!(early.out.contains("Dictionary"), "{}", early.out);
}

#[test]
fn calculator_early_and_late_agree() {
    let body = r#"
Dim v As Double
Debug.Print d.Add(2, 3), d(4), d.Item(5)
v = 4
d.Scale v
Debug.Print v
d.Scale v, 3
Debug.Print v
d.Scale factor:=0.5, v:=v
Debug.Print v
Debug.Print d.Greet, d.Greet("VBA"), d.Greet(name:="named")
d.Total = 7
Debug.Print d.Total
d.Total = d.Total + 1.4
Debug.Print d.Total
d.Mode = 2
Debug.Print d.Mode, d.Pick, d.Pick(2)
Debug.Print d.Join("-", 1, "b", 3.5)
Debug.Print d.Join("+")
Debug.Print d.Version
Dim a(1 To 3) As Long
a(1) = 5: a(2) = 6: a(3) = 7
Debug.Print d.Sum(a)
Dim n As Long
Dim q As Long, r As Long
q = d.DivMod(17, 5, r)
Debug.Print q; r
r = 99
Debug.Print d.DivMod(r, 10, q); q
On Error Resume Next
q = d.DivMod(1, 0, r)
Debug.Print Err.Number; r
n = d.Add(1, 2) * 2
Debug.Print n, TypeName(d.Add(1, 2)), TypeName(d.Greet), TypeName(d.Version)
"#;
    let (early, late) = both("", "Calc.Calculator", "Calc.Calculator", body, || {
        calc_host().0
    });
    assert_eq!(early.error, None, "{}", early.out);
    assert_same(&early, &late);
    assert!(early.out.contains("Hello, world"), "{}", early.out);
    assert!(early.out.contains("Hello, named"), "{}", early.out);
    assert_eq!(early.stats.by_name, 0, "{:?}", early.stats);
    assert!(early.stats.early >= 20);
}

#[test]
fn calculator_conversion_errors_agree() {
    let body = r#"
On Error Resume Next
Dim x As Variant
x = "abc"
d.Add x, 1
Debug.Print "add", Err.Number
Err.Clear
d.Total = x
Debug.Print "total", Err.Number
Err.Clear
d.Add 2147483647, 1
Debug.Print "overflow", Err.Number, Err.Description
Err.Clear
Debug.Print d.Add("7", "8")
Debug.Print Err.Number
"#;
    let (early, late) = both("", "Calc.Calculator", "Calc.Calculator", body, || {
        calc_host().0
    });
    assert_eq!(early.error, None);
    assert_same(&early, &late);
    let lines: Vec<&str> = early.out.lines().collect();
    assert!(
        lines[0].starts_with("add") && lines[0].contains("13"),
        "{lines:?}"
    );
    assert!(
        lines[1].starts_with("total") && lines[1].contains("13"),
        "{lines:?}"
    );
    assert!(
        lines[2].starts_with("overflow") && lines[2].contains(" 6 "),
        "{lines:?}"
    );
    // "7" + "8" converts to Long arguments.
    assert_eq!(lines[3].trim(), "15", "{lines:?}");
}

#[test]
fn the_object_receives_converted_arguments() {
    // `Total` is a Long: the host never sees the Double.
    let (host, made) = calc_host();
    let src = "Sub Main()\nDim d As Calc.Calculator\nSet d = CreateObject(\"Calc.Calculator\")\nd.Total = 2.6\nDebug.Print d.Total\nEnd Sub\n";
    let r = run_on(src, host);
    assert_eq!(r.out, " 3 \n");
    assert_eq!(
        made.borrow()[0].calls.borrow().as_slice(),
        ["Total", "Total"]
    );
}

// ---- compile-time errors versus run-time errors of late binding ------------

#[test]
fn early_binding_reports_at_compile_time_what_late_binding_finds_at_run_time() {
    // Compile time (typed receiver).
    let a = analyse(
        "Option Explicit\n\
         Sub Main()\n\
         Dim d As Calc.Calculator\n\
         Dim n As Long\n\
         d.Add 1\n\
         d.Add 1, 2, 3\n\
         d.Nope\n\
         d.Version = \"x\"\n\
         d.Scale 1, 2\n\
         n = d.Greet(1, 2)\n\
         d.Pick Nope:=1\n\
         End Sub\n",
    );
    assert_eq!(
        errors(&a),
        vec![
            "SEM008@5",
            "SEM008@6",
            "SEM019@7",
            "SEM029@8",
            "SEM008@10",
            "SEM008@11"
        ],
        "{:?}",
        a.diagnostics
    );
    // Line 9, `d.Scale 1, 2`, is fine: a constant passed for a `ByRef`
    // parameter is a temporary. Line 10 has one argument too many and line
    // 11 names a parameter that does not exist.
}

#[test]
fn the_same_mistakes_are_run_time_errors_with_late_binding() {
    let body = r#"
On Error Resume Next
d.Add 1
Debug.Print "count " & Err.Number
Err.Clear
d.Add 1, 2, 3
Debug.Print "many " & Err.Number
Err.Clear
d.Nope
Debug.Print "missing " & Err.Number
Err.Clear
d.Version = "x"
Debug.Print "readonly " & Err.Number
Err.Clear
d.Pick Nope:=1
Debug.Print "named " & Err.Number
"#;
    let program = format!(
        "Sub Main()\nDim d As Object\nSet d = CreateObject(\"Calc.Calculator\")\n{body}End Sub\n"
    );
    let r = run_on(&program, calc_host().0);
    assert_eq!(r.error, None, "{}", r.out);
    // 449 argument not optional, 450 wrong number of arguments, 438 object
    // does not support the member, 451 (a Let on a member that has no
    // Property Let: late binding assigns to the default member of the
    // result), 448 named argument not found.
    assert_eq!(
        r.out,
        "count 449\nmany 450\nmissing 438\nreadonly 451\nnamed 448\n"
    );
}

#[test]
fn argument_types_are_checked_against_the_declaration() {
    let a = analyse(
        "Option Explicit\n\
         Sub Main()\n\
         Dim d As Calc.Calculator\n\
         Dim p As Calc.Point\n\
         Dim o As Object\n\
         d.Corner 5\n\
         d.Corner p\n\
         d.Corner o\n\
         d.Mode = d\n\
         End Sub\n",
    );
    // A number is not a record, and an `Object` variable cannot be passed
    // `ByRef` where a record is declared; the record itself is accepted.
    assert_eq!(
        errors(&a),
        vec!["SEM006@6", "SEM026@8"],
        "{:?}",
        a.diagnostics
    );
}

#[test]
fn the_ir_carries_the_member_id_only_for_typed_receivers() {
    let src = |ty: &str| {
        format!("Sub Main()\nDim d As {ty}\nDim n As Long\nn = d.Count\nd.Add 1, 2\nEnd Sub\n")
    };
    let dump = |ty: &str| {
        let a = analyse(&src(ty));
        let ir = vba_ir::lower(&a).expect("lowering");
        vba_ir::verify(&ir).expect("verified IR");
        vba_ir::dump_program(&ir, vba_ir::DumpOptions::default())
    };
    let typed = dump("Scripting.Dictionary");
    assert!(typed.contains(".Count#7"), "{typed}");
    assert!(typed.contains(".Add#1"), "{typed}");
    let late = dump("Object");
    assert!(!late.contains('#'), "{late}");
}

#[test]
fn events_of_library_classes_come_from_the_source_interface() {
    let class_module = |src: &str| {
        vba_sema::analyze_sources(
            &[SourceModule {
                name: "C".into(),
                kind: ModuleKind::Class,
                source: src.as_bytes().to_vec(),
            }],
            vba_profile::default_profile(),
            &provider(),
            &vba_sema::Options::default(),
        )
    };
    let ok = class_module(
        "Option Explicit\n\
         Private WithEvents c As Calc.Calculator\n\
         Private Sub c_Changed(ByVal m As Calc.Mode)\n\
         End Sub\n\
         Private Sub c_Failed(ByVal code As Long, ByRef cancel As Boolean)\n\
         cancel = True\n\
         End Sub\n",
    );
    assert_eq!(errors(&ok), Vec::<String>::new(), "{:?}", ok.diagnostics);
    assert_eq!(ok.program.modules[0].handlers.len(), 2);
    let bad = class_module(
        "Option Explicit\n\
         Private WithEvents c As Calc.Calculator\n\
         Private Sub c_Changed(ByVal m As String)\n\
         End Sub\n\
         Private Sub c_Nothing()\n\
         End Sub\n",
    );
    assert_eq!(
        errors(&bad),
        vec!["SEM017@3", "SEM017@5"],
        "{:?}",
        bad.diagnostics
    );
}

// ---- library types at run time -----------------------------------------------

#[test]
fn enums_and_records_work_at_run_time() {
    let src = r#"
Sub Main()
    Dim m As Calc.Mode
    Dim p As Calc.Point
    Dim q As Calc.Point
    m = Fast
    Debug.Print m + 1; Calc.Exact; Mode.Fast
    p.X = 3
    p.Y = 4
    q = p
    p.X = 9
    Debug.Print q.X; p.X; q.Y
    With q
        .Y = .X * 2
    End With
    Debug.Print q.Y
    Dim r(1 To 2) As Calc.Point
    r(2).X = 5
    Debug.Print r(1).X; r(2).X
End Sub
"#;
    let r = run_on(src, calc_host().0);
    assert_eq!(r.error, None, "{}", r.out);
    assert_eq!(r.out, " 2  2  1 \n 3  9  4 \n 6 \n 0  5 \n");
}

// ---- objects that do not provide the library's ids -------------------------

#[test]
fn objects_with_private_ids_are_called_by_name() {
    let host = ScriptedHost::new()
        .with_policy(com())
        .with_factory("Scripting.Dictionary", |t| {
            let o = MemObject::new("Fake").property("Count", Variant::Long(42));
            Ok(Variant::Object(ObjectRef::Handle(t.insert_obj(o))))
        });
    let src = "Sub Main()\nDim d As Scripting.Dictionary\nSet d = CreateObject(\"Scripting.Dictionary\")\nDebug.Print d.Count\nEnd Sub\n";
    let r = run_on(src, Rc::new(host));
    assert_eq!(r.out, " 42 \n");
    // Count is id 7 in the library, id 1 on the object: not accepted, so the
    // call went through the name lookup.
    assert_eq!(
        r.stats,
        BindStats {
            early: 0,
            by_name: 1,
            fallbacks: 1
        }
    );
}

#[test]
fn a_matching_id_with_another_name_is_never_invoked() {
    // The library says `Count` is id 7; this object has a *different* member
    // with id 7 and no `Count` at all.
    let host = ScriptedHost::new()
        .with_policy(com())
        .with_factory("Scripting.Dictionary", |t| {
            let mut o = MemObject::new("Fake");
            for n in ["A", "B", "C", "D", "E", "F"] {
                o = o.property(n, Variant::Long(0));
            }
            o = o
                .property("Zed", Variant::from("wrong member"))
                .with_dispid("Zed", DispId(7));
            Ok(Variant::Object(ObjectRef::Handle(t.insert_obj(o))))
        });
    let src = "Sub Main()\nOn Error Resume Next\nDim d As Scripting.Dictionary\nSet d = CreateObject(\"Scripting.Dictionary\")\nDebug.Print d.Count\nDebug.Print Err.Number\nEnd Sub\n";
    let r = run_on(src, Rc::new(host));
    assert_eq!(r.out, " 438 \n");
    assert_eq!(r.stats.early, 0);
    assert_eq!(r.stats.fallbacks, 1);
}

#[test]
fn stale_receivers_report_the_usual_errors() {
    let src = "Sub Main()\nOn Error Resume Next\nDim d As Scripting.Dictionary\nDebug.Print d.Count\nDebug.Print Err.Number\nEnd Sub\n";
    let r = run_on(src, scripting_host());
    assert_eq!(r.out, " 91 \n");
}

#[test]
fn the_table_describes_members() {
    let p = provider();
    let t = p
        .interface_table(&LibraryType {
            library: "Calc".into(),
            name: "ICalc".into(),
        })
        .unwrap();
    assert_eq!(t.member_id("GREET"), Some(DispId(3)));
    assert_eq!(t.member_id("nothing"), None);
    let scale = t.member_info(DispId(2)).unwrap();
    assert!(scale.params[0].by_ref && !scale.params[0].optional);
    assert!(scale.params[1].optional);
    assert_eq!(scale.params[1].default, Some(Variant::Double(2.5)));
    let join = t.by_name("Join").unwrap();
    assert!(join.params[1].param_array);
    assert_eq!(
        t.get_ids_of_names(&["Scale", "factor", "V"]).unwrap(),
        vec![DispId(2), DispId(1), DispId(0)]
    );
    assert_eq!(
        t.get_ids_of_names(&["Scale", "bogus"]).unwrap_err(),
        DispError::UNKNOWN_NAME
    );
    assert!(t.accepts_dispid(DispId(4), "total"));
    assert!(!t.accepts_dispid(DispId(4), "Version"));
    // The class table is the table of its default interface.
    let c = p
        .interface_table(&LibraryType {
            library: "Calc".into(),
            name: "Calculator".into(),
        })
        .unwrap();
    assert_eq!(c.members().len(), t.members().len());
    // A read-only property does not accept a put.
    let mut dp = DispParams::new();
    dp.named.push((
        DispId::PROPERTYPUT,
        vba_automation::Arg::val(Variant::from("x")),
    ));
    assert_eq!(
        t.bind_args(DispId(5), InvokeFlags::PROPERTY_PUT, &dp)
            .unwrap_err(),
        DispError::MEMBER_NOT_FOUND
    );
}

#[test]
fn the_table_converts_arguments() {
    use vba_automation::Arg;
    let p = provider();
    let t = p
        .interface_table(&LibraryType {
            library: "Calc".into(),
            name: "ICalc".into(),
        })
        .unwrap();
    let get = InvokeFlags::METHOD | InvokeFlags::PROPERTY_GET;
    // Add(a, b As Long): "7" and 8.4 convert; "x" is a type mismatch.
    let mut dp = DispParams::new();
    dp.args = vec![Arg::val(Variant::from("7")), Arg::val(Variant::Double(8.4))];
    let b = t.bind_args(DispId(1), get, &dp).unwrap();
    assert_eq!(b.slots[0].value, Variant::Long(7));
    assert_eq!(b.slots[1].value, Variant::Long(8));
    dp.args[0] = Arg::val(Variant::from("x"));
    assert_eq!(number(t.bind_args(DispId(1), get, &dp).unwrap_err()), 13);
    // Too many and too few arguments.
    dp.args = vec![Arg::val(Variant::Long(1))];
    assert_eq!(
        t.bind_args(DispId(1), get, &dp).unwrap_err(),
        DispError::PARAM_NOT_OPTIONAL
    );
    dp.args = vec![Arg::val(Variant::Long(1)); 3];
    assert_eq!(
        t.bind_args(DispId(1), get, &dp).unwrap_err(),
        DispError::BAD_PARAM_COUNT
    );
    // Omitted optionals take the declared defaults.
    dp.args = vec![Arg::val(Variant::Double(1.0))];
    let b = t.bind_args(DispId(2), get, &dp).unwrap();
    assert_eq!(b.slots[1].value, Variant::Double(2.5));
    dp.args = vec![];
    let b = t.bind_args(DispId(3), get, &dp).unwrap();
    assert_eq!(b.slots[0].value, Variant::from("world"));
    // An enum parameter is converted to Long.
    dp.args = vec![Arg::val(Variant::from("2"))];
    let b = t.bind_args(DispId(7), get, &dp).unwrap();
    assert_eq!(b.slots[0].value, Variant::Long(2));
    // A scalar where an array is declared, and a rank mismatch.
    dp.args = vec![Arg::val(Variant::Long(1))];
    assert_eq!(number(t.bind_args(DispId(10), get, &dp).unwrap_err()), 13);
    let rank2 = vba_typelib::automation::coerce(
        &vba_sema::Type::Array {
            elem: Box::new(vba_sema::Type::Long),
            rank: Some(1),
        },
        &Variant::Array(Box::new(
            vba_value::VbaArray::new_fixed(
                vba_value::VarType::Long,
                &[vba_value::Bounds::new(0, 1), vba_value::Bounds::new(0, 1)],
            )
            .unwrap(),
        )),
    );
    assert_eq!(number(rank2.unwrap_err()), 9);
}

// ---- perf sanity (run with --release --ignored --nocapture) -----------------

/// A loop to time: ProgID, early type, setup statement, loop statement, host.
type Case<'a> = (&'a str, &'a str, &'a str, &'a str, fn() -> Rc<dyn Host>);

/// Best-of-N wall time of a loop of a million host calls, early and late.
fn loop_times(
    create: &str,
    early_ty: &str,
    setup: &str,
    stmt: &str,
    host: fn() -> Rc<dyn Host>,
) -> (std::time::Duration, std::time::Duration) {
    let program = |ty: &str| {
        format!(
            "Sub Main()\nDim d As {ty}\nDim i As Long\nDim n As Variant\nSet d = CreateObject(\"{create}\")\n{setup}\nFor i = 1 To 1000000\n{stmt}\nNext\nEnd Sub\n"
        )
    };
    let time = |ty: &str| {
        let start = std::time::Instant::now();
        let r = run_on(&program(ty), host());
        assert_eq!(r.error, None);
        start.elapsed()
    };
    let (mut early, mut late) = (std::time::Duration::MAX, std::time::Duration::MAX);
    for _ in 0..5 {
        early = early.min(time(early_ty));
        late = late.min(time("Object"));
    }
    (early, late)
}

#[test]
#[ignore = "timing measurement; run in release mode"]
fn early_binding_is_not_slower_than_late_binding() {
    let cases: [Case; 3] = [
        (
            "Scripting.Dictionary",
            "Scripting.Dictionary",
            "d.Add \"a\", 1",
            "n = d.Count",
            scripting_host,
        ),
        (
            "Scripting.Dictionary",
            "Scripting.Dictionary",
            "d.Add \"a\", 1",
            "n = d.Exists(\"a\")",
            scripting_host,
        ),
        (
            "Calc.Calculator",
            "Calc.Calculator",
            "",
            "n = d.Add(i, 1)",
            || calc_host().0,
        ),
    ];
    for (create, ty, setup, stmt, host) in cases {
        let (early, late) = loop_times(create, ty, setup, stmt, host);
        eprintln!(
            "1e6 x `{stmt}`  early (by id): {early:?}  late (by name): {late:?}  ratio {:.2}",
            early.as_secs_f64() / late.as_secs_f64()
        );
        assert!(
            early <= late + late / 10,
            "early binding is slower for `{stmt}`: {early:?} vs {late:?}"
        );
    }
}
