//! Shared helpers for the IR tests.
#![allow(dead_code)]

use vba_ir::{DumpOptions, IrProgram, dump_function, lower, verify};
use vba_profile::find_profile;
use vba_project::ModuleKind;
use vba_sema::{
    Analysis, LibMember, LibParam, Options, SourceModule, StaticLibrary, Type, analyze_sources,
};

pub fn library() -> StaticLibrary {
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

pub fn module(name: &str, kind: ModuleKind, src: &str) -> SourceModule {
    SourceModule {
        name: name.to_owned(),
        kind,
        source: src.as_bytes().to_vec(),
    }
}

pub fn analyze(mods: &[SourceModule]) -> Analysis {
    let profile = *find_profile("vba7.1-excel-win64").expect("profile");
    analyze_sources(mods, &profile, &library(), &Options::default())
}

pub fn lower_ok(mods: &[SourceModule]) -> IrProgram {
    let a = analyze(mods);
    let errs: Vec<_> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == vba_diagnostics::Severity::Error)
        .map(|d| vba_diagnostics::render_plain(d, &a.sources))
        .collect();
    assert!(errs.is_empty(), "analysis errors:\n{}", errs.join("\n"));
    let ir = lower(&a).expect("lower");
    if let Err(e) = verify(&ir) {
        panic!(
            "verifier errors:\n{}\n{}",
            e.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
            vba_ir::dump_program(&ir, DumpOptions::default())
        );
    }
    ir
}

/// Lowers one standard module `M` and dumps procedure `name`.
pub fn dump(src: &str, name: &str) -> String {
    dump_opts(src, name, DumpOptions::default())
}

pub fn dump_opts(src: &str, name: &str, opts: DumpOptions) -> String {
    let ir = lower_ok(&[module("M", ModuleKind::Standard, src)]);
    let p = ir
        .program
        .find_proc("M", name)
        .unwrap_or_else(|| panic!("no procedure {name}"));
    dump_function(&ir, ir.function_of(p.id), opts)
}
