//! Shared test helpers: a typelib builder, a PE builder and a sample library.
#![allow(dead_code, missing_docs)]

pub mod builder;
pub mod pe;

use builder::{
    Cv, Dt, FuncKind, FuncSpec, ImportSpec, InvokeKind, LibSpec, ParamSpec, R, TypeKind, TypeSpec,
    VarKind, VarSpec,
};

pub const IDISPATCH: [u8; 16] = [
    0x00, 0x04, 0x02, 0x00, 0, 0, 0, 0, 0xC0, 0, 0, 0, 0, 0, 0, 0x46,
];

pub const VT_I2: u16 = 2;
pub const VT_I4: u16 = 3;
pub const VT_R8: u16 = 5;
pub const VT_BSTR: u16 = 8;
pub const VT_BOOL: u16 = 11;
pub const VT_VARIANT: u16 = 12;
pub const VT_UI1: u16 = 17;
pub const VT_VOID: u16 = 24;
pub const VT_HRESULT: u16 = 25;

pub const IN: u32 = 1;
pub const OUT: u32 = 2;
pub const OPT: u32 = 0x10;
pub const HASDEF: u32 = 0x20;
pub const RETVAL: u32 = 8;

fn var(name: &str, memid: i32, kind: VarKind, ty: Dt) -> VarSpec {
    VarSpec {
        name: name.to_owned(),
        memid,
        kind,
        ty,
        flags: 0,
        offset: 0,
        value: None,
    }
}

/// A library covering every kind of type the model has.
///
/// Types (index): 0 `Color` enum, 1 `Rect` record, 2 `Handle` alias,
/// 3 `IShape` interface (imports `IDispatch`), 4 `Shape` dispinterface,
/// 5 `ShapeClass` coclass, 6 `Native` module.
pub fn sample() -> LibSpec {
    let mut lib = LibSpec::new("Shapes");
    lib.doc = Some("Shape library".to_owned());
    lib.version = (2, 5);
    lib.flags = 0x4;
    lib.guid = [0x11; 16];
    lib.imports.push(ImportSpec {
        file: "stdole2.tlb".to_owned(),
        lib_guid: [0x22; 16],
        type_guid: IDISPATCH,
        kind: 3,
    });

    let mut color = TypeSpec::new("Color", TypeKind::Enum);
    for (i, (n, v)) in [("Red", 1), ("Green", 2), ("Blue", 4), ("Big", 70_000_000)]
        .iter()
        .enumerate()
    {
        let mut e = var(n, i as i32, VarKind::Const, Dt::Prim(22));
        e.value = Some(Cv::I4(*v));
        color.vars.push(e);
    }
    color.vars.push({
        let mut e = var("Label", 4, VarKind::Const, Dt::Prim(VT_BSTR));
        e.value = Some(Cv::Str("blue-ish".to_owned()));
        e
    });
    lib.types.push(color);

    let mut rect = TypeSpec::new("Rect", TypeKind::Record);
    rect.guid = Some([0x33; 16]);
    rect.size = 24;
    let mut f = var("Left", 0x40000000, VarKind::PerInstance, Dt::Prim(VT_I4));
    f.offset = 0;
    rect.vars.push(f);
    let mut f = var(
        "Tag",
        0x40000001,
        VarKind::PerInstance,
        Dt::CArray(Box::new(Dt::Prim(VT_UI1)), vec![(8, 0)]),
    );
    f.offset = 4;
    rect.vars.push(f);
    let mut f = var(
        "Fill",
        0x40000002,
        VarKind::PerInstance,
        Dt::User(R::Local(0)),
    );
    f.offset = 12;
    rect.vars.push(f);
    lib.types.push(rect);

    let mut handle = TypeSpec::new("Handle", TypeKind::Alias);
    handle.alias = Some(Dt::Prim(VT_I4));
    lib.types.push(handle);

    let mut ishape = TypeSpec::new("IShape", TypeKind::Interface);
    ishape.guid = Some([0x44; 16]);
    ishape.flags = 0x40 | 0x100;
    ishape.doc = Some("A shape".to_owned());
    ishape.implements.push((R::Import(0), 0));
    ishape.vft = 8 * 5;
    let mut area = FuncSpec::new("Area", 0x10, InvokeKind::PropertyGet, Dt::Prim(VT_HRESULT));
    area.kind = FuncKind::PureVirtual;
    area.vtable_offset = 24;
    area.params.push(ParamSpec {
        name: Some("pArea".to_owned()),
        ty: Dt::Ptr(Box::new(Dt::Prim(VT_R8))),
        flags: OUT | RETVAL,
        default: None,
    });
    area.help = Some((77, "The area".to_owned()));
    ishape.funcs.push(area);
    let mut scale = FuncSpec::new("Scale", 0x11, InvokeKind::Func, Dt::Prim(VT_HRESULT));
    scale.kind = FuncKind::PureVirtual;
    scale.vtable_offset = 32;
    scale.params.push(ParamSpec {
        name: Some("factor".to_owned()),
        ty: Dt::Prim(VT_R8),
        flags: IN | OPT | HASDEF,
        default: Some(Cv::R8(1.5)),
    });
    scale.params.push(ParamSpec {
        name: Some("label".to_owned()),
        ty: Dt::Prim(VT_BSTR),
        flags: IN | OPT | HASDEF,
        default: Some(Cv::Str("none".to_owned())),
    });
    scale.params.push(ParamSpec {
        name: Some("n".to_owned()),
        ty: Dt::Prim(VT_I4),
        flags: IN | OPT | HASDEF,
        default: Some(Cv::I4(-3)),
    });
    scale.optional = 3;
    ishape.funcs.push(scale);
    lib.types.push(ishape);

    let mut shape = TypeSpec::new("Shape", TypeKind::Dispatch);
    shape.guid = Some([0x55; 16]);
    shape.flags = 0x1000;
    shape.implements.push((R::Import(0), 0));
    let mut name = FuncSpec::new("Name", 0, InvokeKind::PropertyGet, Dt::Prim(VT_BSTR));
    name.flags = 0;
    shape.funcs.push(name);
    let mut put = FuncSpec::new("Name", 0, InvokeKind::PropertyPut, Dt::Prim(VT_VOID));
    put.params.push(ParamSpec {
        name: None,
        ty: Dt::Prim(VT_BSTR),
        flags: IN,
        default: None,
    });
    shape.funcs.push(put);
    let setref = FuncSpec::new("Parent", 5, InvokeKind::PropertyPutRef, Dt::Prim(VT_VOID)).param(
        "p",
        Dt::Ptr(Box::new(Dt::User(R::Local(4)))),
        IN,
    );
    shape.funcs.push(setref);
    let mut draw = FuncSpec::new("Draw", 6, InvokeKind::Func, Dt::Prim(VT_VOID))
        .param("at", Dt::Ptr(Box::new(Dt::User(R::Local(1)))), IN)
        .param("items", Dt::Safe(Box::new(Dt::Prim(VT_VARIANT))), IN | OPT);
    draw.optional = -1;
    shape.funcs.push(draw);
    let mut w = var("Width", 8, VarKind::Dispatch, Dt::Prim(VT_I4));
    w.flags = 1;
    shape.vars.push(w);
    lib.types.push(shape);

    let mut cls = TypeSpec::new("ShapeClass", TypeKind::CoClass);
    cls.guid = Some([0x66; 16]);
    cls.flags = 2;
    cls.implements.push((R::Local(4), 1));
    cls.implements.push((R::Local(3), 0));
    lib.types.push(cls);

    let mut m = TypeSpec::new("Native", TypeKind::Module);
    m.dll = Some("shapes.dll".to_owned());
    let mut f = FuncSpec::new("MakeShape", 0x60000000, InvokeKind::Func, Dt::Prim(VT_I4));
    f.kind = FuncKind::Static;
    f.params.push(ParamSpec {
        name: Some("name".to_owned()),
        ty: Dt::Prim(VT_BSTR),
        flags: IN,
        default: None,
    });
    m.funcs.push(f);
    lib.types.push(m);
    lib
}
