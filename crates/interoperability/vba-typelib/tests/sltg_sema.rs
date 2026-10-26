//! An SLTG library feeds the `vba-sema` adapter like an MSFT one.
#![cfg(feature = "sema")]

mod common;

use common::sltg::build_sltg;
use common::sltg_libs::sample;
use vba_diagnostics::Severity;
use vba_profile::find_profile;
use vba_project::ModuleKind;
use vba_sema::{Analysis, LibraryType, Options, SourceModule, Type, TypeProvider, analyze_sources};
use vba_typelib::{parse_sltg, sema::TypelibProvider};

fn provider() -> TypelibProvider {
    TypelibProvider::new().with_library(parse_sltg(&build_sltg(&sample())).unwrap())
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
fn types_resolve_like_msft_ones() {
    let p = provider();
    let lt = |n: &str| LibraryType {
        library: "Shapes".into(),
        name: n.into(),
    };
    assert_eq!(p.resolve_type(Some("shapes"), "ishape"), Some(lt("IShape")));
    assert_eq!(p.resolve_type(None, "Rect"), Some(lt("Rect")));
    assert_eq!(
        p.resolve_declared(None, "Color"),
        Some(Type::LibEnum(lt("Color")))
    );
}

#[test]
fn module_binds_against_the_library() {
    let a = analyse(
        "Option Explicit\n\
         Sub Main()\n\
         Dim q As Shapes.IShape\n\
         Dim r As Shapes.Rect\n\
         Dim area As Double\n\
         Dim n As Long\n\
         area = q.Area\n\
         q.Move 1, 2, r\n\
         r.Left = 3\n\
         n = Red + Blue\n\
         q.Nope\n\
         q.Move 1, 2, r, 4\n\
         End Sub\n",
    );
    assert_eq!(
        codes(&a),
        vec!["SEM019@11", "SEM008@12"],
        "{:?}",
        a.diagnostics
    );
}
