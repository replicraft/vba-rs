//! The SLTG reader on synthetic libraries written by the test builder.

mod common;

use common::pe::{Res, Ty as PeTy, build_pe};
use common::sltg::build_sltg;
use common::sltg_libs::{IDISPATCH, IUNKNOWN, sample};
use vba_typelib::{
    Const, ConstValue, FuncFlags, FuncKind, Guid, ImplFlags, InvokeKind, LibFlags, Limits,
    ParamFlags, TypeDesc, TypeFlags, TypeInfo, TypeKind, TypeLib, TypeRef, TypelibError, VarKind,
    Version, load, load_all, parse_any, parse_msft, parse_sltg, sltg, vt,
};

fn lib() -> TypeLib {
    parse_sltg(&build_sltg(&sample())).expect("sample parses")
}

fn ty<'a>(l: &'a TypeLib, name: &str) -> &'a TypeInfo {
    l.type_by_name(name).unwrap_or_else(|| panic!("{name}")).1
}

#[test]
fn library_header() {
    let l = lib();
    assert_eq!(l.name, "Shapes");
    assert_eq!(l.doc.as_deref(), Some("Shape library"));
    assert_eq!(l.help_context, 9);
    assert_eq!(l.guid, Guid([0x11; 16]));
    assert_eq!(l.lcid, 0x409);
    assert_eq!(l.sys_kind, Some(1));
    assert_eq!(l.flags, LibFlags::HIDDEN);
    assert_eq!(l.version, Version { major: 2, minor: 5 });
    assert!(l.warnings.is_empty(), "{:?}", l.warnings);
    let names: Vec<&str> = l.types.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Color",
            "Rect",
            "IUnknown",
            "IShape",
            "IDispatch",
            "Canvas",
            "Shape",
            "Value",
            "Native"
        ]
    );
    let kinds: Vec<TypeKind> = l.types.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        [
            TypeKind::Enum,
            TypeKind::Record,
            TypeKind::Interface,
            TypeKind::Interface,
            TypeKind::Interface,
            TypeKind::Dispatch,
            TypeKind::CoClass,
            TypeKind::Union,
            TypeKind::Module
        ]
    );
}

#[test]
fn enum_members_are_valued_constants() {
    let l = lib();
    let c = ty(&l, "Color");
    let got: Vec<(&str, i32, Option<&Const>)> = c
        .variables
        .iter()
        .map(|v| (v.name.as_str(), v.memid, v.value.as_ref()))
        .collect();
    let int = |n| Const {
        vt: vt::I4,
        value: ConstValue::Int(n),
    };
    assert_eq!(
        got,
        [
            ("Red", 0, Some(&int(1))),
            ("Green", 1, Some(&int(2))),
            ("Blue", 2, Some(&int(4)))
        ]
    );
    assert!(c.variables.iter().all(|v| v.kind == VarKind::Const));
    assert_eq!(c.variables[0].ty, TypeDesc::Prim(vt::INT));
}

#[test]
fn record_and_union_fields() {
    let l = lib();
    let r = ty(&l, "Rect");
    assert_eq!(r.guid, Some(Guid([0x33; 16])));
    assert_eq!(r.instance_size, 12);
    assert_eq!(r.alignment, 4);
    let f: Vec<_> = r
        .variables
        .iter()
        .map(|v| (v.name.as_str(), v.memid, v.offset, v.kind))
        .collect();
    assert_eq!(
        f,
        [
            ("Left", 0x4000_0000, Some(0), VarKind::PerInstance),
            ("Top", 0x4000_0001, Some(4), VarKind::PerInstance),
            ("Size", 0x4000_0002, Some(8), VarKind::PerInstance),
        ]
    );
    assert_eq!(r.variables[0].ty, TypeDesc::Prim(vt::I4));
    assert_eq!(
        r.variables[2].ty,
        TypeDesc::CArray {
            elem: Box::new(TypeDesc::Prim(vt::I2)),
            bounds: vec![(2, 0), (3, 1)],
        }
    );
    let u = ty(&l, "Value");
    assert_eq!(u.kind, TypeKind::Union);
    assert_eq!(u.instance_size, 8);
    assert_eq!(u.variables.len(), 2);
    assert!(u.guid.is_none());
}

#[test]
fn interface_members_parameters_and_flags() {
    let l = lib();
    let i = ty(&l, "IShape");
    assert_eq!(i.guid, Some(Guid([0x44; 16])));
    assert_eq!(i.flags, TypeFlags::DUAL | TypeFlags::OLE_AUTOMATION);
    assert_eq!(i.version, Version { major: 1, minor: 2 });
    assert_eq!(i.vtable_size, 28);
    assert_eq!(i.implements.len(), 1);
    assert_eq!(i.implements[0].target, TypeRef::Local(2));
    assert_eq!(l.local(i.implements[0].target).unwrap().name, "IUnknown");
    assert_eq!(i.functions.len(), 4);

    let area = &i.functions[0];
    assert_eq!(area.name, "Area");
    assert_eq!(area.memid, 0x6001_0000);
    assert_eq!(area.kind, FuncKind::PureVirtual);
    assert_eq!(area.invoke, InvokeKind::Func);
    assert_eq!(area.vtable_offset, 12);
    assert_eq!(area.flags, FuncFlags::HIDDEN);
    assert_eq!(area.ret, TypeDesc::Prim(vt::R8));
    assert!(area.params.is_empty());

    let mv = &i.functions[1];
    assert_eq!(mv.ret, TypeDesc::Prim(vt::HRESULT));
    let params: Vec<_> = mv
        .params
        .iter()
        .map(|p| (p.name.as_deref(), p.ty.clone(), p.flags))
        .collect();
    assert_eq!(
        params,
        [
            (Some("dx"), TypeDesc::Prim(vt::I4), ParamFlags::IN),
            (Some("dy"), TypeDesc::Prim(vt::I4), ParamFlags::IN),
            (
                Some("r"),
                TypeDesc::Ptr(Box::new(TypeDesc::User(TypeRef::Local(1)))),
                ParamFlags::OUT
            ),
        ]
    );

    // A record without function flags, an enum parameter and an unnamed
    // SAFEARRAY parameter.
    let paint = &i.functions[2];
    assert_eq!(paint.flags, FuncFlags::empty());
    assert_eq!(paint.ret, TypeDesc::Prim(vt::VOID));
    assert_eq!(paint.params[0].ty, TypeDesc::User(TypeRef::Local(0)));
    assert_eq!(paint.params[1].name, None);
    assert_eq!(
        paint.params[1].ty,
        TypeDesc::SafeArray(Box::new(TypeDesc::Prim(vt::BSTR)))
    );

    let get = &i.functions[3];
    assert_eq!(get.invoke, InvokeKind::PropertyGet);
    assert_eq!(
        get.params[0].flags,
        ParamFlags::OUT | ParamFlags::RETVAL,
        "retval parameter"
    );
    assert_eq!(
        get.params[0].ty,
        TypeDesc::Ptr(Box::new(TypeDesc::Prim(vt::I4)))
    );
    let u = ty(&l, "IUnknown");
    assert_eq!(u.guid.unwrap().0, IUNKNOWN);
    assert_eq!(u.flags, TypeFlags::RESTRICTED);
    assert_eq!(ty(&l, "IDispatch").guid.unwrap().0, IDISPATCH);
}

#[test]
fn dispatch_members_carry_dispids_and_invoke_kinds() {
    let l = lib();
    let c = ty(&l, "Canvas");
    assert_eq!(c.flags, TypeFlags::HIDDEN | TypeFlags::DISPATCHABLE);
    assert_eq!(c.implements[0].target, TypeRef::Local(4));
    assert!(c.functions.iter().all(|f| f.kind == FuncKind::Dispatch));
    let got: Vec<_> = c
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f.memid, f.invoke))
        .collect();
    assert_eq!(
        got,
        [
            ("Title", 2, InvokeKind::PropertyGet),
            ("Title", 2, InvokeKind::PropertyPut),
            ("Item", 0, InvokeKind::PropertyGet),
            ("Clear", 5, InvokeKind::Func),
            ("Parent", 6, InvokeKind::PropertyPutRef),
            ("_NewEnum", -4, InvokeKind::PropertyGet),
        ]
    );
    assert_eq!(c.functions[0].ret, TypeDesc::Prim(vt::BSTR));
    assert_eq!(c.functions[2].ret, TypeDesc::Prim(vt::VARIANT));
    assert_eq!(c.functions[2].params[0].name.as_deref(), Some("Index"));
    assert_eq!(c.functions[5].flags, FuncFlags::RESTRICTED);
    assert_eq!(
        c.functions[5].ret,
        TypeDesc::Ptr(Box::new(TypeDesc::Prim(vt::UNKNOWN)))
    );
    assert_eq!(c.variables.len(), 1);
    assert_eq!(c.variables[0].kind, VarKind::Dispatch);
    assert_eq!(c.variables[0].memid, 1);
}

#[test]
fn imports_are_collected_and_deduplicated() {
    // Canvas lists the import as its second reference; use it twice.
    let mut s = sample();
    let canvas = &mut s.types[5];
    canvas
        .funcs
        .last_mut()
        .unwrap()
        .params
        .push(common::sltg::SParam {
            name: Some("o".to_owned()),
            ty: common::sltg::ptr(common::sltg::Ty::User(1)),
            bits: 0,
        });
    let l = parse_sltg(&build_sltg(&s)).unwrap();
    assert_eq!(l.imports.len(), 1);
    let imp = &l.imports[0];
    assert_eq!(imp.file, "stdole2.tlb");
    assert_eq!(imp.lcid, 0);
    assert_eq!(imp.version, Version { major: 2, minor: 0 });
    assert_eq!(
        imp.library_guid.unwrap().to_string(),
        common::sltg_libs::STDOLE
    );
    let f = ty(&l, "Canvas").functions.last().unwrap();
    assert_eq!(
        f.params[0].ty,
        TypeDesc::Ptr(Box::new(TypeDesc::User(TypeRef::Imported(0))))
    );
}

#[test]
fn coclass_lists_implemented_types_with_flags() {
    let l = lib();
    let c = ty(&l, "Shape");
    let got: Vec<_> = c.implements.iter().map(|i| (i.target, i.flags)).collect();
    assert_eq!(
        got,
        [
            (TypeRef::Local(3), ImplFlags::DEFAULT),
            (TypeRef::Local(5), ImplFlags::DEFAULT | ImplFlags::SOURCE),
        ]
    );
    assert_eq!(c.flags, TypeFlags::CAN_CREATE);
}

#[test]
fn module_functions_without_flags() {
    let l = lib();
    let m = ty(&l, "Native");
    assert_eq!(m.functions.len(), 1);
    assert_eq!(m.functions[0].name, "Beep");
    assert_eq!(m.functions[0].ret, TypeDesc::Prim(vt::I4));
    assert_eq!(m.functions[0].params[0].name.as_deref(), Some("hz"));
}

#[test]
fn blocks_are_read_in_chain_order() {
    let reference = lib();
    let mut s = sample();
    let n = s.types.len();
    // dir first, then the types in reverse.
    let mut order = vec![n];
    order.extend((0..n).rev());
    s.order = Some(order);
    assert_eq!(parse_sltg(&build_sltg(&s)).unwrap(), reference);
}

#[test]
fn format_is_chosen_by_the_magic() {
    let data = build_sltg(&sample());
    let a = parse_any(&data).unwrap();
    assert_eq!(a, lib());
    assert_eq!(load(&data).unwrap(), a);
    assert_eq!(load_all(&data).unwrap().len(), 1);
    // The MSFT entry point does not take SLTG data, and vice versa.
    assert!(matches!(
        parse_msft(&data),
        Err(TypelibError::UnsupportedFormat(_))
    ));
    let msft = common::builder::build(&common::sample());
    assert!(matches!(
        parse_sltg(&msft),
        Err(TypelibError::NotATypelib(_))
    ));
    assert!(parse_any(&msft).is_ok());
    assert!(matches!(
        parse_any(b"hello"),
        Err(TypelibError::NotATypelib(_))
    ));
    assert!(matches!(parse_any(b""), Err(TypelibError::NotATypelib(_))));
}

#[test]
fn sltg_resources_in_pe_images() {
    let img = build_pe(
        false,
        &[
            Res {
                ty: PeTy::Name("TYPELIB"),
                id: 1,
                lang: 0,
                data: build_sltg(&sample()),
            },
            Res {
                ty: PeTy::Name("TYPELIB"),
                id: 2,
                lang: 0,
                data: common::builder::build(&common::builder::LibSpec::new("Other")),
            },
        ],
    );
    let all = load_all(&img).unwrap();
    let got: Vec<_> = all.iter().map(|(id, l)| (*id, l.name.as_str())).collect();
    assert_eq!(got, [(1, "Shapes"), (2, "Other")]);
    assert_eq!(load(&img).unwrap(), lib());
}

#[test]
fn empty_library() {
    let l = parse_sltg(&build_sltg(&common::sltg::SLib::new("Empty"))).unwrap();
    assert_eq!(l.name, "Empty");
    assert!(l.types.is_empty());
}

#[test]
fn limits_are_honoured() {
    let data = build_sltg(&sample());
    let tight = Limits {
        max_types: 3,
        ..Limits::default()
    };
    assert_eq!(
        sltg::parse(&data, tight),
        Err(TypelibError::LimitExceeded("too many types"))
    );
    let tight = Limits {
        max_members: 4,
        ..Limits::default()
    };
    assert_eq!(
        sltg::parse(&data, tight),
        Err(TypelibError::LimitExceeded("too many members"))
    );
    let tight = Limits {
        max_params: 2,
        ..Limits::default()
    };
    assert_eq!(
        sltg::parse(&data, tight),
        Err(TypelibError::LimitExceeded("too many parameters"))
    );
    // A pointer chain deeper than the limit.
    let mut s = sample();
    let mut t = common::sltg::Ty::Prim(vt::VOID);
    for _ in 0..8 {
        t = common::sltg::Ty::SafeArray(Box::new(t));
    }
    s.types[3].funcs[0].ret = t;
    let data = build_sltg(&s);
    assert!(parse_sltg(&data).is_ok());
    let shallow = Limits {
        max_type_depth: 4,
        ..Limits::default()
    };
    assert!(matches!(
        sltg::parse(&data, shallow),
        Err(TypelibError::LimitExceeded(_))
    ));
}

#[test]
fn damaged_structures_have_typed_errors() {
    let data = build_sltg(&sample());
    // Wrong magic of the library block and of a type block.
    let at = data.windows(2).position(|w| w == [0xcc, 0x51]).unwrap();
    let mut d = data.clone();
    d[at] = 0;
    assert!(matches!(
        parse_sltg(&d),
        Err(TypelibError::Malformed {
            what: "library block",
            ..
        })
    ));
    let at = data.windows(2).position(|w| w == [0x01, 0x05]).unwrap();
    let mut d = data.clone();
    d[at] = 0;
    assert!(matches!(
        parse_sltg(&d),
        Err(TypelibError::Malformed {
            what: "type block",
            ..
        })
    ));
    // A block chain that loops.
    let mut d = data.clone();
    d[0x0a] = 0x7f;
    assert!(matches!(
        parse_sltg(&d),
        Err(TypelibError::Malformed { .. })
    ));
    // Fewer than two index names.
    let mut d = data.clone();
    d[4] = 1;
    assert!(matches!(
        parse_sltg(&d),
        Err(TypelibError::Malformed { .. })
    ));
}
