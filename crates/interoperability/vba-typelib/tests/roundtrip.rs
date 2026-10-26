//! Round trip: libraries built with the test writer (per `docs/formats/typelib.md`)
//! are read back into the model.

mod common;

use common::builder::build;
use vba_typelib::{
    CallConv, ConstValue, FuncKind, Guid, ImplFlags, InvokeKind, LibFlags, ParamFlags, TypeDesc,
    TypeFlags, TypeKind, TypeRef, VarKind, parse_msft, vt,
};

#[test]
fn library_header_fields() {
    let lib = parse_msft(&build(&common::sample())).unwrap();
    assert_eq!(lib.name, "Shapes");
    assert_eq!(lib.guid, Guid([0x11; 16]));
    assert_eq!(lib.lcid, 0x409);
    assert_eq!((lib.version.major, lib.version.minor), (2, 5));
    assert_eq!(lib.flags, LibFlags::HIDDEN);
    assert_eq!(lib.sys_kind, Some(1));
    assert_eq!(lib.doc.as_deref(), Some("Shape library"));
    assert_eq!(lib.types.len(), 7);
    assert!(lib.warnings.is_empty(), "{:?}", lib.warnings);
    assert_eq!(lib.imports.len(), 1);
    assert_eq!(lib.imports[0].file, "stdole2.tlb");
    assert_eq!(lib.imports[0].type_guid, Some(Guid::IDISPATCH));
    assert_eq!(lib.imports[0].library_guid, Some(Guid([0x22; 16])));
    assert_eq!(lib.imports[0].kind, Some(TypeKind::Interface));
    assert_eq!(lib.imports[0].version.major, 2);
}

#[test]
fn enum_members_and_constants() {
    let lib = parse_msft(&build(&common::sample())).unwrap();
    let (_, e) = lib.type_by_name("color").unwrap();
    assert_eq!(e.kind, TypeKind::Enum);
    let vals: Vec<_> = e
        .variables
        .iter()
        .map(|v| (v.name.as_str(), v.kind, v.value.clone().unwrap().value))
        .collect();
    assert_eq!(vals[0], ("Red", VarKind::Const, ConstValue::Int(1)));
    assert_eq!(vals[2].2, ConstValue::Int(4));
    // Too large for the immediate form: stored out of line.
    assert_eq!(vals[3].2, ConstValue::Int(70_000_000));
    assert_eq!(vals[4].2, ConstValue::Str("blue-ish".to_owned()));
    assert_eq!(e.variables[1].memid, 1);
}

#[test]
fn record_fields_arrays_and_references() {
    let lib = parse_msft(&build(&common::sample())).unwrap();
    let (_, r) = lib.type_by_name("Rect").unwrap();
    assert_eq!(r.kind, TypeKind::Record);
    assert_eq!(r.guid, Some(Guid([0x33; 16])));
    assert_eq!(r.instance_size, 24);
    assert_eq!(r.alignment, 4);
    assert_eq!(r.variables[0].ty, TypeDesc::Prim(vt::I4));
    assert_eq!(r.variables[0].offset, Some(0));
    assert_eq!(
        r.variables[1].ty,
        TypeDesc::CArray {
            elem: Box::new(TypeDesc::Prim(vt::UI1)),
            bounds: vec![(8, 0)]
        }
    );
    assert_eq!(r.variables[1].offset, Some(4));
    assert_eq!(r.variables[2].ty, TypeDesc::User(TypeRef::Local(0)));
    assert_eq!(lib.describe(&r.variables[1].ty), "UI1[8]");
    assert_eq!(lib.describe(&r.variables[2].ty), "Color");
}

#[test]
fn alias_target() {
    let lib = parse_msft(&build(&common::sample())).unwrap();
    let (_, a) = lib.type_by_name("Handle").unwrap();
    assert_eq!(a.kind, TypeKind::Alias);
    assert_eq!(a.alias, Some(TypeDesc::Prim(vt::I4)));
}

#[test]
fn interface_functions_defaults_and_help() {
    let lib = parse_msft(&build(&common::sample())).unwrap();
    let (_, i) = lib.type_by_name("IShape").unwrap();
    assert_eq!(i.kind, TypeKind::Interface);
    assert_eq!(i.flags, TypeFlags::DUAL | TypeFlags::OLE_AUTOMATION);
    assert_eq!(i.doc.as_deref(), Some("A shape"));
    assert_eq!(i.vtable_size, 40);
    assert_eq!(i.implements.len(), 1);
    assert_eq!(i.implements[0].target, TypeRef::Imported(0));
    let area = &i.functions[0];
    assert_eq!(area.name, "Area");
    assert_eq!(area.memid, 0x10);
    assert_eq!(area.kind, FuncKind::PureVirtual);
    assert_eq!(area.invoke, InvokeKind::PropertyGet);
    assert_eq!(area.call_conv, CallConv::Stdcall);
    assert_eq!(area.vtable_offset, 24);
    assert_eq!(area.ret, TypeDesc::Prim(vt::HRESULT));
    assert_eq!(area.doc.as_deref(), Some("The area"));
    assert_eq!(area.help_context, 77);
    assert_eq!(area.params.len(), 1);
    assert_eq!(area.params[0].flags, ParamFlags::OUT | ParamFlags::RETVAL);
    assert_eq!(
        area.params[0].ty,
        TypeDesc::Ptr(Box::new(TypeDesc::Prim(vt::R8)))
    );
    let scale = &i.functions[1];
    assert_eq!(scale.optional_count, 3);
    assert_eq!(scale.doc, None);
    let d: Vec<_> = scale
        .params
        .iter()
        .map(|p| p.default.clone().unwrap().value)
        .collect();
    assert_eq!(
        d,
        vec![
            ConstValue::Real(1.5),
            ConstValue::Str("none".to_owned()),
            ConstValue::Int(-3)
        ]
    );
}

#[test]
fn dispinterface_members() {
    let lib = parse_msft(&build(&common::sample())).unwrap();
    let (_, s) = lib.type_by_name("Shape").unwrap();
    assert_eq!(s.kind, TypeKind::Dispatch);
    assert_eq!(s.flags, TypeFlags::DISPATCHABLE);
    let names: Vec<_> = s
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.invoke, f.memid))
        .collect();
    assert_eq!(
        names,
        vec![
            ("Name", InvokeKind::PropertyGet, 0),
            ("Name", InvokeKind::PropertyPut, 0),
            ("Parent", InvokeKind::PropertyPutRef, 5),
            ("Draw", InvokeKind::Func, 6),
        ]
    );
    assert_eq!(s.functions[1].params[0].name, None);
    assert_eq!(s.functions[1].ret, TypeDesc::Prim(vt::VOID));
    let draw = &s.functions[3];
    assert_eq!(draw.optional_count, -1);
    assert_eq!(lib.describe(&draw.params[0].ty), "Rect*");
    assert_eq!(lib.describe(&draw.params[1].ty), "SAFEARRAY(VARIANT)");
    assert_eq!(s.functions[2].params[0].name.as_deref(), Some("p"));
    let w = &s.variables[0];
    assert_eq!(
        (w.name.as_str(), w.kind, w.memid),
        ("Width", VarKind::Dispatch, 8)
    );
    assert!(w.flags.bits() & 1 == 1);
    assert_eq!(w.offset, None);
}

#[test]
fn coclass_chain_and_module() {
    let lib = parse_msft(&build(&common::sample())).unwrap();
    let (_, c) = lib.type_by_name("ShapeClass").unwrap();
    assert_eq!(c.kind, TypeKind::CoClass);
    assert!(c.flags.contains(TypeFlags::CAN_CREATE));
    assert_eq!(c.implements.len(), 2);
    assert_eq!(c.implements[0].target, TypeRef::Local(4));
    assert_eq!(c.implements[0].flags, ImplFlags::DEFAULT);
    assert_eq!(c.implements[1].target, TypeRef::Local(3));
    let (_, m) = lib.type_by_name("Native").unwrap();
    assert_eq!(m.kind, TypeKind::Module);
    assert_eq!(m.dll_name.as_deref(), Some("shapes.dll"));
    assert_eq!(m.functions[0].kind, FuncKind::Static);
    assert_eq!(m.functions[0].memid, 0x6000_0000);
}

#[test]
fn lookup_by_guid_and_name() {
    let lib = parse_msft(&build(&common::sample())).unwrap();
    assert_eq!(
        lib.type_by_guid(&Guid([0x44; 16])).unwrap().1.name,
        "IShape"
    );
    assert!(lib.type_by_name("nope").is_none());
    assert_eq!(lib.local(TypeRef::Local(1)).unwrap().name, "Rect");
    assert!(lib.local(TypeRef::Imported(0)).is_none());
    assert!(lib.import(TypeRef::Imported(0)).is_some());
}

#[test]
fn guid_display_is_canonical() {
    assert_eq!(
        Guid::IDISPATCH.to_string(),
        "00020400-0000-0000-C000-000000000046"
    );
}

#[test]
fn empty_library() {
    let lib = parse_msft(&build(&vba_typelib_empty())).unwrap();
    assert_eq!(lib.name, "Empty");
    assert!(lib.types.is_empty());
}

fn vba_typelib_empty() -> common::builder::LibSpec {
    common::builder::LibSpec::new("Empty")
}

#[test]
fn sltg_and_garbage_have_typed_errors() {
    use vba_typelib::TypelibError;
    assert!(matches!(
        parse_msft(b"SLTG\x08\0\x09\0"),
        Err(TypelibError::UnsupportedFormat(_))
    ));
    assert!(matches!(
        parse_msft(b"hello world"),
        Err(TypelibError::NotATypelib(_))
    ));
    assert!(matches!(parse_msft(b""), Err(TypelibError::NotATypelib(_))));
    let mut bad = build(&common::sample());
    bad[4] = 9;
    assert!(matches!(
        parse_msft(&bad),
        Err(TypelibError::UnsupportedFormat(_))
    ));
}
