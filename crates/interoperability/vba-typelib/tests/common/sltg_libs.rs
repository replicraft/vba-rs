//! A synthetic SLTG library that covers every kind of type the reader decodes.
#![allow(dead_code, missing_docs)]

use super::sltg::{
    KIND_COCLASS, KIND_DISPATCH, KIND_ENUM, KIND_INTERFACE, KIND_MODULE, KIND_RECORD, KIND_UNION,
    SFunc, SImport, SLib, SParam, SRef, SType, SVar, Ty, ptr,
};

pub const IUNKNOWN: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0xC0, 0, 0, 0, 0, 0, 0, 0x46];
pub const IDISPATCH: [u8; 16] = [
    0x00, 0x04, 0x02, 0x00, 0, 0, 0, 0, 0xC0, 0, 0, 0, 0, 0, 0, 0x46,
];
pub const STDOLE: &str = "00020430-0000-0000-C000-000000000046";

pub const OUT: u16 = 0x4000;
pub const RETVAL: u16 = 0x4000 | 0x80;

fn param(name: &str, ty: Ty, bits: u16) -> SParam {
    SParam {
        name: Some(name.to_owned()),
        ty,
        bits,
    }
}

fn func(name: &str, invoke: u8, dispid: i32, ret: Ty, vtbl: u16) -> SFunc {
    SFunc {
        name: name.to_owned(),
        invoke,
        dispid,
        ret,
        params: Vec::new(),
        vtbl,
        flags: Some(0),
    }
}

fn var(name: &str, memid: i32, ty: Ty, byte_offs: u16, konst: bool) -> SVar {
    SVar {
        name: name.to_owned(),
        memid,
        ty,
        flags: Some(0),
        byte_offs,
        konst,
    }
}

/// Types by index: 0 `Color` enum, 1 `Rect` record, 2 `IUnknown`, 3 `IShape`
/// interface, 4 `IDispatch`, 5 `Canvas` dispinterface, 6 `Shape` coclass,
/// 7 `Value` union, 8 `Native` module.
pub fn sample() -> SLib {
    let mut lib = SLib::new("Shapes");
    lib.doc = Some("Shape library".to_owned());
    lib.help_context = 9;
    lib.guid = [0x11; 16];
    lib.flags = 0x4;
    lib.version = (2, 5);
    lib.imports.push(SImport {
        guid: STDOLE.to_owned(),
        version: (2, 0),
        lcid: 0,
        file: "stdole2.tlb".to_owned(),
    });

    let mut color = SType::new("Color", KIND_ENUM);
    for (i, (n, v)) in [("Red", 1u16), ("Green", 2), ("Blue", 4)]
        .iter()
        .enumerate()
    {
        color
            .vars
            .push(var(n, i32::try_from(i).unwrap(), Ty::Prim(22), *v, true));
    }
    lib.types.push(color);

    let mut rect = SType::new("Rect", KIND_RECORD);
    rect.guid = [0x33; 16];
    rect.size = 12;
    rect.align = 4;
    rect.vars
        .push(var("Left", 0x4000_0000, Ty::Prim(3), 0, false));
    rect.vars
        .push(var("Top", 0x4000_0001, Ty::Prim(3), 4, false));
    rect.vars.push(var(
        "Size",
        0x4000_0002,
        Ty::CArray(vec![(2, 0), (3, 1)], Box::new(Ty::Prim(2))),
        8,
        false,
    ));
    lib.types.push(rect);

    let mut unknown = SType::new("IUnknown", KIND_INTERFACE);
    unknown.guid = IUNKNOWN;
    unknown.flags = 0x200;
    unknown.vft = 12;
    lib.types.push(unknown);

    let mut ishape = SType::new("IShape", KIND_INTERFACE);
    ishape.guid = [0x44; 16];
    ishape.flags = 0x40 | 0x100;
    ishape.version = (1, 2);
    ishape.vft = 12 + 4 * 4;
    ishape.refs = vec![SRef::Local(2), SRef::Local(1), SRef::Local(0)];
    ishape.impls.push((0, 0));
    let mut area = func("Area", 1, 0x6001_0000, Ty::Prim(5), 12);
    area.flags = Some(0x40);
    ishape.funcs.push(area);
    let mut mv = func("Move", 1, 0x6001_0001, Ty::Prim(25), 16);
    mv.params.push(param("dx", Ty::Prim(3), 0));
    mv.params.push(param("dy", Ty::Prim(3), 0));
    mv.params.push(param("r", ptr(Ty::User(1)), OUT));
    ishape.funcs.push(mv);
    let mut paint = func("Paint", 1, 0x6001_0002, Ty::Prim(24), 20);
    paint.flags = None;
    paint.params.push(param("c", Ty::User(2), 0));
    paint.params.push(SParam {
        name: None,
        ty: Ty::SafeArray(Box::new(Ty::Prim(8))),
        bits: 0,
    });
    ishape.funcs.push(paint);
    let mut size = func("Size", 2, 0x6001_0003, Ty::Prim(25), 24);
    size.params.push(param("pSize", ptr(Ty::Prim(3)), RETVAL));
    ishape.funcs.push(size);
    lib.types.push(ishape);

    let mut idisp = SType::new("IDispatch", KIND_INTERFACE);
    idisp.guid = IDISPATCH;
    idisp.refs = vec![SRef::Local(2)];
    idisp.impls.push((0, 0));
    idisp.vft = 28;
    lib.types.push(idisp);

    let mut canvas = SType::new("Canvas", KIND_DISPATCH);
    canvas.guid = [0x55; 16];
    canvas.flags = 0x10 | 0x1000;
    canvas.refs = vec![SRef::Local(4), SRef::Import(0, 4)];
    canvas.impls.push((0, 0));
    let mut width = var("Width", 1, Ty::Prim(3), 0, false);
    width.flags = Some(1);
    canvas.vars.push(width);
    canvas.funcs.push(func("Title", 2, 2, Ty::Prim(8), 0));
    let mut put = func("Title", 4, 2, Ty::Prim(24), 0);
    put.params.push(param("v", Ty::Prim(8), 0));
    canvas.funcs.push(put);
    let mut item = func("Item", 2, 0, Ty::Prim(12), 0);
    item.params.push(param("Index", Ty::Prim(3), 0));
    canvas.funcs.push(item);
    canvas.funcs.push(func("Clear", 1, 5, Ty::Prim(24), 0));
    let mut parent = func("Parent", 8, 6, Ty::Prim(24), 0);
    parent.params.push(param("p", ptr(Ty::User(1)), 0));
    canvas.funcs.push(parent);
    let mut enumf = func("_NewEnum", 2, -4, ptr(Ty::Prim(13)), 0);
    enumf.flags = Some(1);
    canvas.funcs.push(enumf);
    lib.types.push(canvas);

    let mut shape = SType::new("Shape", KIND_COCLASS);
    shape.guid = [0x66; 16];
    shape.flags = 2;
    shape.refs = vec![SRef::Local(3), SRef::Local(5)];
    shape.impls.push((0, 1));
    shape.impls.push((1, 3));
    lib.types.push(shape);

    let mut value = SType::new("Value", KIND_UNION);
    value.size = 8;
    value.align = 8;
    value
        .vars
        .push(var("Int", 0x4000_0000, Ty::Prim(3), 0, false));
    value
        .vars
        .push(var("Real", 0x4000_0001, Ty::Prim(5), 0, false));
    lib.types.push(value);

    let mut native = SType::new("Native", KIND_MODULE);
    let mut beep = func("Beep", 1, 0x6000_0000, Ty::Prim(3), 0);
    beep.flags = None;
    beep.params.push(param("hz", Ty::Prim(3), 0));
    native.funcs.push(beep);
    lib.types.push(native);
    lib
}
