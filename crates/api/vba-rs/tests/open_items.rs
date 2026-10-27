//! Facade details: the `source()` of load errors, the borrowing output
//! accessor and `Project.Module.Proc` call-by-name.

use std::error::Error as _;

use vba::runtime::{Error as RunError, LoadError};
use vba::{Engine, Error, Project, Variant};

const SOURCE: &str = "\
Public Function Twice(ByVal n As Long) As Long
  Debug.Print \"twice\"
  Twice = n * 2
End Function
";

fn instance() -> vba::Instance {
    let project = Project::from_source("Module1", SOURCE);
    let mut i = Engine::new().load(&project).unwrap();
    i.trust();
    i
}

#[test]
fn a_load_error_has_the_structured_error_as_its_source() {
    let project = Project::from_source("Module1", "Public Sub Main()\n  Dim x As\nEnd Sub\n");
    let err = Engine::new().load(&project).map(|_| ()).unwrap_err();
    assert!(matches!(&err, Error::Run(RunError::Load(_))), "{err:?}");
    let source = err.source().expect("a Load error has a source");
    let load = source
        .downcast_ref::<LoadError>()
        .expect("the source is the LoadError");
    assert!(!load.diagnostics.is_empty());
    // Other families keep their behavior: a plain run error has no source.
    let mut i = instance();
    let err = i.run("Nope", vec![]).unwrap_err();
    assert!(err.source().is_none());
}

#[test]
fn with_output_borrows_what_output_copies() {
    let mut i = instance();
    i.run("Twice", vec![Variant::Long(2)]).unwrap();
    assert_eq!(i.output(), "twice\n");
    let n = i.with_output(|text| {
        assert_eq!(text, "twice\n");
        text.len()
    });
    assert_eq!(n, 6);
}

#[test]
fn the_project_name_may_qualify_a_procedure() {
    let mut i = instance();
    let name = Project::from_source("Module1", SOURCE).name().to_owned();
    for qualified in [
        format!("{name}.Module1.Twice"),
        format!("{}.module1.twice", name.to_uppercase()),
        "Module1.Twice".to_owned(),
        "Twice".to_owned(),
    ] {
        let out = i.run(&qualified, vec![Variant::Long(4)]).unwrap();
        assert_eq!(out.value, Variant::Long(8), "{qualified}");
    }
    // A wrong project name is not ignored.
    let err = i.run("Other.Module1.Twice", vec![Variant::Long(4)]);
    assert!(
        matches!(err, Err(Error::Run(RunError::NoSuchProcedure(_)))),
        "{err:?}"
    );
}
