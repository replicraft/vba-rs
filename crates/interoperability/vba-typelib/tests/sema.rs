//! The `vba-sema` adapter: a VBA module analysed against a synthetic library.
#![cfg(feature = "sema")]

mod common;

use common::builder::build;
use vba_diagnostics::Severity;
use vba_profile::find_profile;
use vba_project::ModuleKind;
use vba_sema::{
    Analysis, LibGlobal, LibMemberKind, LibraryType, Options, SourceModule, Type, TypeProvider,
    analyze_sources,
};
use vba_typelib::{parse_msft, sema::TypelibProvider};

fn provider() -> TypelibProvider {
    TypelibProvider::new().with_library(parse_msft(&build(&common::sample())).unwrap())
}

fn lt(name: &str) -> LibraryType {
    LibraryType {
        library: "Shapes".into(),
        name: name.into(),
    }
}

fn analyse(src: &str) -> Analysis {
    let profile = *find_profile("vba7.1-excel-win64").expect("profile");
    analyze_sources(
        &[SourceModule {
            name: "M".into(),
            kind: ModuleKind::Standard,
            source: src.as_bytes().to_vec(),
        }],
        &profile,
        &provider(),
        &Options::default(),
    )
}

/// `CODE@line` of errors and warnings.
fn codes(a: &Analysis) -> Vec<String> {
    a.diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error | Severity::Warning))
        .map(|d| {
            let f = a.sources.get(d.file).expect("file");
            format!("{}@{}", d.code.0, f.line_col(d.span.start).line + 1)
        })
        .collect()
}

#[test]
fn vba_module_binds_against_the_library() {
    let a = analyse(
        "Option Explicit\n\
         Sub Main()\n\
         Dim s As Shapes.Shape\n\
         Dim q As Shapes.IShape\n\
         Dim r As Shapes.Rect\n\
         Dim t As String\n\
         Dim c As Long\n\
         Dim area As Double\n\
         s.Name = \"x\"\n\
         t = s.Name\n\
         s.Draw r\n\
         s.Draw r, 1, 2, 3\n\
         r.Left = 3\n\
         c = Red + Blue\n\
         area = q.Area\n\
         q.Scale 2#, \"a\", 3\n\
         q.Scale\n\
         c = MakeShape(\"n\")\n\
         End Sub\n",
    );
    assert_eq!(codes(&a), Vec::<String>::new(), "{:?}", a.diagnostics);
}

#[test]
fn errors_are_found_with_library_knowledge() {
    let a = analyse(
        "Option Explicit\n\
         Sub Main()\n\
         Dim s As Shapes.Shape\n\
         Dim q As Shapes.IShape\n\
         s.Nope\n\
         q.Scale 1#, \"a\", 3, 4\n\
         Dim e As Shapes.Color\n\
         End Sub\n",
    );
    // Line 5: no such member; line 6: too many arguments; line 7: library
    // enums cannot be declared types (sema gap). Assignment to a read-only
    // library property is not diagnosed by vba-sema.
    assert_eq!(
        codes(&a),
        vec!["SEM019@5", "SEM008@6", "SEM004@7"],
        "{:?}",
        a.diagnostics
    );
}

#[test]
fn resolve_type_rules() {
    let p = provider();
    assert_eq!(p.resolve_type(Some("shapes"), "ishape"), Some(lt("IShape")));
    assert_eq!(p.resolve_type(None, "Rect"), Some(lt("Rect")));
    assert_eq!(p.resolve_type(Some("Other"), "Rect"), None);
    assert_eq!(p.resolve_type(None, "ShapeClass"), Some(lt("ShapeClass")));
    // Enums, aliases of primitives and modules are not declarable types.
    assert_eq!(p.resolve_type(None, "Color"), None);
    assert_eq!(p.resolve_type(None, "Handle"), None);
    assert_eq!(p.resolve_type(None, "Native"), None);
}

#[test]
fn members_map_to_vba_shapes() {
    let p = provider();
    let area = p.member(&lt("IShape"), "area").unwrap();
    assert_eq!(area.kind, LibMemberKind::PropertyGet);
    assert_eq!(area.ty, Type::Double);
    assert!(area.params.is_empty(), "retval is hidden");
    let scale = p.member(&lt("IShape"), "Scale").unwrap();
    assert!(scale.is_sub);
    assert_eq!(scale.params.len(), 3);
    assert!(scale.params.iter().all(|a| a.optional && !a.by_ref));
    assert_eq!(scale.params[2].ty, Type::Long);

    let name = p.member(&lt("Shape"), "Name").unwrap();
    assert_eq!(name.kind, LibMemberKind::Property);
    assert_eq!(name.ty, Type::String);
    assert!(name.is_default, "member id 0");
    assert_eq!(p.default_member(&lt("Shape")).unwrap().name, "Name");
    let parent = p.member(&lt("Shape"), "Parent").unwrap();
    assert_eq!(parent.kind, LibMemberKind::PropertyPut);
    assert_eq!(parent.ty, Type::Library(lt("Shape")));
    let draw = p.member(&lt("Shape"), "Draw").unwrap();
    assert!(draw.is_sub);
    assert_eq!(draw.params[0].ty, Type::Library(lt("Rect")));
    assert!(draw.params[0].by_ref, "pointer to a record is ByRef");
    assert!(draw.params[1].param_array);
    assert_eq!(
        draw.params[1].ty,
        Type::Array {
            elem: Box::new(Type::Variant),
            rank: None
        }
    );
    let width = p.member(&lt("Shape"), "Width").unwrap();
    assert_eq!(width.kind, LibMemberKind::PropertyGet, "read-only variable");

    // Record fields are properties.
    let fill = p.member(&lt("Rect"), "Fill").unwrap();
    assert_eq!(fill.ty, Type::Long);
    let tag = p.member(&lt("Rect"), "Tag").unwrap();
    assert_eq!(
        tag.ty,
        Type::Array {
            elem: Box::new(Type::Byte),
            rank: Some(1)
        }
    );
    assert!(p.describes_members(&lt("Rect")));
    assert!(p.member(&lt("Rect"), "nothing").is_none());
}

#[test]
fn coclass_forwards_to_its_default_interface_and_hides_restricted() {
    let p = provider();
    // `ShapeClass` implements `Shape` (default) and `IShape`.
    assert!(p.member(&lt("ShapeClass"), "Draw").is_some());
    assert!(p.member(&lt("ShapeClass"), "Area").is_none());
    assert!(p.describes_members(&lt("ShapeClass")));
    // Not a source: no events.
    assert_eq!(p.events(&lt("ShapeClass")), Some(vec![]));
}

#[test]
fn interface_with_unresolved_base_is_not_claimed_complete() {
    let p = provider();
    // IShape derives from IDispatch (imported, not modelled): complete.
    assert!(p.interface_members(&lt("IShape")).is_some());
    // A library type that is not in the provider is unknown.
    assert!(p.interface_members(&lt("Missing")).is_none());
    assert!(!p.describes_members(&lt("Missing")));
}

#[test]
fn globals_are_enum_constants_and_module_functions() {
    let p = provider();
    match p.global("blue") {
        Some(LibGlobal::Constant(v, t)) => {
            assert_eq!(t, Type::Long);
            assert_eq!(format!("{v:?}"), "Long(4)");
        }
        other => panic!("{other:?}"),
    }
    match p.global("Big") {
        Some(LibGlobal::Constant(_, t)) => assert_eq!(t, Type::Long),
        other => panic!("{other:?}"),
    }
    match p.global("Label") {
        Some(LibGlobal::Constant(_, t)) => assert_eq!(t, Type::String),
        other => panic!("{other:?}"),
    }
    match p.global("MakeShape") {
        Some(LibGlobal::Function(m)) => {
            assert_eq!(m.ty, Type::Long);
            assert_eq!(m.params[0].ty, Type::String);
        }
        other => panic!("{other:?}"),
    }
    assert!(p.global("Nope").is_none());
}
