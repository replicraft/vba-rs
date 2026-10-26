//! A project assembled from module text the embedding application keeps in
//! its own tables: no files, no `vbaProject.bin`.

use vba::{Arg, CallTarget, Engine, Error, ModuleKind, Project, Variant};

fn modules() -> Vec<(&'static str, ModuleKind, &'static str)> {
    vec![
        (
            "Utils",
            ModuleKind::Standard,
            "Option Explicit\nPublic Function Twice(ByVal n As Long) As Long\n  Twice = n * 2\nEnd Function\nPublic Sub Swap(ByRef a As Long, ByRef b As Long)\n  Dim t As Long\n  t = a: a = b: b = t\nEnd Sub\n",
        ),
        (
            "Customer",
            ModuleKind::Class,
            "Option Explicit\nPublic Name As String\nPublic Function Greeting() As String\n  Greeting = \"Hi \" & Name\nEnd Function\n",
        ),
        (
            "Form_Orders",
            ModuleKind::Document,
            "Option Explicit\nPublic Sub Hello()\n  Debug.Print \"form\"\nEnd Sub\n",
        ),
    ]
}

#[test]
fn from_modules_builds_a_runnable_project_from_text() {
    let project = Project::from_modules("AppProject", modules()).unwrap();
    assert_eq!(project.name(), "AppProject");
    assert_eq!(project.modules().len(), 3);
    assert!(!Engine::new().analyze(&project).has_errors());

    let mut inst = Engine::new().load(&project).unwrap();
    inst.trust();
    let swapped = inst
        .call(
            CallTarget::Project,
            "Utils.Swap",
            vec![Arg::by_ref(1), Arg::by_ref(2)],
        )
        .unwrap();
    assert_eq!(swapped.byref, vec![Variant::Long(2), Variant::Long(1)]);
    let obj = inst.create_object("Customer", None).unwrap();
    inst.set_member(obj, "Name", Variant::from("Ada")).unwrap();
    let g = inst.call_on(obj, "Greeting", vec![]).unwrap();
    assert_eq!(g.value, Variant::from("Hi Ada"));
}

#[test]
fn a_duplicate_module_name_is_a_model_error() {
    let mut m = modules();
    m.push(("utils", ModuleKind::Standard, ""));
    let err = Project::from_modules("P", m).unwrap_err();
    assert_eq!(err.kind(), "model");
}

#[test]
fn replacing_one_module_source_reloads_cleanly() {
    let mut project = Project::from_modules("AppProject", modules()).unwrap();
    project
        .set_module_source(
            "Utils",
            "Public Function Twice(ByVal n As Long) As Long\n  Twice = n * 3\nEnd Function\n",
        )
        .unwrap();
    let out = Engine::new()
        .run(&project, "Utils.Twice", vec![Variant::Long(5)])
        .unwrap();
    assert_eq!(out.value, Variant::Long(15));
}

#[test]
fn a_module_that_does_not_compile_is_reported_before_anything_runs() {
    let project =
        Project::from_modules("P", [("M", ModuleKind::Standard, "Public Sub A(\nEnd Sub\n")])
            .unwrap();
    let report = Engine::new().analyze(&project);
    assert!(report.has_errors());
    let err = Engine::new().load(&project).err().expect("load must fail");
    assert!(matches!(err, Error::Run(_)));
    assert_eq!(err.kind(), "load");
}
