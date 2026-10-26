//! Tests of the facade's own contract (the conformance harness covers the
//! language).

use vba::runtime::Error as RunError;
use vba::{CancelToken, Capability, Engine, Error, ModuleKind, Project, ReadLimits, Variant};

fn project(src: &str) -> Project {
    Project::from_source("Module1", src)
}

#[test]
fn cancel_token_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync + Clone>() {}
    assert_send_sync::<CancelToken>();
}

#[test]
fn cancel_token_stops_from_another_thread() {
    let engine = Engine::new();
    let token = engine.cancel_token();
    let p = project("Public Sub Main()\n  Do\n    DoEvents\n  Loop\nEnd Sub\n");
    let handle = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(20));
        token.cancel();
    });
    let err = engine.run(&p, "Main", vec![]).unwrap_err();
    handle.join().unwrap();
    assert!(matches!(err, Error::Run(RunError::Cancelled(_))), "{err:?}");
    assert_eq!(err.kind(), "cancelled");
}

#[test]
fn a_cancelled_token_can_be_reset_and_reused() {
    let engine = Engine::new();
    let token = engine.cancel_token();
    token.cancel();
    assert!(token.is_cancelled());
    token.reset();
    let p = project("Public Sub Main()\n  DoEvents\n  Debug.Print \"ok\"\nEnd Sub\n");
    assert_eq!(engine.run(&p, "Main", vec![]).unwrap().output, "ok\n");
}

#[test]
fn nothing_runs_until_trusted() {
    let p = project("Public Sub Main()\n  Debug.Print \"ran\"\nEnd Sub\n");
    let mut inst = Engine::new().load(&p).unwrap();
    assert!(!inst.is_trusted());
    assert!(matches!(
        inst.run("Main", vec![]),
        Err(Error::Run(RunError::NotTrusted))
    ));
    assert_eq!(inst.output(), "");
    inst.trust();
    assert_eq!(inst.run("Main", vec![]).unwrap().output, "ran\n");
}

#[test]
fn class_initialize_does_not_run_before_trust() {
    let mut p = Project::new();
    p.add_module(
        "Module1",
        ModuleKind::Standard,
        "Public Sub Main()\nEnd Sub\n",
    )
    .unwrap();
    p.add_module(
        "Thing",
        ModuleKind::Class,
        "Private Sub Class_Initialize()\n  Debug.Print \"init\"\nEnd Sub\n",
    )
    .unwrap();
    let inst = Engine::new().load(&p).unwrap();
    assert_eq!(inst.output(), "");
}

#[test]
fn runs_are_deterministic() {
    let p = project(
        "Public Sub Main()\n  Randomize 3\n  Debug.Print Rnd; Rnd\n  Debug.Print Now\n  Debug.Print Timer\nEnd Sub\n",
    );
    let a = Engine::new().run(&p, "Main", vec![]).unwrap();
    let b = Engine::new().run(&p, "Main", vec![]).unwrap();
    assert_eq!(a.output, b.output);
}

#[test]
fn sessions_are_isolated() {
    let p = project("Dim n As Long\nPublic Sub Main()\n  n = n + 1\n  Debug.Print n\nEnd Sub\n");
    let engine = Engine::new();
    let mut a = engine.load(&p).unwrap();
    let mut b = engine.load(&p).unwrap();
    a.trust();
    b.trust();
    a.run("Main", vec![]).unwrap();
    a.run("Main", vec![]).unwrap();
    assert_eq!(b.run("Main", vec![]).unwrap().output, " 1 \n");
    assert_eq!(a.output(), " 1 \n 2 \n");
}

#[test]
fn analyze_reports_errors_without_running() {
    let p = project("Option Explicit\nPublic Sub Main()\n  x = 1\nEnd Sub\n");
    let report = Engine::new().analyze(&p);
    assert!(report.has_errors());
    assert!(!report.rendered().is_empty());
    assert_eq!(report.rendered().len(), report.diagnostics().len());
    assert!(report.codes().iter().any(|c| c.starts_with("SEM")));
    let err = Engine::new().load(&p).err().expect("does not compile");
    assert_eq!(err.kind(), "load");
}

#[test]
fn error_kinds_are_stable_names() {
    let engine = Engine::new();
    let p = project("Public Sub Main()\n  Dim a(1) As Long\n  a(5) = 1\nEnd Sub\n");
    assert_eq!(engine.run(&p, "Main", vec![]).unwrap_err().kind(), "vba");
    assert_eq!(
        engine.run(&p, "Nope", vec![]).unwrap_err().kind(),
        "no-such-procedure"
    );
    let q = project("Public Sub Main()\n  Open \"/x\" For Output As #1\nEnd Sub\n");
    assert_eq!(
        engine.run(&q, "Main", vec![]).unwrap_err().kind(),
        "permission-denied"
    );
    let r = project("Public Sub Main()\n  Do\n  Loop\nEnd Sub\n");
    assert_eq!(
        engine
            .max_steps(100)
            .run(&r, "Main", vec![])
            .unwrap_err()
            .kind(),
        "limit"
    );
}

#[test]
fn arguments_and_results_cross_the_facade() {
    let p =
        project("Public Function Add(a As Long, b As Long) As Long\n  Add = a + b\nEnd Function\n");
    let out = Engine::new()
        .run(&p, "Add", vec![Variant::Long(40), Variant::Long(2)])
        .unwrap();
    assert_eq!(out.value, Variant::Long(42));
}

#[test]
fn grants_are_the_only_way_to_reach_the_file_system() {
    let src = "Public Sub Main()\n  Open \"/a.txt\" For Output As #1\n  Print #1, \"hi\"\n  Close #1\n  Debug.Print FileLen(\"/a.txt\")\nEnd Sub\n";
    let p = project(src);
    assert!(Engine::new().run(&p, "Main", vec![]).is_err());
    let out = Engine::new()
        .grant(Capability::FileWrite)
        .grant(Capability::FileRead)
        .run(&p, "Main", vec![])
        .unwrap();
    assert_eq!(out.output, " 4 \n");
}

#[test]
fn seeded_files_are_visible_to_the_program() {
    let p = project(
        "Public Sub Main()\n  Dim s As String\n  Open \"/in.txt\" For Input As #1\n  Line Input #1, s\n  Debug.Print s\nEnd Sub\n",
    );
    let out = Engine::new()
        .grant(Capability::FileRead)
        .with_file("/in.txt", b"seeded\r\n")
        .run(&p, "Main", vec![])
        .unwrap();
    assert_eq!(out.output, "seeded\n");
}

#[test]
fn project_edit_round_trip_preserves_everything_else() {
    let mut p = Project::new();
    p.add_module("A", ModuleKind::Standard, "Sub A()\r\nEnd Sub\r\n")
        .unwrap();
    p.add_module("B", ModuleKind::Class, "Public X As Long\r\n")
        .unwrap();
    let bytes = p.to_bytes().unwrap();

    let mut loaded = Project::from_bytes(&bytes).unwrap();
    let before = loaded.clone();
    loaded
        .set_module_source("A", "Sub A()\r\n  Debug.Print 1\r\nEnd Sub\r\n")
        .unwrap();
    let again = Project::from_bytes(&loaded.to_bytes().unwrap()).unwrap();

    assert_eq!(again.module_source("B"), before.module_source("B"));
    assert_eq!(again.module("B").unwrap().kind, ModuleKind::Class);
    assert_eq!(again.name(), before.name());
    assert!(again.module_source("A").unwrap().contains("Debug.Print 1"));
    // Writing an untouched project reproduces the same model.
    let untouched = Project::from_bytes(&before.to_bytes().unwrap()).unwrap();
    assert_eq!(untouched.modules(), before.modules());
}

#[test]
fn unknown_module_edit_is_a_model_error() {
    let mut p = Project::new();
    let err = p.set_module_source("Nope", "x").unwrap_err();
    assert_eq!(err.kind(), "model");
    let dup = {
        p.add_module("A", ModuleKind::Standard, "").unwrap();
        p.add_module("a", ModuleKind::Standard, "").unwrap_err()
    };
    assert_eq!(dup.kind(), "model");
}

#[test]
fn hostile_container_is_a_format_error_not_a_panic() {
    let err = Project::from_bytes(b"this is not a compound file").unwrap_err();
    assert_eq!(err.kind(), "format");
    let tiny = ReadLimits {
        max_container_bytes: 8,
        ..ReadLimits::default()
    };
    assert!(Project::from_bytes_with(&[0u8; 64], &tiny).is_err());
}

#[test]
fn standalone_source_files_become_modules() {
    let mut p = Project::new();
    p.add_source_file(
        "Thing.cls",
        "VERSION 1.0 CLASS\nBEGIN\n  MultiUse = -1\nEND\nAttribute VB_Name = \"Thing\"\nPublic V As Long\n",
    )
    .unwrap();
    assert_eq!(p.module("thing").unwrap().kind, ModuleKind::Class);
}

#[test]
fn raw_layer_reaches_the_lower_crates() {
    let profile = vba::raw::profile::default_profile();
    assert_eq!(*profile, *vba::default_profile());
    let _ = vba::raw::host::Policy::deny_all();
}

#[test]
fn streams_round_trip_through_the_facade() {
    let p = project("Public Sub Main()\n  Debug.Print \"x\"\nEnd Sub\n");
    let (streams, report) = p.to_streams(&vba::WriteOptions::default()).unwrap();
    assert!(streams.contains("vba/DIR") && streams.contains("PROJECT"));
    assert_eq!(report.changed_streams.len(), streams.len());
    let back = Project::from_streams(&streams, &ReadLimits::default()).unwrap();
    assert_eq!(back.module_source("Module1"), p.module_source("Module1"));
    let (again, report) = back.to_streams(&vba::WriteOptions::default()).unwrap();
    assert_eq!(again, streams);
    assert!(report.changed_streams.is_empty());
}

#[test]
fn version_facts_and_the_profile_hint_name_a_real_profile() {
    let mut p = project("");
    let v = p.version();
    assert_eq!(v.version_word, None);
    assert_eq!(v.profile_hint(), None);
    let mut header = vec![0xCC, 0x61, 0x97, 0x00, 0x00, 0x01, 0x00];
    header.extend([0; 8]);
    p.model_mut()
        .preserved
        .insert("ovba:vba-project-stream", header);
    let v = p.version();
    assert_eq!(v.version_word, Some(0x97));
    assert_eq!(v.sys_kind, vba::SysKind::Win32);
    let hint = v.profile_hint().unwrap();
    assert!(vba::find_profile(hint).is_some(), "{hint}");
}
