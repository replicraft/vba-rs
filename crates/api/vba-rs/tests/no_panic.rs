//! The facade returns errors, never panics, for malformed projects, absurd
//! arguments and misuse. A panic anywhere in these tests is a bug.

use std::time::Duration;

use vba::automation::{ObjectHandle, ObjectRef};
use vba::{
    Arg, CallLimits, CallTarget, Engine, Error, ModuleKind, Project, ProjectStreams, ReadLimits,
    Variant, VmConfig, WriteOptions,
};

const SOURCE: &str = "\
Public Counter As Long
Public Sub Hello()
  Debug.Print \"hello\"
End Sub
Public Function Id(ByVal v As Variant) As Variant
  Id = v
End Function
Public Sub Spin()
  Do
  Loop
End Sub
Public Sub Stopper()
  End
End Sub
Public Function Deep(ByVal n As Long) As Long
  Deep = Deep(n + 1)
End Function
";

fn loaded() -> vba::Instance {
    let project = Project::from_source("Module1", SOURCE);
    let mut inst = Engine::new().load(&project).unwrap();
    inst.trust();
    inst
}

/// xorshift: the same bytes on every run.
fn noise(seed: u64, len: usize) -> Vec<u8> {
    let mut x = seed | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

#[test]
fn malformed_project_bytes_are_errors() {
    assert!(Project::from_bytes(&[]).is_err());
    assert!(Project::from_bytes(b"not a compound file").is_err());
    let magic = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    for seed in 0..64u64 {
        let mut bytes = magic.to_vec();
        bytes.extend(noise(seed, 16 + (seed as usize) * 37));
        let _ = Project::from_bytes(&bytes);
        let _ = Project::from_bytes(&noise(seed, 1 + (seed as usize) * 101));
    }
}

#[test]
fn mutated_valid_projects_never_panic() {
    let mut p = Project::new();
    p.add_module("Module1", ModuleKind::Standard, "Sub A()\r\nEnd Sub\r\n")
        .unwrap();
    p.add_module("Class1", ModuleKind::Class, "Public X As Long\r\n")
        .unwrap();
    let bytes = p.to_bytes().unwrap();
    for seed in 0..200u64 {
        let mut b = bytes.clone();
        let n = noise(seed, 8);
        for pair in n.chunks(2) {
            let at = (usize::from(pair[0]) * 251 + usize::from(pair[1]) * 7 + seed as usize * 13)
                % b.len();
            b[at] ^= pair[1] | 1;
        }
        if seed % 5 == 0 {
            b.truncate(b.len() * (seed as usize % 7 + 1) / 8);
        }
        if let Ok(project) = Project::from_bytes(&b) {
            // Whatever was read must analyse, load and write without panics.
            let _ = project.to_bytes();
            let engine = Engine::new();
            let _ = engine.analyze(&project);
            if let Ok(mut inst) = engine.load(&project) {
                inst.trust();
                let _ = inst.run("A", vec![]);
                let _ = inst.shutdown();
            }
        }
    }
}

#[test]
fn malformed_streams_are_errors() {
    let empty = ProjectStreams::new();
    assert!(Project::from_streams(&empty, &ReadLimits::default()).is_err());
    let mut garbage = ProjectStreams::new();
    for (i, path) in ["VBA/dir", "VBA/_VBA_PROJECT", "PROJECT", "VBA/Module1", ""]
        .into_iter()
        .enumerate()
    {
        garbage.insert(path.to_owned(), noise(i as u64 + 1, 300));
    }
    assert!(Project::from_streams(&garbage, &ReadLimits::default()).is_err());
    let mut tiny = ReadLimits::default();
    tiny.max_stream_bytes = 0;
    tiny.max_entries = 0;
    assert!(Project::from_streams(&garbage, &tiny).is_err());
}

#[test]
fn opening_unusable_paths_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        Project::open(dir.path().join("missing.bin")),
        Err(Error::Io(_))
    ));
    assert!(Project::open(dir.path()).is_err()); // a directory
    let empty = dir.path().join("empty.bin");
    std::fs::write(&empty, b"").unwrap();
    assert!(Project::open(&empty).is_err());
    // A file over the limit is refused without being read to the end.
    let big = dir.path().join("big.bin");
    std::fs::write(&big, vec![0u8; 4096]).unwrap();
    let mut limits = ReadLimits::default();
    limits.max_container_bytes = 1024;
    assert!(Project::open_with(&big, &limits).is_err());
    assert!(Project::open("").is_err());
    assert!(Project::open("a\0b").is_err());
}

#[test]
fn project_edits_with_odd_names() {
    let mut p = Project::new();
    for name in ["", " ", "a b", "ü", "\0", "1", &"x".repeat(100_000)] {
        let _ = p.add_module(name, ModuleKind::Standard, "");
        let _ = p.set_module_source(name, "Sub A()\nEnd Sub\n");
        let _ = p.module_source(name);
        let _ = p.module(name);
    }
    let _ = p.to_bytes();
    let _ = p.to_streams(&WriteOptions::default());
    assert!(
        Project::from_modules(
            "P",
            [
                ("A", ModuleKind::Standard, ""),
                ("a", ModuleKind::Standard, "")
            ]
        )
        .is_err()
    );
    for (file, text) in [
        ("", ""),
        ("x", "garbage"),
        ("a.cls", "VERSION 1.0 CLASS\nBEGIN\n"),
        ("a.frm", "\0\0\0"),
        (".bas", "Attribute VB_Name = "),
        ("a.bas", "Attribute VB_Name = \"\"\n"),
    ] {
        let _ = Project::new().add_source_file(file, text);
    }
}

#[test]
fn absurd_procedure_names_and_arguments() {
    let mut inst = loaded();
    let long = "A".repeat(1 << 20);
    for name in [
        "",
        ".",
        "..",
        "A.B.C",
        "Module1.",
        ".Hello",
        "\0",
        "Hello\0",
        "ü",
        &long,
        "Module1.Id",
    ] {
        let _ = inst.run(name, vec![]);
        let _ = inst.call(CallTarget::Project, name, vec![]);
        let _ = inst.raise_event(name, name, vec![]);
        let _ = inst.get_global(name);
        let _ = inst.set_global(name, Variant::Empty);
        let _ = inst.create_object(name, None);
        let _ = inst.default_instance(name, None);
        let _ = inst.modules_defining(name);
    }
    let many: Vec<Variant> = (0..10_000).map(Variant::Long).collect();
    assert!(inst.run("Id", many).is_err());
    assert!(inst.run("Id", vec![]).is_err());
    assert!(inst.run("Id", vec![Variant::Empty, Variant::Null]).is_err());
    let big = Variant::from("x".repeat(8 << 20).as_str());
    let _ = inst.run("Id", vec![big]);
    for v in [
        Variant::Empty,
        Variant::Null,
        Variant::Long(i32::MIN),
        Variant::Double(f64::NAN),
        Variant::Double(f64::INFINITY),
        Variant::Double(-0.0),
        Variant::from(""),
    ] {
        let _ = inst.run("Id", vec![v.clone()]);
        let _ = inst.set_global("Counter", v);
    }
}

#[test]
fn malformed_argument_lists() {
    let mut inst = loaded();
    let nested = Arg::named("a", Arg::named("b", Arg::val(1)));
    assert!(inst.call(CallTarget::Project, "Id", vec![nested]).is_err());
    let after = vec![Arg::named("v", Arg::val(1)), Arg::val(2)];
    assert!(inst.call(CallTarget::Project, "Id", after).is_err());
    for args in [
        vec![Arg::named("", Arg::val(1))],
        vec![Arg::named("v", Arg::val(1)), Arg::named("v", Arg::val(2))],
        vec![Arg::missing()],
        vec![Arg::missing(), Arg::missing(), Arg::missing()],
        vec![Arg::by_ref(Variant::Empty); 100],
    ] {
        let _ = inst.call(CallTarget::Project, "Id", args);
    }
}

#[test]
fn forged_and_stale_object_references() {
    let mut inst = loaded();
    let forged = [
        ObjectRef::Nothing,
        ObjectRef::Handle(ObjectHandle(0)),
        ObjectRef::Handle(ObjectHandle(1)),
        ObjectRef::Handle(ObjectHandle(u64::MAX)),
        ObjectRef::Handle(ObjectHandle(1 << 63)),
        ObjectRef::Handle(ObjectHandle((1 << 63) | 1)),
    ];
    for o in forged {
        let _ = inst.destroy_object(o);
        let _ = inst.call_on(o, "Hello", vec![]);
        let _ = inst.get_member(o, "X");
        let _ = inst.set_member(o, "X", Variant::Long(1));
        let _ = inst.has_procedure(o, "Hello");
        let _ = inst.raise_event_on(o, "Form", "Current", vec![Arg::by_ref(Variant::Long(0))]);
        let _ = inst.call(CallTarget::Object(o), "Hello", vec![]);
        inst.release(&Variant::Object(o));
    }
    inst.release(&Variant::Empty);
    inst.release(&Variant::from("not an object"));
}

#[test]
fn objects_after_destroy_and_reset() {
    let mut p = Project::new();
    p.add_module(
        "Class1",
        ModuleKind::Class,
        "Public X As Long\r\nPublic Sub M()\r\nEnd Sub\r\n",
    )
    .unwrap();
    let mut inst = Engine::new().load(&p).unwrap();
    inst.trust();
    let o = inst.create_object("Class1", None).unwrap();
    inst.destroy_object(o).unwrap();
    assert!(inst.destroy_object(o).is_err()); // double destroy
    assert!(inst.call_on(o, "M", vec![]).is_err()); // stale
    let o2 = inst.create_object("Class1", None).unwrap();
    inst.reset().unwrap();
    assert!(inst.call_on(o2, "M", vec![]).is_err()); // invalid after reset
    let _ = inst.destroy_object(o2);
}

#[test]
fn calls_after_a_fatal_error_return_errors() {
    // Resource limit.
    let mut inst = loaded();
    inst.reset_step_counter();
    let err = inst
        .with_call_limits(CallLimits::new().max_steps(1000), |i| i.run("Spin", vec![]))
        .unwrap_err();
    assert!(matches!(err, Error::Run(_)));
    assert!(inst.run("Hello", vec![]).is_ok());

    // Whole-instance step budget spent.
    let project = Project::from_source("Module1", SOURCE);
    let mut inst = Engine::new().max_steps(5_000).load(&project).unwrap();
    inst.trust();
    assert!(inst.run("Spin", vec![]).is_err());
    assert!(!inst.is_usable());
    assert!(inst.run("Hello", vec![]).is_err());
    let _ = inst.get_global("Counter");
    let _ = inst.set_global("Counter", Variant::Long(1));
    let _ = inst.shutdown();
    inst.reset().unwrap();
    assert!(inst.is_usable());
    inst.run("Hello", vec![]).unwrap();

    // Cancellation.
    let engine = Engine::new();
    let mut inst = engine.load(&project).unwrap();
    inst.trust();
    engine.cancel_token().cancel();
    assert!(inst.run("Hello", vec![]).is_err());
    assert!(inst.run("Hello", vec![]).is_err());
    assert!(inst.shutdown().is_err() || !inst.is_usable());
    engine.cancel_token().reset();
    inst.reset().unwrap();
    inst.run("Hello", vec![]).unwrap();

    // `End`, then keep using the instance.
    let mut inst = loaded();
    let _ = inst.run("Stopper", vec![]);
    let _ = inst.run("Hello", vec![]);
    let _ = inst.shutdown();
    let _ = inst.shutdown();
    let _ = inst.run("Hello", vec![]);

    // Unbounded recursion ends with a limit error, not a stack overflow.
    let mut inst = loaded();
    assert!(inst.run("Deep", vec![Variant::Long(0)]).is_err());
    let _ = inst.run("Hello", vec![]);
}

#[test]
fn absurd_engine_configuration() {
    let project = Project::from_source("Module1", SOURCE);
    for engine in [
        Engine::new().max_steps(0),
        Engine::new().max_memory(0),
        Engine::new().call_timeout(Some(Duration::ZERO)),
        Engine::new().call_timeout(Some(Duration::MAX)),
        Engine::new().max_steps(u64::MAX).max_memory(u64::MAX),
        Engine::new().vm_config(VmConfig::default()),
        Engine::new().host_members(["", "\0", "a b"]),
        Engine::new().dispatch_class("", || vba::automation::testing::MemObject::new("X")),
    ] {
        let _ = engine.analyze(&project);
        let _ = engine.run(&project, "Hello", vec![]);
        let _ = engine.run(&project, "", vec![]);
    }
    let mut inst = loaded();
    let zero = CallLimits::new()
        .max_steps(0)
        .max_memory(0)
        .timeout(Duration::ZERO);
    let _ = inst.with_call_limits(zero, |i| i.run("Hello", vec![]));
    let huge = CallLimits::new()
        .max_steps(u64::MAX)
        .max_memory(u64::MAX)
        .timeout(Duration::MAX);
    let _ = inst.with_call_limits(huge, |i| i.run("Hello", vec![]));
    inst.default_call_timeout(Some(Duration::MAX));
    let _ = inst.run("Hello", vec![]);
    inst.default_call_timeout(Some(Duration::ZERO));
    let _ = inst.run("Hello", vec![]);
}

#[test]
fn untrusted_and_unusual_sources() {
    for src in [
        "",
        "\0",
        "\u{feff}",
        "Sub",
        "Sub A(",
        "Public Sub A()\n  Debug.Print\n",
        &"(".repeat(100_000),
        &"Sub A()\n".repeat(10_000),
        &format!("Public Sub A()\n  x = {}1\nEnd Sub\n", "-".repeat(50_000)),
        &format!(
            "Public Sub A()\n  x = {}1{}\nEnd Sub\n",
            "(".repeat(20_000),
            ")".repeat(20_000)
        ),
        &"If True Then\n".repeat(5_000),
    ] {
        let project = Project::from_source("Module1", src);
        let engine = Engine::new();
        let _ = engine.analyze(&project);
        if let Ok(mut inst) = engine.load(&project) {
            inst.trust();
            let _ = inst.run("A", vec![]);
        }
    }
}
