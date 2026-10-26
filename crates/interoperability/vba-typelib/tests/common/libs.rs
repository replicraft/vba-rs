//! Synthetic libraries for the early-binding tests: one that describes the
//! native `Scripting.Dictionary` class of `vba-scripting` with the member ids
//! that class uses, and a small calculator library with every parameter
//! shape (by value, `ByRef`, optional with defaults, enum, `ParamArray`).
#![allow(dead_code)]

use super::builder::{
    Cv, Dt, FuncKind, FuncSpec, ImportSpec, InvokeKind, LibSpec, ParamSpec, R, TypeKind, TypeSpec,
    VarKind, VarSpec,
};
use super::{HASDEF, IDISPATCH, IN, OPT, VT_BOOL, VT_BSTR, VT_I4, VT_R8, VT_VARIANT, VT_VOID};

fn idispatch_import(lib: &mut LibSpec) {
    lib.imports.push(ImportSpec {
        file: "stdole2.tlb".to_owned(),
        lib_guid: [0x22; 16],
        type_guid: IDISPATCH,
        kind: 3,
    });
}

fn dispatch_func(name: &str, id: i32, invoke: InvokeKind, ret: Dt) -> FuncSpec {
    FuncSpec::new(name, id, invoke, ret)
}

fn variant_ref() -> Dt {
    Dt::Ptr(Box::new(Dt::Prim(VT_VARIANT)))
}

/// `Scripting`: `CompareMethod`, `IDictionary` and the creatable
/// `Dictionary`, with the ids of `vba_scripting::Dictionary`.
pub fn scripting() -> LibSpec {
    let mut lib = LibSpec::new("Scripting");
    lib.guid = [0x51; 16];
    idispatch_import(&mut lib);

    let mut cmp = TypeSpec::new("CompareMethod", TypeKind::Enum);
    for (i, (n, v)) in [
        ("BinaryCompare", 0),
        ("TextCompare", 1),
        ("DatabaseCompare", 2),
    ]
    .iter()
    .enumerate()
    {
        cmp.vars.push(VarSpec {
            name: (*n).to_owned(),
            memid: i as i32,
            kind: VarKind::Const,
            ty: Dt::Prim(22),
            flags: 0,
            offset: 0,
            value: Some(Cv::I4(*v)),
        });
    }
    lib.types.push(cmp);

    let mut dict = TypeSpec::new("IDictionary", TypeKind::Dispatch);
    dict.guid = Some([0x52; 16]);
    dict.flags = 0x1000;
    dict.implements.push((R::Import(0), 0));
    let key = |f: FuncSpec| f.param("Key", variant_ref(), IN);
    // Item: get, put, putref (id 0, the default member).
    dict.funcs.push(key(dispatch_func(
        "Item",
        0,
        InvokeKind::PropertyGet,
        Dt::Prim(VT_VARIANT),
    )));
    dict.funcs.push(
        key(dispatch_func(
            "Item",
            0,
            InvokeKind::PropertyPut,
            Dt::Prim(VT_VOID),
        ))
        .param("Item", variant_ref(), IN),
    );
    dict.funcs.push(
        key(dispatch_func(
            "Item",
            0,
            InvokeKind::PropertyPutRef,
            Dt::Prim(VT_VOID),
        ))
        .param("Item", variant_ref(), IN),
    );
    dict.funcs.push(
        key(dispatch_func("Add", 1, InvokeKind::Func, Dt::Prim(VT_VOID))).param(
            "Item",
            variant_ref(),
            IN,
        ),
    );
    dict.funcs.push(key(dispatch_func(
        "Exists",
        2,
        InvokeKind::Func,
        Dt::Prim(VT_BOOL),
    )));
    dict.funcs.push(dispatch_func(
        "Items",
        3,
        InvokeKind::Func,
        Dt::Prim(VT_VARIANT),
    ));
    dict.funcs.push(dispatch_func(
        "Keys",
        4,
        InvokeKind::Func,
        Dt::Prim(VT_VARIANT),
    ));
    dict.funcs.push(key(dispatch_func(
        "Remove",
        5,
        InvokeKind::Func,
        Dt::Prim(VT_VOID),
    )));
    dict.funcs.push(dispatch_func(
        "RemoveAll",
        6,
        InvokeKind::Func,
        Dt::Prim(VT_VOID),
    ));
    dict.funcs.push(dispatch_func(
        "Count",
        7,
        InvokeKind::PropertyGet,
        Dt::Prim(VT_I4),
    ));
    dict.funcs.push(
        dispatch_func("Key", 8, InvokeKind::PropertyPut, Dt::Prim(VT_VOID))
            .param("Key", variant_ref(), IN)
            .param("NewKey", variant_ref(), IN),
    );
    dict.funcs.push(dispatch_func(
        "CompareMode",
        9,
        InvokeKind::PropertyGet,
        Dt::User(R::Local(0)),
    ));
    dict.funcs.push(
        dispatch_func("CompareMode", 9, InvokeKind::PropertyPut, Dt::Prim(VT_VOID)).param(
            "pcomp",
            Dt::User(R::Local(0)),
            IN,
        ),
    );
    lib.types.push(dict);

    let mut class = TypeSpec::new("Dictionary", TypeKind::CoClass);
    class.guid = Some([0x53; 16]);
    class.flags = 2;
    class.implements.push((R::Local(1), 1));
    lib.types.push(class);
    lib
}

/// `Calc`: an interface using every parameter shape.
///
/// Types: 0 `Mode` enum (`Fast` 1, `Exact` 2), 1 `Point` record (`X`, `Y`
/// `Long`), 2 `ICalc` dispinterface, 3 `Calculator` coclass (creatable).
pub fn calc() -> LibSpec {
    let mut lib = LibSpec::new("Calc");
    lib.guid = [0x61; 16];
    idispatch_import(&mut lib);

    let mut mode = TypeSpec::new("Mode", TypeKind::Enum);
    for (i, (n, v)) in [("Fast", 1), ("Exact", 2)].iter().enumerate() {
        mode.vars.push(VarSpec {
            name: (*n).to_owned(),
            memid: i as i32,
            kind: VarKind::Const,
            ty: Dt::Prim(22),
            flags: 0,
            offset: 0,
            value: Some(Cv::I4(*v)),
        });
    }
    lib.types.push(mode);

    let mut point = TypeSpec::new("Point", TypeKind::Record);
    point.guid = Some([0x62; 16]);
    point.size = 8;
    for (i, n) in ["X", "Y"].iter().enumerate() {
        point.vars.push(VarSpec {
            name: (*n).to_owned(),
            memid: 0x4000_0000 + i as i32,
            kind: VarKind::PerInstance,
            ty: Dt::Prim(VT_I4),
            flags: 0,
            offset: 4 * i as u32,
            value: None,
        });
    }
    lib.types.push(point);

    let mut ic = TypeSpec::new("ICalc", TypeKind::Dispatch);
    ic.guid = Some([0x63; 16]);
    ic.flags = 0x1000;
    ic.implements.push((R::Import(0), 0));
    let f = |name: &str, id: i32, invoke: InvokeKind, ret: Dt| {
        let mut f = FuncSpec::new(name, id, invoke, ret);
        f.kind = FuncKind::Dispatch;
        f
    };
    // Item(Index As Long) As Long: the default member.
    ic.funcs.push(
        f("Item", 0, InvokeKind::PropertyGet, Dt::Prim(VT_I4)).param("Index", Dt::Prim(VT_I4), IN),
    );
    ic.funcs.push(
        f("Add", 1, InvokeKind::Func, Dt::Prim(VT_I4))
            .param("a", Dt::Prim(VT_I4), IN)
            .param("b", Dt::Prim(VT_I4), IN),
    );
    // Scale(ByRef v As Double, Optional factor As Double = 2.5)
    let mut scale = f("Scale", 2, InvokeKind::Func, Dt::Prim(VT_VOID)).param(
        "v",
        Dt::Ptr(Box::new(Dt::Prim(VT_R8))),
        IN | 2,
    );
    scale.params.push(ParamSpec {
        name: Some("factor".to_owned()),
        ty: Dt::Prim(VT_R8),
        flags: IN | OPT | HASDEF,
        default: Some(Cv::R8(2.5)),
    });
    scale.optional = 1;
    ic.funcs.push(scale);
    // Greet(Optional name As String = "world") As String
    let mut greet = f("Greet", 3, InvokeKind::Func, Dt::Prim(VT_BSTR));
    greet.params.push(ParamSpec {
        name: Some("name".to_owned()),
        ty: Dt::Prim(VT_BSTR),
        flags: IN | OPT | HASDEF,
        default: Some(Cv::Str("world".to_owned())),
    });
    greet.optional = 1;
    ic.funcs.push(greet);
    // Total As Long (read/write)
    ic.funcs
        .push(f("Total", 4, InvokeKind::PropertyGet, Dt::Prim(VT_I4)));
    ic.funcs.push(
        f("Total", 4, InvokeKind::PropertyPut, Dt::Prim(VT_VOID)).param("v", Dt::Prim(VT_I4), IN),
    );
    // Version As String (read-only)
    ic.funcs
        .push(f("Version", 5, InvokeKind::PropertyGet, Dt::Prim(VT_BSTR)));
    // Mode As Mode (read/write)
    ic.funcs
        .push(f("Mode", 6, InvokeKind::PropertyGet, Dt::User(R::Local(0))));
    ic.funcs.push(
        f("Mode", 6, InvokeKind::PropertyPut, Dt::Prim(VT_VOID)).param(
            "v",
            Dt::User(R::Local(0)),
            IN,
        ),
    );
    // Pick(Optional which As Mode = Fast) As Mode
    let mut pick = f("Pick", 7, InvokeKind::Func, Dt::User(R::Local(0)));
    pick.params.push(ParamSpec {
        name: Some("which".to_owned()),
        ty: Dt::User(R::Local(0)),
        flags: IN | OPT | HASDEF,
        default: Some(Cv::I4(1)),
    });
    pick.optional = 1;
    ic.funcs.push(pick);
    // Join(Sep As String, ParamArray Items()) As String
    let mut join = f("Join", 8, InvokeKind::Func, Dt::Prim(VT_BSTR))
        .param("Sep", Dt::Prim(VT_BSTR), IN)
        .param("Items", Dt::Safe(Box::new(Dt::Prim(VT_VARIANT))), IN | OPT);
    join.optional = -1;
    ic.funcs.push(join);
    // Corner(p As Point) As Long: a record by reference.
    ic.funcs
        .push(f("Corner", 9, InvokeKind::Func, Dt::Prim(VT_I4)).param(
            "p",
            Dt::Ptr(Box::new(Dt::User(R::Local(1)))),
            IN,
        ));
    // Sum(ByRef Values() As Long) As Long
    ic.funcs
        .push(f("Sum", 10, InvokeKind::Func, Dt::Prim(VT_I4)).param(
            "Values",
            Dt::Ptr(Box::new(Dt::Safe(Box::new(Dt::Prim(VT_I4))))),
            IN,
        ));
    lib.types.push(ic);

    let mut class = TypeSpec::new("Calculator", TypeKind::CoClass);
    class.guid = Some([0x64; 16]);
    class.flags = 2;
    class.implements.push((R::Local(2), 1));
    lib.types.push(class);
    lib
}
