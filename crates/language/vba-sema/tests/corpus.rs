//! The realistic fixture project must analyse without errors.

mod common;

use common::*;
use vba_project::ModuleKind;
use vba_sema::{Analysis, SourceModule};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn corpus() -> Vec<SourceModule> {
    vec![
        module("Geometry", ModuleKind::Standard, &fixture("Geometry.bas")),
        module("IShape", ModuleKind::Class, &fixture("IShape.cls")),
        module("Disc", ModuleKind::Class, &fixture("Disc.cls")),
        module("Watcher", ModuleKind::Class, &fixture("Watcher.cls")),
        module("WinApi", ModuleKind::Standard, &fixture("WinApi.bas")),
        module("Tour", ModuleKind::Standard, &fixture("Tour.bas")),
        module("Text", ModuleKind::Standard, &fixture("Text.bas")),
        module("Bag", ModuleKind::Class, &fixture("Bag.cls")),
        module("BagUser", ModuleKind::Class, &fixture("BagUser.cls")),
    ]
}

fn analyzed() -> Analysis {
    analyze(&corpus())
}

#[test]
fn corpus_has_no_errors() {
    let a = analyzed();
    assert_clean(&a);
}

#[test]
fn corpus_symbols() {
    let a = analyzed();
    let p = &a.program;
    let circle = p.module_by_name("disc").expect("Disc");
    assert_eq!(circle.implements.len(), 1);
    assert_eq!(circle.implements[0].members.len(), 3);
    assert!(circle.initialize.is_some() && circle.terminate.is_some());
    let watcher = p.module_by_name("Watcher").unwrap();
    assert_eq!(watcher.handlers.len(), 1);
    assert_eq!(watcher.default_member.as_ref().unwrap().to_string(), "Item");
    let geo = p.module_by_name("GEOMETRY").unwrap();
    assert_eq!(geo.enums.len(), 1);
    let e = &p.enums[geo.enums[0].index()];
    let values: Vec<_> = e.members.iter().map(|m| m.1).collect();
    assert_eq!(values, vec![1, 2, 7]);
}

#[test]
fn win64_requires_ptrsafe_and_longptr_is_longlong() {
    let a = analyzed();
    let p = &a.program;
    let find = p
        .declares
        .iter()
        .find(|d| d.name.to_string() == "FindWindow")
        .unwrap();
    assert_eq!(find.ret, vapi_longlong());
    assert!(find.ptr_safe);
}

fn vapi_longlong() -> vba_sema::Type {
    vba_sema::Type::LongLong
}

#[test]
fn win32_longptr_is_long() {
    let mods = corpus();
    let a = analyze_with(&mods, &profile32(), &test_library());
    assert_clean(&a);
    let find = a
        .program
        .declares
        .iter()
        .find(|d| d.name.to_string() == "FindWindow")
        .unwrap();
    assert_eq!(find.ret, vba_sema::Type::Long);
}

#[test]
fn deterministic() {
    let a = render(&analyzed());
    let b = render(&analyzed());
    assert_eq!(a, b);
}
