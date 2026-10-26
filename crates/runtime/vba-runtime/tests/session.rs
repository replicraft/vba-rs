//! Sessions: the trust gate, invocation by name, isolation, loading, the
//! error taxonomy and the opt-in services.

mod common;

use std::rc::Rc;

use common::*;
use vba_automation::DispError;
use vba_automation::testing::MemObject;
use vba_host::{Capability, HeadlessHost, Policy, ResourceLimits, ScriptedHost};
use vba_project::{Module, ModuleKind, VbaProject};
use vba_runtime::*;
use vba_value::{ObjectRef, Variant};
use vba_vm::{BufferSink, LimitKind, RunStatus};

fn build(mods: &[vba_sema::SourceModule]) -> (Session, BufferSink) {
    let sink = BufferSink::new();
    let s = Session::builder()
        .debug_sink(sink.clone())
        .build_sources(mods)
        .unwrap_or_else(|e| panic!("{e}"));
    (s, sink)
}

#[test]
fn building_a_session_runs_nothing_and_execution_needs_trust() {
    let (mut s, sink) = build(&[
        std_module(
            "Dim g As Long\nPublic Sub Auto_Open()\nDebug.Print \"auto\"\nEnd Sub\nPublic Sub Hello()\nDebug.Print \"hello\"\nEnd Sub\n",
        ),
        module(
            "ThisWorkbook",
            ModuleKind::Document,
            "Private Sub Class_Initialize()\nDebug.Print \"doc init\"\nEnd Sub\nPrivate Sub Workbook_Open()\nDebug.Print \"opened\"\nEnd Sub\n",
        ),
    ]);
    assert_eq!(sink.contents(), "", "loading must not run user code");
    assert!(!s.is_trusted());
    assert_eq!(s.run_procedure("Hello", vec![]), Err(Error::NotTrusted));
    assert_eq!(
        s.raise_event("Workbook", "Open", vec![]),
        Err(Error::NotTrusted)
    );
    assert_eq!(s.shutdown(), Err(Error::NotTrusted));
    assert_eq!(sink.contents(), "");
    s.trust();
    s.run_procedure("hello", vec![]).unwrap();
    assert_eq!(sink.contents(), "hello\n");
}

#[test]
fn procedures_run_by_name_like_application_run() {
    let (mut s, sink) = build(&[
        module(
            "Mod1",
            ModuleKind::Standard,
            "Public Function Add(a As Long, Optional b As Long = 10) As Long\nAdd = a + b\nEnd Function\nPublic Sub Same()\nDebug.Print \"mod1\"\nEnd Sub\nPrivate Sub Hidden()\nEnd Sub\nPublic Property Get Prop() As Long\nEnd Property\n",
        ),
        module(
            "Mod2",
            ModuleKind::Standard,
            "Public Sub Same()\nDebug.Print \"mod2\"\nEnd Sub\n",
        ),
        module("Cls", ModuleKind::Class, "Public Sub Method()\nEnd Sub\n"),
    ]);
    s.trust();
    let r = s
        .run_procedure("MOD1.add", vec![Variant::Long(1), Variant::Long(2)])
        .unwrap();
    assert_eq!(
        (r.value, r.status),
        (Variant::Long(3), RunStatus::Completed)
    );
    let r = s.run_procedure("add", vec![Variant::Long(5)]).unwrap();
    assert_eq!(r.value, Variant::Long(15));
    assert_eq!(
        s.run_procedure("Same", vec![]),
        Err(Error::AmbiguousProcedure("Same".into()))
    );
    s.run_procedure("Mod2.Same", vec![]).unwrap();
    assert_eq!(sink.contents(), "mod2\n");
    assert!(matches!(
        s.run_procedure("Mod1.Hidden", vec![]),
        Err(Error::NotCallable(_, "it is private"))
    ));
    assert!(matches!(
        s.run_procedure("Cls.Method", vec![]),
        Err(Error::NotCallable(..))
    ));
    assert_eq!(
        s.run_procedure("Nope", vec![]),
        Err(Error::NoSuchProcedure("Nope".into()))
    );
    assert_eq!(
        s.run_procedure("Mod1.Prop", vec![]),
        Err(Error::NoSuchProcedure("Mod1.Prop".into())),
        "properties do not run by name"
    );
    let names: Vec<String> = s
        .procedures()
        .iter()
        .map(|p| format!("{}.{}", p.module, p.name))
        .collect();
    assert!(names.contains(&"Mod1.Add".to_owned()) && names.contains(&"Cls.Method".to_owned()));
}

#[test]
fn module_state_is_isolated_per_session() {
    let mods = [std_module(
        "Dim n As Long\nPublic Sub Bump()\nn = n + 1\nDebug.Print n\nEnd Sub\n",
    )];
    let (mut a, sa) = build(&mods);
    let (mut b, sb) = build(&mods);
    a.trust();
    b.trust();
    a.run_procedure("Bump", vec![]).unwrap();
    a.run_procedure("Bump", vec![]).unwrap();
    b.run_procedure("Bump", vec![]).unwrap();
    assert_eq!(sa.contents(), " 1 \n 2 \n");
    assert_eq!(sb.contents(), " 1 \n");
}

#[test]
fn default_instances_are_per_session_and_events_reach_document_modules() {
    let mods = [
        module(
            "ThisWorkbook",
            ModuleKind::Document,
            "Private Sub Workbook_Open()\nDebug.Print \"opened \" & Counter\nCounter = Counter + 1\nEnd Sub\nPublic Counter As Long\n",
        ),
        std_module("Public Sub Main()\nDebug.Print \"main\"\nEnd Sub\n"),
    ];
    let (mut a, sa) = build(&mods);
    let (mut b, sb) = build(&mods);
    a.trust();
    b.trust();
    assert!(a.raise_event("Workbook", "Open", vec![]).unwrap().is_some());
    assert!(a.raise_event("workbook", "OPEN", vec![]).unwrap().is_some());
    assert!(b.raise_event("Workbook", "Open", vec![]).unwrap().is_some());
    assert_eq!(a.raise_event("Workbook", "Close", vec![]).unwrap(), None);
    assert_eq!(sa.contents(), "opened 0\nopened 1\n");
    assert_eq!(sb.contents(), "opened 0\n");
}

#[test]
fn load_errors_carry_rendered_diagnostics() {
    let err = Session::builder()
        .build_sources(&[std_module("Option Explicit\nSub Main()\nx = 1\nEnd Sub\n")])
        .err()
        .expect("load must fail");
    match err {
        Error::Load(l) => {
            assert!(!l.diagnostics.is_empty());
            assert!(l.diagnostics[0].contains("error"), "{:?}", l.diagnostics);
        }
        other => panic!("{other:?}"),
    }
    let err = Session::builder()
        .build_sources(&[std_module("Sub Main(\n")])
        .err()
        .expect("syntax errors must fail");
    assert_eq!(err.kind(), "load");
}

#[test]
fn projects_load_with_conditional_compilation_constants() {
    let mut p = VbaProject::new("Demo");
    p.constants = "FLAG = 1".into();
    p.modules.push(Module::new(
        "Main",
        ModuleKind::Standard,
        "Public Sub Run()\n#If FLAG Then\nDebug.Print \"flag on\"\n#Else\nDebug.Print \"flag off\"\n#End If\nEnd Sub\n",
    ));
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .debug_sink(sink.clone())
        .build_project(&p)
        .unwrap();
    s.trust();
    s.run_procedure("Main.Run", vec![]).unwrap();
    assert_eq!(sink.contents(), "flag on\n");
    // Err.Source defaults to the project name.
    let mut p2 = VbaProject::new("MyProject");
    p2.modules.push(Module::new(
        "M",
        ModuleKind::Standard,
        "Public Sub Run()\nOn Error Resume Next\nErr.Raise 9\nDebug.Print Err.Source\nEnd Sub\n",
    ));
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .debug_sink(sink.clone())
        .build_project(&p2)
        .unwrap();
    s.trust();
    s.run_procedure("Run", vec![]).unwrap();
    assert_eq!(sink.contents(), "MyProject\n");
}

fn first_error(src: &str, session: SessionBuilder) -> Error {
    let mut s = session.build_sources(&[std_module(src)]).unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).expect_err("must fail")
}

#[test]
fn error_taxonomy_separates_who_is_responsible() {
    // The VBA program.
    let e = first_error(
        "Public Sub Main()\nErr.Raise 1234, \"S\", \"boom\"\nEnd Sub\n",
        Session::builder(),
    );
    match &e {
        Error::Vba(r) => {
            assert_eq!(
                (r.number, r.source.as_str(), r.description.as_str()),
                (1234, "S", "boom")
            );
            assert_eq!(r.procedure, "M.Main");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(e.kind(), "vba");
    // The language.
    let e = first_error(
        "Public Sub Main()\nDebug.Print 1 / 0\nEnd Sub\n",
        Session::builder(),
    );
    assert!(matches!(&e, Error::Vba(r) if r.number == 11), "{e:?}");
    // The permission policy.
    let e = first_error(
        "Public Sub Main()\nOpen \"x\" For Input As #1\nEnd Sub\n",
        Session::builder(),
    );
    assert!(
        matches!(&e, Error::PermissionDenied(r) if r.number == 70),
        "{e:?}"
    );
    // A host object.
    let host = ScriptedHost::new()
        .with_policy(Policy::deny_all().grant(Capability::ComCreate))
        .with_factory("T.Bad", |t| {
            Ok(Variant::Object(ObjectRef::Handle(t.insert_obj(
                MemObject::new("Bad").method("Fail", vec![], |_, _| {
                    Err(DispError::raise(1004, "host says no", "Bad"))
                }),
            ))))
        });
    let e = first_error(
        "Public Sub Main()\nDim o As Object\nSet o = CreateObject(\"T.Bad\")\no.Fail\nEnd Sub\n",
        Session::builder().host(Rc::new(host)),
    );
    assert!(
        matches!(&e, Error::Host(r) if r.number == 1004 && r.source == "Bad"),
        "{e:?}"
    );
    // A resource limit.
    let limited = HeadlessHost::new().with_limits(ResourceLimits {
        max_steps: Some(1000),
        ..ResourceLimits::default()
    });
    let e = first_error(
        "Public Sub Main()\nDo\nLoop\nEnd Sub\n",
        Session::builder().host(Rc::new(limited)),
    );
    assert_eq!(e, Error::Limit(LimitKind::Steps));
    // The native bridge.
    let host = HeadlessHost::new().with_policy(Policy::deny_all().grant(Capability::NativeDeclare));
    let e = first_error(
        "Private Declare PtrSafe Function GetDesktopWindow Lib \"user32\" () As LongPtr\nPublic Sub Main()\nDebug.Print GetDesktopWindow()\nEnd Sub\n",
        Session::builder().host(Rc::new(host)),
    );
    assert!(
        matches!(&e, Error::Bridge(m) if m.contains("GetDesktopWindow")),
        "{e:?}"
    );
    // Cancellation by the host.
    let host = ScriptedHost::new().cancel_after_idles(1);
    let e = first_error(
        "Public Sub Main()\nDoEvents\nDoEvents\nEnd Sub\n",
        Session::builder().host(Rc::new(host)),
    );
    assert!(matches!(e, Error::Cancelled(_)), "{e:?}");
    // A feature that is not implemented.
    let host = HeadlessHost::new().with_policy(Policy::deny_all().grant(Capability::ProcessSpawn));
    let e = first_error(
        "Public Sub Main()\nDim p As Double\np = Shell(\"echo hi\")\nEnd Sub\n",
        Session::builder().host(Rc::new(host)),
    );
    assert!(
        matches!(&e, Error::Unsupported(m) if m.contains("Shell")),
        "{e:?}"
    );
}

#[test]
fn errors_stay_trappable_except_limits_bridge_and_unsupported() {
    let o = run(
        "Sub Main()\nOn Error Resume Next\nOpen \"x\" For Input As #1\nDebug.Print Err.Number\nEnd Sub\n",
        SharedFs::default(),
        vba_host::Policy::deny_all(),
    );
    assert!(o.result.is_ok());
    assert_eq!(o.out, " 70 \n");
}

struct Recorder(Rc<std::cell::RefCell<Vec<String>>>);

impl ShellHook for Recorder {
    fn spawn(&mut self, cmd: &str, style: i16) -> Result<u32, FsError> {
        self.0.borrow_mut().push(format!("{cmd}|{style}"));
        Ok(4242)
    }
}

#[test]
fn a_program_grant_does_not_admit_other_programs() {
    let log = Rc::new(std::cell::RefCell::new(Vec::new()));
    let src = "Public Sub Main()\nOn Error Resume Next\nDim p As Double\nShell \"notepad-x.exe\"\nDebug.Print \"x\" & Err.Number\nErr.Clear\nShell \"evil.exe notepad\"\nDebug.Print \"y\" & Err.Number\nErr.Clear\nShell \"notepad.exe a.txt & evil.exe\"\nDebug.Print \"z\" & Err.Number\nEnd Sub\n";
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .host(Rc::new(HeadlessHost::new().with_policy(
            Policy::deny_all().grant_prefixes(Capability::ProcessSpawn, ["notepad"]),
        )))
        .debug_sink(sink.clone())
        .shell(Recorder(log.clone()))
        .build_sources(&[std_module(src)])
        .unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    assert_eq!(sink.contents(), "x70\ny70\nz0\n");
    // The last line starts notepad (with arguments the launcher does not
    // interpret); the first two never reach the launcher.
    assert_eq!(*log.borrow(), ["notepad.exe a.txt & evil.exe|1"]);
}

#[test]
fn shell_needs_permission_and_a_launcher() {
    let log = Rc::new(std::cell::RefCell::new(Vec::new()));
    let src = "Public Sub Main()\nOn Error Resume Next\nDim p As Double\np = Shell(\"notepad a.txt\", 2)\nDebug.Print Err.Number & \"/\" & p\nEnd Sub\n";
    // Denied by default, even with a launcher installed.
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .debug_sink(sink.clone())
        .shell(Recorder(log.clone()))
        .build_sources(&[std_module(src)])
        .unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    assert_eq!(sink.contents(), "70/0\n");
    assert!(log.borrow().is_empty());
    // Granted and installed.
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .host(Rc::new(HeadlessHost::new().with_policy(
            Policy::deny_all().grant_prefixes(Capability::ProcessSpawn, ["notepad"]),
        )))
        .debug_sink(sink.clone())
        .shell(Recorder(log.clone()))
        .build_sources(&[std_module(src)])
        .unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    assert_eq!(sink.contents(), "0/4242\n");
    assert_eq!(*log.borrow(), ["notepad a.txt|2"]);
}

#[cfg(unix)]
#[test]
fn std_shell_starts_a_real_process_when_installed_and_granted() {
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .host(Rc::new(HeadlessHost::new().with_policy(
            Policy::deny_all().grant(Capability::ProcessSpawn),
        )))
        .debug_sink(sink.clone())
        .shell(StdShell)
        .build_sources(&[std_module(
            "Public Sub Main()\nDebug.Print Shell(\"true\") > 0\nEnd Sub\n",
        )])
        .unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    assert_eq!(sink.contents(), "True\n");
}

#[test]
fn clock_environment_and_command_are_injected() {
    let mut host = vba_stdlib::FixedHost::default();
    host.env.insert("HOME".into(), "/home/test".into());
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .std_host(host)
        .command_line("/x /y")
        .debug_sink(sink.clone())
        .build_sources(&[std_module(
            "Public Sub Main()\nDebug.Print Environ(\"HOME\") & \"|\" & Environ(\"NOPE\") & \"|\" & Command & \"|\" & Format(Date, \"yyyy-mm-dd\")\nEnd Sub\n",
        )])
        .unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    assert_eq!(sink.contents(), "/home/test||/x /y|2000-01-01\n");
}

#[cfg(unix)]
#[test]
fn system_host_reads_the_real_environment_only_when_installed() {
    let src = std_module("Public Sub Main()\nDebug.Print Len(Environ(\"PATH\")) > 0\nEnd Sub\n");
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .std_host(SystemHost::new())
        .debug_sink(sink.clone())
        .build_sources(std::slice::from_ref(&src))
        .unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    assert_eq!(sink.contents(), "True\n");
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .debug_sink(sink.clone())
        .build_sources(&[src])
        .unwrap();
    s.trust();
    s.run_procedure("Main", vec![]).unwrap();
    assert_eq!(
        sink.contents(),
        "False\n",
        "the default environment is empty"
    );
}

#[test]
fn shutdown_terminates_module_objects_and_flushes_files() {
    let fs = SharedFs::default();
    let mods = [
        module(
            "Res",
            ModuleKind::Class,
            "Public Tag As String\nPrivate Sub Class_Terminate()\nDebug.Print \"term \" & Tag\nEnd Sub\n",
        ),
        std_module(
            "Public A As Res\nPublic B As Res\nPublic Sub Setup()\nSet A = New Res\nA.Tag = \"A\"\nSet B = New Res\nB.Tag = \"B\"\nOpen \"keep.txt\" For Output As #1\nPrint #1, \"data\"\nEnd Sub\n",
        ),
    ];
    let sink = BufferSink::new();
    let mut s = Session::builder()
        .host(host(all_files()))
        .vfs(fs.clone())
        .debug_sink(sink.clone())
        .build_sources(&mods)
        .unwrap();
    s.trust();
    s.run_procedure("Setup", vec![]).unwrap();
    assert_eq!(fs.text("/keep.txt").as_deref(), Some(""), "still buffered");
    s.shutdown().unwrap();
    assert_eq!(sink.contents(), "term A\nterm B\n");
    assert_eq!(fs.text("/keep.txt").as_deref(), Some("data\r\n"));
    assert_eq!(s.vm().live_objects(), 0);
}
