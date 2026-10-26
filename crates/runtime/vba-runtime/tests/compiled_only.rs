//! Sessions never run compiled-only or stripped modules.

mod common;

use common::*;
use vba_project::{Module, ModuleKind, PERFORMANCE_CACHE_KEY, VbaProject};
use vba_runtime::{Error, Session};
use vba_value::Variant;

fn stripped(name: &str, cache: bool) -> Module {
    let mut m = Module::new(name, ModuleKind::Standard, "");
    m.has_source = false;
    if cache {
        m.preserved.insert(PERFORMANCE_CACHE_KEY, vec![1, 2, 3]);
    }
    m
}

fn full() -> Module {
    Module::new(
        "Main",
        ModuleKind::Standard,
        "Public Function Answer() As Long\r\nAnswer = 42\r\nEnd Function\r\n",
    )
}

fn project(modules: Vec<Module>) -> VbaProject {
    let mut p = VbaProject::new("Proj");
    p.modules = modules;
    p
}

fn builder() -> vba_runtime::SessionBuilder {
    Session::builder().host(host(all_files()))
}

#[test]
fn compiled_only_project_is_refused_before_analysis() {
    let p = project(vec![stripped("A", true), stripped("B", false)]);
    for b in [builder(), builder().allow_partial()] {
        let Err(e) = b.build_project(&p) else {
            panic!("built a compiled-only project")
        };
        assert_eq!(
            e,
            Error::CompiledOnly {
                project: "Proj".into(),
                modules: vec!["A".into(), "B".into()],
            }
        );
        assert_eq!(e.kind(), "compiled-only");
        let text = e.to_string();
        assert!(text.contains("\"Proj\"") && text.contains("\"A\"") && text.contains("\"B\""));
    }
}

#[test]
fn mixed_project_needs_explicit_opt_in() {
    let p = project(vec![full(), stripped("Hidden", true)]);
    let Err(e) = builder().build_project(&p) else {
        panic!("built a mixed project without opt-in")
    };
    assert!(matches!(&e, Error::CompiledOnly { modules, .. } if modules == &["Hidden"]));

    let mut s = builder().allow_partial().build_project(&p).unwrap();
    s.trust();
    // Present modules run.
    let out = s.run_procedure("Main.Answer", vec![]).unwrap();
    assert_eq!(out.value, Variant::Long(42));
    // Calling into the absent module is explicit, qualified or not.
    for name in ["Hidden.Anything", "hidden.Anything", "Anything"] {
        let e = s.run_procedure(name, vec![]).unwrap_err();
        assert!(
            matches!(&e, Error::CompiledOnly { project, modules }
                if project == "Proj" && modules == &["Hidden"]),
            "{name}: {e:?}"
        );
    }
}

#[test]
fn full_source_project_and_empty_modules_run_as_before() {
    let empty = Module::new("Empty", ModuleKind::Class, "");
    let p = project(vec![full(), empty]);
    let mut s = builder().build_project(&p).unwrap();
    s.trust();
    assert!(s.run_procedure("Answer", vec![]).is_ok());
    assert!(matches!(
        s.run_procedure("Nope", vec![]),
        Err(Error::NoSuchProcedure(_))
    ));
}
