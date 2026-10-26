//! Shared helpers for the sema tests.
#![allow(dead_code)]

use vba_diagnostics::{Severity, render_plain};
use vba_profile::{Profile, find_profile};
use vba_project::ModuleKind;
use vba_sema::{
    Analysis, LibMember, LibParam, Options, SourceModule, StaticLibrary, Type, TypeProvider,
    analyze_sources,
};

pub fn profile64() -> Profile {
    *find_profile("vba7.1-excel-win64").expect("profile")
}

pub fn profile32() -> Profile {
    *find_profile("vba7.1-excel-win32").expect("profile")
}

pub fn module(name: &str, kind: ModuleKind, src: &str) -> SourceModule {
    SourceModule {
        name: name.to_owned(),
        kind,
        source: src.as_bytes().to_vec(),
    }
}

pub fn bas(name: &str, src: &str) -> SourceModule {
    module(name, ModuleKind::Standard, src)
}

pub fn cls(name: &str, src: &str) -> SourceModule {
    module(name, ModuleKind::Class, src)
}

/// A tiny host library: `Scripting.Dictionary` and a `Widget` class.
pub fn test_library() -> StaticLibrary {
    StaticLibrary::new("Scripting")
        .with_type(
            "Dictionary",
            vec![
                LibMember::sub(
                    "Add",
                    vec![
                        LibParam::required("Key", Type::Variant),
                        LibParam::required("Item", Type::Variant),
                    ],
                ),
                LibMember::function(
                    "Exists",
                    vec![LibParam::required("Key", Type::Variant)],
                    Type::Boolean,
                ),
                LibMember::property("Count", Type::Long),
                LibMember::function(
                    "Item",
                    vec![LibParam::required("Key", Type::Variant)],
                    Type::Variant,
                )
                .default_member(),
            ],
        )
        .with_opaque_type("FileSystemObject")
}

pub fn analyze(mods: &[SourceModule]) -> Analysis {
    analyze_with(mods, &profile64(), &test_library())
}

pub fn analyze_with(mods: &[SourceModule], profile: &Profile, p: &dyn TypeProvider) -> Analysis {
    analyze_sources(mods, profile, p, &Options::default())
}

/// `CODE@line` of every error and warning, in diagnostic order.
pub fn codes(a: &Analysis) -> Vec<String> {
    a.diagnostics
        .iter()
        .filter(|d| d.severity != Severity::Info && d.severity != Severity::Hint)
        .map(|d| {
            let f = a.sources.get(d.file).expect("file");
            format!("{}@{}", d.code.0, f.line_col(d.span.start).line + 1)
        })
        .collect()
}

pub fn render(a: &Analysis) -> String {
    a.diagnostics
        .iter()
        .map(|d| render_plain(d, &a.sources))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn assert_clean(a: &Analysis) {
    let errs: Vec<_> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error || d.severity == Severity::Warning)
        .collect();
    assert!(errs.is_empty(), "unexpected diagnostics:\n{}", render(a));
}
