//! Programs that reach outside the interpreter: host objects, permissions,
//! UI, `Declare`, limits, cancellation and the `Stop`/`End` statuses.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::*;
use vba_automation::testing::MemObject;
use vba_automation::{
    Arg as AutoArg, ConnectionPoint, DispError, DispId, DispParams, Dispatch, EventSource,
    InvokeFlags, MemberInfo, ObjectTable, ParamInfo,
};
use vba_host::{Capability, HeadlessHost, Host, Policy, ResourceLimits, ScriptedHost};
use vba_project::ModuleKind;
use vba_value::{ObjectRef, Variant};
use vba_vm::{LimitKind, NoServices, RunStatus, VmError};

fn run_host(src: &str, host: impl Host + 'static) -> Outcome {
    run_with(
        &[module("M", ModuleKind::Standard, src)],
        "Main",
        vec![],
        Rc::new(host),
        Box::new(NoServices),
    )
}

fn object(table: &ObjectTable, o: MemObject) -> Variant {
    Variant::Object(ObjectRef::Handle(table.insert_obj(o)))
}

#[test]
fn create_object_is_denied_by_default() {
    let o = run_host(
        "Sub Main()\nOn Error Resume Next\nDim o As Object\nSet o = CreateObject(\"Test.Thing\")\nDebug.Print Err.Number\nDebug.Print Err.Source\nEnd Sub\n",
        HeadlessHost::new(),
    );
    assert!(o.result.is_ok());
    assert!(o.out.starts_with(" 70 \n"), "{}", o.out);
    assert!(o.out.contains("COM object creation denied"), "{}", o.out);
}

#[test]
fn create_object_without_a_factory_is_error_429() {
    let host = HeadlessHost::new().with_policy(Policy::deny_all().grant(Capability::ComCreate));
    let o = run_host(
        "Sub Main()\nOn Error Resume Next\nDim o As Object\nSet o = CreateObject(\"Test.Thing\")\nDebug.Print Err.Number\nEnd Sub\n",
        host,
    );
    assert_eq!(o.out, " 429 \n");
}

#[test]
fn create_object_grants_are_scoped_by_prefix() {
    let host = ScriptedHost::new()
        .with_policy(Policy::deny_all().grant_prefixes(Capability::ComCreate, ["Test."]))
        .with_factory("Test.Thing", |t| {
            Ok(object(
                t,
                MemObject::new("Thing").property("Name", Variant::from("t")),
            ))
        })
        .with_factory("Other.Thing", |t| Ok(object(t, MemObject::new("Other"))));
    let o = run_host(
        "Sub Main()\nOn Error Resume Next\nDim a As Object, b As Object\nSet a = CreateObject(\"Test.Thing\")\nDebug.Print a.Name\nSet b = CreateObject(\"Other.Thing\")\nDebug.Print Err.Number\nEnd Sub\n",
        host,
    );
    assert_eq!(o.out, "t\n 70 \n");
}

fn allow_com() -> Policy {
    Policy::deny_all().grant(Capability::ComCreate)
}

#[test]
fn late_bound_properties_methods_and_byref() {
    let host = ScriptedHost::new()
        .with_policy(allow_com())
        .with_factory("Test.Thing", |t| {
            let o = MemObject::new("Thing")
                .property("Count", Variant::Long(1))
                .method(
                    "Add",
                    vec![ParamInfo::required("a"), ParamInfo::required("b")],
                    |_, c| {
                        let n = |v: &Variant| match v {
                            Variant::Long(n) => Some(*n),
                            Variant::Integer(n) => Some(i32::from(*n)),
                            _ => None,
                        };
                        match (n(&c.args[0].value), n(&c.args[1].value)) {
                            (Some(a), Some(b)) => Ok(Variant::Long(a + b)),
                            _ => Err(DispError::TYPE_MISMATCH),
                        }
                    },
                )
                .method("Bump", vec![ParamInfo::required("x")], |_, c| {
                    if let Variant::Long(n) = c.args[0].value {
                        c.args[0].value = Variant::Long(n + 100);
                    }
                    Ok(Variant::Empty)
                });
            Ok(object(t, o))
        });
    let o = run_host(
        "Sub Main()\nDim o As Object, n As Long\nSet o = CreateObject(\"Test.Thing\")\no.Count = o.Count + 4\nDebug.Print o.Count\nDebug.Print o.Add(2, 3)\nn = 1\no.Bump n\nDebug.Print n\no.Bump (n)\nDebug.Print n\nEnd Sub\n",
        host,
    );
    assert_eq!(o.out, " 5 \n 5 \n 101 \n 101 \n", "{:?}", o.result);
}

#[test]
fn for_each_and_default_member_on_a_host_collection() {
    let host = ScriptedHost::new()
        .with_policy(allow_com())
        .with_factory("Test.List", |t| {
            Ok(object(
                t,
                MemObject::new("List").collection(vec![
                    Variant::from("a"),
                    Variant::from("b"),
                    Variant::from("c"),
                ]),
            ))
        });
    let o = run_host(
        "Sub Main()\nDim o As Object, v As Variant, s As String\nSet o = CreateObject(\"Test.List\")\nFor Each v In o\ns = s & v\nNext\nDebug.Print s & o.Count & o(2) & o.Item(3)\nEnd Sub\n",
        host,
    );
    assert_eq!(o.out, "abc3bc\n", "{:?}", o.result);
}

#[test]
fn host_errors_are_vba_errors_with_source_and_description() {
    let host = ScriptedHost::new()
        .with_policy(allow_com())
        .with_factory("Test.Bad", |t| {
            Ok(object(
                t,
                MemObject::new("Bad").method("Fail", vec![], |_, _| {
                    Err(DispError::raise(1004, "disk on fire", "Bad.Component"))
                }),
            ))
        });
    let o = run_host(
        "Sub Main()\nDim o As Object\nSet o = CreateObject(\"Test.Bad\")\nOn Error Resume Next\no.Fail\nDebug.Print Err.Number & \"|\" & Err.Source & \"|\" & Err.Description\nErr.Clear\no.Missing\nDebug.Print Err.Number\nEnd Sub\n",
        host,
    );
    assert_eq!(
        o.out, "1004|Bad.Component|disk on fire\n 438 \n",
        "{:?}",
        o.result
    );
}

#[test]
fn host_objects_are_released_with_their_references() {
    let log: vba_automation::testing::TerminateLog = Rc::new(RefCell::new(Vec::new()));
    let l2 = log.clone();
    let host = ScriptedHost::new()
        .with_policy(allow_com())
        .with_factory("Test.Thing", move |t| {
            Ok(object(t, MemObject::new("Thing").log_terminate(&l2)))
        });
    let o = run_host(
        "Sub Work()\nDim o As Object\nSet o = CreateObject(\"Test.Thing\")\nDebug.Print \"in\"\nEnd Sub\nSub Main()\nWork\nDebug.Print \"out\"\nEnd Sub\n",
        host,
    );
    assert_eq!(o.out, "in\nout\n");
    assert_eq!(log.borrow().len(), 1, "the host object must be terminated");
}

#[test]
fn host_globals_resolve_through_the_host() {
    use vba_sema::{LibGlobal, LibMember, LibraryType, StaticLibrary, Type};
    let lib = StaticLibrary::new("Lib")
        .with_type("App", vec![LibMember::property("Name", Type::String)])
        .with_global(
            "Application",
            LibGlobal::Object(Type::Library(LibraryType {
                library: "Lib".into(),
                name: "App".into(),
            })),
        );
    let host = ScriptedHost::new();
    let app = host
        .objects()
        .insert_obj(MemObject::new("App").property("Name", Variant::from("TestApp")));
    let host = host.with_global("Application", Variant::Object(ObjectRef::Handle(app)));
    let mods = [module(
        "M",
        ModuleKind::Standard,
        "Sub Main()\nDebug.Print Application.Name\nApplication.Name = \"Renamed\"\nDebug.Print Application.Name\nEnd Sub\n",
    )];
    let o = run_full(
        &mods,
        "Main",
        vec![],
        Rc::new(host),
        Box::new(NoServices),
        &lib,
    );
    assert_eq!(o.out, "TestApp\nRenamed\n", "{:?}", o.result);
}

#[test]
fn ui_goes_through_the_host() {
    let host = ScriptedHost::new()
        .with_msg_box_answers([6])
        .with_input_box_answers([Some("typed".to_owned()), None]);
    let o = run_host(
        "Sub Main()\nDim r As Long\nr = MsgBox(\"Sure?\", vbYesNo)\nDebug.Print r\nDebug.Print InputBox(\"Name?\")\nDebug.Print \"[\" & InputBox(\"Again?\") & \"]\"\nEnd Sub\n",
        host,
    );
    assert_eq!(o.out, " 6 \ntyped\n[]\n", "{:?}", o.result);
}

#[test]
fn headless_ui_is_error_70() {
    let o =
        run("Sub Main()\nOn Error Resume Next\nMsgBox \"x\"\nDebug.Print Err.Number\nEnd Sub\n");
    assert_eq!(o.out, " 70 \n");
}

struct Emitter {
    cp: ConnectionPoint,
}

impl Dispatch for Emitter {
    fn class_name(&self) -> String {
        "Emitter".into()
    }
    fn member_id(&self, name: &str) -> Option<DispId> {
        name.eq_ignore_ascii_case("Fire").then_some(DispId(1))
    }
    fn member_info(&self, id: DispId) -> Option<MemberInfo> {
        (id == DispId(1)).then(|| MemberInfo {
            name: "Fire".into(),
            id,
            supports: InvokeFlags::METHOD,
            params: vec![ParamInfo::required("v")],
        })
    }
    fn invoke(
        &self,
        table: &ObjectTable,
        _: DispId,
        _: InvokeFlags,
        params: &mut DispParams,
    ) -> Result<Variant, DispError> {
        let mut args: Vec<AutoArg> = params.args.clone();
        self.cp.fire(table, "Changed", &mut args)?;
        Ok(Variant::Empty)
    }
    fn event_source(&self) -> Option<&dyn EventSource> {
        Some(&self.cp)
    }
}

#[test]
fn events_from_host_objects_are_delivered_after_the_call() {
    use vba_sema::{LibEvent, LibMember, LibParam, StaticLibrary, Type};
    let host = ScriptedHost::new()
        .with_policy(allow_com())
        .with_factory("Lib.Emitter", |t| {
            Ok(Variant::Object(ObjectRef::Handle(t.insert_obj(Emitter {
                cp: ConnectionPoint::new("EmitterEvents"),
            }))))
        });
    let lib = StaticLibrary::new("Lib")
        .with_type(
            "Emitter",
            vec![LibMember::sub(
                "Fire",
                vec![LibParam::required("v", Type::Long)],
            )],
        )
        .with_events(
            "Emitter",
            vec![LibEvent {
                name: "Changed".into(),
                params: vec![LibParam::required("v", Type::Long)],
            }],
        );
    let mods = [
        module(
            "Watcher",
            ModuleKind::Class,
            "Private WithEvents src As Lib.Emitter\nPublic Sub Attach(s As Lib.Emitter)\nSet src = s\nEnd Sub\nPrivate Sub src_Changed(ByVal v As Long)\nDebug.Print \"changed \" & v\nEnd Sub\n",
        ),
        module(
            "M",
            ModuleKind::Standard,
            "Sub Main()\nDim e As Lib.Emitter, w As New Watcher\nSet e = CreateObject(\"Lib.Emitter\")\nw.Attach e\ne.Fire 7\nDebug.Print \"after\"\nSet w = Nothing\ne.Fire 8\nDebug.Print \"end\"\nEnd Sub\n",
        ),
    ];
    let o = run_full(
        &mods,
        "Main",
        vec![],
        Rc::new(host),
        Box::new(NoServices),
        &lib,
    );
    assert!(o.result.is_ok(), "{:?}", o.result);
    // The event is queued while the host call runs and delivered right after
    // it returns; a destroyed sink is unadvised.
    assert_eq!(o.out, "changed 7\nafter\nend\n");
}

#[test]
fn declare_needs_native_declare_permission() {
    let src = "Private Declare PtrSafe Function GetTickCount Lib \"kernel32\" () As Long\nSub Main()\nOn Error Resume Next\nDim t As Long\nt = GetTickCount()\nDebug.Print Err.Number\nEnd Sub\n";
    let o = run_host(src, HeadlessHost::new());
    assert_eq!(o.out, " 70 \n", "{:?}", o.result);
    let host = HeadlessHost::new().with_policy(Policy::deny_all().grant(Capability::NativeDeclare));
    let o = run_host(
        "Private Declare PtrSafe Function GetTickCount Lib \"kernel32\" () As Long\nSub Main()\nDim t As Long\nt = GetTickCount()\nDebug.Print t\nEnd Sub\n",
        host,
    );
    assert!(o.result.is_ok(), "{:?}", o.result);
    assert_eq!(o.out, " 0 \n");
}

#[test]
fn a_scoped_declare_grant_cannot_be_smuggled_into_another_library() {
    // Granted: every procedure of user32. The library string below starts
    // with it but names kernel32 once the directory is dropped.
    let src = "Private Declare PtrSafe Function Smuggled Lib \"user32!/kernel32\" Alias \"GetTickCount\" () As Long\nPrivate Declare PtrSafe Function Plain Lib \"kernel32\" Alias \"GetTickCount\" () As Long\nSub Main()\nOn Error Resume Next\nDim t As Long\nt = Smuggled()\nDebug.Print \"smuggled \" & Err.Number\nErr.Clear\nt = Plain()\nDebug.Print \"plain \" & Err.Number\nEnd Sub\n";
    let host = HeadlessHost::new()
        .with_policy(Policy::deny_all().grant_prefixes(Capability::NativeDeclare, ["user32!*"]));
    let o = run_host(src, host);
    assert_eq!(o.out, "smuggled 70\nplain 70\n", "{:?}", o.result);
    // A grant for the library actually called works with the .dll spelling.
    let host = HeadlessHost::new()
        .with_policy(Policy::deny_all().grant_prefixes(Capability::NativeDeclare, ["kernel32!*"]));
    let src = "Private Declare PtrSafe Function Tick Lib \"kernel32.dll\" Alias \"GetTickCount\" () As Long\nSub Main()\nOn Error Resume Next\nDim t As Long\nt = Tick()\nDebug.Print Err.Number\nEnd Sub\n";
    let o = run_host(src, host);
    assert_eq!(o.out, " 0 \n", "{:?}", o.result);
}

#[test]
fn last_dll_error_follows_declare_calls() {
    let host = HeadlessHost::new().with_policy(Policy::deny_all().grant(Capability::NativeDeclare));
    let o = run_host(
        "Private Declare PtrSafe Sub SetLastError Lib \"kernel32\" (ByVal code As Long)\nSub Main()\nDebug.Print Err.LastDllError\nSetLastError 1234\nDebug.Print Err.LastDllError\nEnd Sub\n",
        host,
    );
    assert_eq!(o.out, " 0 \n 1234 \n", "{:?}", o.result);
}

#[test]
fn unimplemented_declare_is_an_explicit_unsupported_error() {
    let host = HeadlessHost::new().with_policy(Policy::deny_all().grant(Capability::NativeDeclare));
    let o = run_host(
        "Private Declare PtrSafe Function MessageBoxA Lib \"user32\" (ByVal h As LongPtr, ByVal t As String, ByVal c As String, ByVal f As Long) As Long\nSub Main()\nOn Error Resume Next\nDebug.Print MessageBoxA(0, \"a\", \"b\", 0)\nDebug.Print \"trapped?\"\nEnd Sub\n",
        host,
    );
    match &o.result {
        Err(VmError::Bridge(m)) => assert!(m.contains("MessageBoxA"), "{m}"),
        other => panic!("expected a bridge error, got {other:?}"),
    }
    assert_eq!(o.out, "", "an unsupported call must not be trappable");
}

#[test]
fn declare_hook_of_a_scripted_host_is_used_when_no_provider_exists() {
    let host = ScriptedHost::new()
        .with_policy(Policy::deny_all().grant(Capability::NativeDeclare))
        .with_declare("mylib", "Twice", |req| match &req.args[0].value {
            Variant::Long(n) => Ok(Variant::Long(n * 2)),
            _ => Err(vba_automation::ErrInfo::standard(13)),
        });
    let o = run_host(
        "Private Declare PtrSafe Function Twice Lib \"mylib\" (ByVal n As Long) As Long\nSub Main()\nDebug.Print Twice(21)\nEnd Sub\n",
        host,
    );
    assert_eq!(o.out, " 42 \n", "{:?}", o.result);
}

#[test]
fn declare_string_buffers_are_written_back() {
    let host = HeadlessHost::new().with_policy(Policy::deny_all().grant(Capability::NativeDeclare));
    let o = run_host(
        "Private Declare PtrSafe Function GetUserNameA Lib \"advapi32\" (ByVal buf As String, ByRef n As Long) As Long\nSub Main()\nDim b As String, n As Long, ok As Long\nb = Space(32)\nn = 32\nok = GetUserNameA(b, n)\nDebug.Print ok & \" \" & Left(b, n - 1)\nEnd Sub\n",
        host,
    );
    assert!(o.result.is_ok(), "{:?}", o.result);
    assert_eq!(o.out, "1 user\n");
}

#[test]
fn step_limit_stops_runaway_loops_and_is_not_trappable() {
    let o = run_limited(
        "Sub Main()\nOn Error Resume Next\nDo\nLoop\nDebug.Print \"never\"\nEnd Sub\n",
        ResourceLimits {
            max_steps: Some(10_000),
            ..ResourceLimits::default()
        },
    );
    assert_eq!(o.result, Err(VmError::Limit(LimitKind::Steps)));
    assert_eq!(o.out, "");
}

#[test]
fn call_depth_limit_stops_runaway_recursion() {
    let o = run_limited(
        "Sub Down(n As Long)\nDown n + 1\nEnd Sub\nSub Main()\nOn Error Resume Next\nDown 0\nEnd Sub\n",
        ResourceLimits {
            max_call_depth: 50,
            ..ResourceLimits::default()
        },
    );
    assert_eq!(o.result, Err(VmError::Limit(LimitKind::CallDepth)));
}

#[test]
fn memory_limit_is_charged_by_arrays() {
    let o = run_limited(
        "Sub Main()\nDim a() As Long\nReDim a(1 To 100000)\nDebug.Print \"allocated\"\nEnd Sub\n",
        ResourceLimits {
            max_memory_bytes: Some(1_000_000),
            ..ResourceLimits::default()
        },
    );
    assert_eq!(o.result, Err(VmError::Limit(LimitKind::Memory)));
}

#[test]
fn collection_growth_is_charged_to_the_memory_limit() {
    let o = run_limited(
        "Sub Main()\nDim c As New Collection, i As Long\nFor i = 1 To 100000\nc.Add i\nNext\nDebug.Print \"allocated\"\nEnd Sub\n",
        ResourceLimits {
            max_steps: Some(10_000_000),
            max_memory_bytes: Some(100_000),
            ..ResourceLimits::default()
        },
    );
    assert_eq!(o.result, Err(VmError::Limit(LimitKind::Memory)));
}

#[test]
fn strings_held_by_a_program_are_charged_to_the_memory_limit() {
    let o = run_limited(
        "Sub Main()\nDim a(1 To 8) As String, i As Long\nFor i = 1 To 8\na(i) = String$(1000000, \"x\")\nNext\nDebug.Print \"held\"\nEnd Sub\n",
        ResourceLimits {
            max_steps: Some(1_000_000),
            max_memory_bytes: Some(100_000),
            ..ResourceLimits::default()
        },
    );
    assert_eq!(o.result, Err(VmError::Limit(LimitKind::Memory)));
    assert_eq!(o.out, "");
}

#[test]
fn oversized_arrays_and_strings_are_refused() {
    let o = run(
        "Sub Main()\nOn Error Resume Next\nDim a() As Long\nReDim a(1 To 100000000)\nDebug.Print Err.Number\nEnd Sub\n",
    );
    assert_eq!(o.out, " 7 \n", "{:?}", o.result);
    let o = run(
        "Sub Main()\nDim s As String\ns = Space(1000)\nDim i As Long\nFor i = 1 To 40\ns = s & s\nNext\nEnd Sub\n",
    );
    assert_eq!(o.result, Err(VmError::Limit(LimitKind::StringLength)));
}

#[test]
fn do_events_runs_the_idle_hook_and_can_cancel() {
    let host = ScriptedHost::new()
        .with_limits(ResourceLimits {
            max_steps: Some(1_000_000),
            ..ResourceLimits::default()
        })
        .cancel_after_idles(3);
    let o = run_host(
        "Sub Main()\nDim i As Long\nFor i = 1 To 10\nDoEvents\nDebug.Print i\nNext\nEnd Sub\n",
        host,
    );
    assert!(
        matches!(o.result, Err(VmError::Cancelled(_))),
        "{:?}",
        o.result
    );
    assert_eq!(o.out, " 1 \n 2 \n");
}

#[test]
fn stop_and_failed_assert_halt_the_run() {
    let o = run("Sub Main()\nDebug.Print 1\nStop\nDebug.Print 2\nEnd Sub\n");
    assert_eq!(o.result.as_ref().unwrap().status, RunStatus::Stopped);
    assert_eq!(o.out, " 1 \n");
    let o = run(
        "Sub Main()\nDebug.Assert 1 = 1\nDebug.Print 1\nDebug.Assert 1 = 2\nDebug.Print 2\nEnd Sub\n",
    );
    assert_eq!(o.result.as_ref().unwrap().status, RunStatus::Stopped);
    assert_eq!(o.out, " 1 \n");
    assert_eq!(o.vm.stop_reason(), Some("Debug.Assert failed"));
}

#[test]
fn end_resets_module_state_and_reports_ended() {
    let mods = [module(
        "M",
        ModuleKind::Standard,
        "Dim g As Long\nSub Main()\ng = 5\nEnd\nEnd Sub\nSub Show()\nDebug.Print g\nEnd Sub\n",
    )];
    let mut o = run_with(
        &mods,
        "Main",
        vec![],
        Rc::new(HeadlessHost::new()),
        Box::new(NoServices),
    );
    assert_eq!(o.result.as_ref().unwrap().status, RunStatus::Ended);
    let show = o.vm.find_procedure("Show").unwrap();
    o.vm.run_procedure(show, vec![]).unwrap();
}

#[test]
fn unhandled_error_reports_procedure_and_line() {
    let o =
        run("Sub Inner()\n10 Dim x As Long\n20 x = 1 / 0\nEnd Sub\nSub Main()\nInner\nEnd Sub\n");
    match o.result {
        Err(VmError::Runtime(e)) => {
            assert_eq!(e.error.number, 11);
            assert_eq!(e.procedure, "M.Inner");
            assert_eq!(e.line, 20);
            assert_eq!(e.stack, vec!["M.Inner".to_owned(), "M.Main".to_owned()]);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_vm_is_usable_after_an_error() {
    let mods = [module(
        "M",
        ModuleKind::Standard,
        "Dim n As Long\nSub Boom()\nn = n + 1\nErr.Raise 5\nEnd Sub\nSub Show()\nDebug.Print n\nEnd Sub\n",
    )];
    let mut o = run_with(
        &mods,
        "Boom",
        vec![],
        Rc::new(HeadlessHost::new()),
        Box::new(NoServices),
    );
    assert!(o.result.is_err());
    let boom = o.vm.find_procedure("Boom").unwrap();
    assert!(o.vm.run_procedure(boom, vec![]).is_err());
    let show = o.vm.find_procedure("Show").unwrap();
    assert!(o.vm.run_procedure(show, vec![]).is_ok());
}

#[test]
fn runs_are_deterministic() {
    let src = "Sub Main()\nDim i As Long, s As String\nRandomize 42\nFor i = 1 To 5\ns = s & Int(Rnd * 100) & \",\"\nNext\nDebug.Print s & Format(Now, \"yyyy-mm-dd hh:nn:ss\") & Timer\nEnd Sub\n";
    let a = run(src);
    let b = run(src);
    assert!(a.result.is_ok());
    assert_eq!(a.out, b.out);
    assert!(a.out.contains("2000-01-01 00:00:00"), "{}", a.out);
}

#[test]
fn call_by_name_on_a_host_object() {
    let host = ScriptedHost::new()
        .with_policy(allow_com())
        .with_factory("Test.Thing", |t| {
            let o = MemObject::new("Thing")
                .property("Count", Variant::Long(1))
                .method(
                    "Add",
                    vec![ParamInfo::required("a"), ParamInfo::required("b")],
                    |_, c| match (&c.args[0].value, &c.args[1].value) {
                        (Variant::Integer(a), Variant::Integer(b)) => {
                            Ok(Variant::Long(i32::from(*a) + i32::from(*b)))
                        }
                        _ => Err(DispError::TYPE_MISMATCH),
                    },
                );
            Ok(object(t, o))
        });
    let o = run_host(
        "Sub Main()\nDim o As Object\nSet o = CreateObject(\"Test.Thing\")\nCallByName o, \"Count\", VbLet, 9\nDebug.Print CallByName(o, \"Count\", VbGet)\nDebug.Print CallByName(o, \"Add\", VbMethod, 2, 3)\nEnd Sub\n",
        host,
    );
    assert_eq!(o.out, " 9 \n 5 \n", "{:?}", o.result);
}

#[test]
fn a_vba_object_kept_by_a_host_object_stays_alive() {
    // The host object stores the class instance (AddRef through the table,
    // which records it for the VM-owned handle) and hands it back later.
    let host = ScriptedHost::new()
        .with_policy(allow_com())
        .with_factory("Test.Holder", |t| {
            Ok(object(
                t,
                MemObject::new("Holder").property("Kept", Variant::Empty),
            ))
        });
    let o = run_with(
        &[
            module(
                "Box",
                ModuleKind::Class,
                "Public V As Long\nPrivate Sub Class_Terminate()\nDebug.Print \"terminate\"\nEnd Sub\n",
            ),
            module(
                "M",
                ModuleKind::Standard,
                "Sub Keep(h As Object)\nDim b As New Box\nb.V = 42\nSet h.Kept = b\nEnd Sub\nSub Main()\nDim h As Object, x As Object\nSet h = CreateObject(\"Test.Holder\")\nKeep h\nDebug.Print \"kept\"\nSet x = h.Kept\nDebug.Print x.V\nSet x = Nothing\nSet h = Nothing\nDebug.Print \"done\"\nEnd Sub\n",
            ),
        ],
        "Main",
        vec![],
        Rc::new(host),
        Box::new(NoServices),
    );
    // The instance survives the end of Keep because the holder owns a
    // reference (`MemObject` never gives it back, so it terminates only
    // when the VM is dropped, after the output above).
    assert_eq!(o.out, "kept\n 42 \ndone\n", "{:?}", o.result);
}
