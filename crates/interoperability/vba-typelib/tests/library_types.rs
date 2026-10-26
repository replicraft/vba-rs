//! Library enums, records, classes, member ids and defaults through the
//! `vba-sema` adapter.
#![cfg(feature = "sema")]

mod common;

use common::builder::build;
use vba_diagnostics::Severity;
use vba_profile::find_profile;
use vba_project::ModuleKind;
use vba_sema::{Analysis, LibraryType, Options, SourceModule, Type, TypeProvider, analyze_sources};
use vba_typelib::{parse_msft, sema::TypelibProvider};
use vba_value::Variant;

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
fn members_carry_ids_and_declared_defaults() {
    let p = provider();
    let area = p.member(&lt("IShape"), "Area").unwrap();
    assert_eq!(area.dispid, Some(0x10));
    let scale = p.member(&lt("IShape"), "Scale").unwrap();
    assert_eq!(scale.dispid, Some(0x11));
    assert_eq!(scale.params[0].default, Some(Variant::Double(1.5)));
    assert_eq!(scale.params[1].default, Some(Variant::from("none")));
    // Converted to the parameter type.
    assert_eq!(scale.params[2].default, Some(Variant::Long(-3)));
    // The default member of the dispinterface is DISPID 0.
    assert_eq!(p.member(&lt("Shape"), "Name").unwrap().dispid, Some(0));
    // A coclass forwards the ids of its default interface.
    assert_eq!(p.member(&lt("ShapeClass"), "Draw").unwrap().dispid, Some(6));
}

#[test]
fn enums_records_and_classes_are_described() {
    let p = provider();
    let members = p.enum_members(&lt("Color")).unwrap();
    assert_eq!(members[0], ("Red".to_owned(), 1));
    assert_eq!(members[3], ("Big".to_owned(), 70_000_000));
    // The string constant is not an enum member value.
    assert_eq!(members.len(), 4);
    assert!(p.enum_members(&lt("Rect")).is_none());

    let rec = p.record(&lt("Rect")).unwrap();
    assert_eq!(rec.size, 24);
    let names: Vec<_> = rec.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["Left", "Tag", "Fill"]);
    assert_eq!(rec.fields[1].offset, Some(4));
    assert_eq!(rec.fields[2].ty, Type::LibEnum(lt("Color")));

    let class = p.class_info(&lt("ShapeClass")).unwrap();
    assert!(class.creatable);
    assert_eq!(class.default_interface, Some(lt("Shape")));
    assert!(!p.class_info(&lt("IShape")).unwrap().creatable);
    assert!(p.class_info(&lt("Rect")).is_none());
}

#[test]
fn library_types_in_vba_code() {
    let a = analyse(
        "Option Explicit\n\
         Sub Main()\n\
         Dim r As Shapes.Rect\n\
         Dim h As Shapes.Handle\n\
         Dim c As Shapes.Color\n\
         Dim s As New Shapes.ShapeClass\n\
         Dim i As New Shapes.IShape\n\
         c = Shapes.Color.Blue\n\
         c = Color.Red\n\
         r.Fill = Green\n\
         r.Left = h\n\
         s.Draw r\n\
         End Sub\n",
    );
    // Only `New` on an interface is wrong.
    assert_eq!(codes(&a), vec!["SEM031@7"], "{:?}", a.diagnostics);
    assert_eq!(a.program.lib_records.len(), 1);
    assert_eq!(a.program.lib_records[0].size, 24);
}
