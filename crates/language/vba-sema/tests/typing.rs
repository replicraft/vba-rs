//! Expression typing and call classification in the bound model.

mod common;

use common::*;
use vba_diagnostics::Severity;
use vba_sema::bound::*;
use vba_sema::{Analysis, ProcInfo};

/// Analyses `Function T()` with `decls` and returns the bound value of the
/// last assignment (`T = expr`) with a top-level conversion to the Variant
/// result stripped.
fn bound_value(decls: &str, expr: &str) -> (Analysis, BExpr) {
    let src = format!("Function T()\n{decls}\nT = {expr}\nEnd Function\n");
    let a = analyze(&[bas("M", &src)]);
    assert_clean(&a);
    let p = proc_of(&a, "T");
    let last = p
        .body
        .iter()
        .rev()
        .find_map(|s| match &s.kind {
            BStmtKind::Assign { value, .. } => Some(value.clone()),
            _ => None,
        })
        .expect("assignment");
    let value = match last.kind {
        BExprKind::Convert(inner) => *inner,
        _ => last,
    };
    (a, value)
}

fn proc_of<'a>(a: &'a Analysis, name: &str) -> &'a ProcInfo {
    a.program.find_proc("M", name).expect("proc")
}

fn type_of(decls: &str, expr: &str) -> String {
    let (a, v) = bound_value(decls, expr);
    // A constant stored into the Variant result keeps its own subtype.
    if let BExprKind::Const(Constant::Value(var)) = &v.kind {
        return a.program.type_name(&vba_sema::fold::type_of_variant(var));
    }
    a.program.type_name(&v.ty)
}

const DECLS: &str = "Dim i As Integer, l As Long, d As Double, s As Single, c As Currency, b As Boolean, str As String, v As Variant, dt As Date, by As Byte, ll As LongLong";

#[test]
fn operator_result_types() {
    let cases: &[(&str, &str)] = &[
        ("i + i", "Integer"),
        ("i + l", "Long"),
        ("i * d", "Double"),
        ("l / l", "Double"),
        ("s / s", "Single"),
        ("l \\ l", "Long"),
        ("i \\ i", "Integer"),
        ("i Mod i", "Integer"),
        ("l Mod i", "Long"),
        ("ll \\ i", "LongLong"),
        ("l ^ l", "Double"),
        ("str & str", "String"),
        ("str & v", "Variant"),
        ("i & i", "String"),
        ("i = i", "Boolean"),
        ("i < v", "Variant"),
        ("str Like \"a*\"", "Boolean"),
        ("Not b", "Boolean"),
        ("Not i", "Integer"),
        ("-b", "Integer"),
        ("-by", "Integer"),
        ("b And b", "Boolean"),
        ("i And l", "Long"),
        ("by Or by", "Byte"),
        ("c + i", "Currency"),
        ("c + s", "Currency"),
        ("c + d", "Double"),
        ("dt - dt", "Double"),
        ("dt + d", "Date"),
        ("str + str", "String"),
        ("i + v", "Variant"),
        ("by + by", "Byte"),
        ("(i)", "Integer"),
    ];
    for (expr, want) in cases {
        assert_eq!(type_of(DECLS, expr), *want, "expression `{expr}`");
    }
}

#[test]
fn literal_types() {
    let cases: &[(&str, &str)] = &[
        ("5", "Integer"),
        ("32767", "Integer"),
        ("32768", "Long"),
        ("40000", "Long"),
        ("2147483647", "Long"),
        ("2147483648", "Double"),
        ("&HFFFF", "Integer"),
        ("&H10000", "Long"),
        ("&HFFFFFFFF", "Long"),
        ("&HFFFF&", "Long"),
        ("5&", "Long"),
        ("5#", "Double"),
        ("5!", "Single"),
        ("5@", "Currency"),
        ("1.5", "Double"),
        ("1.5!", "Single"),
        ("\"s\"", "String"),
        ("True", "Boolean"),
        ("#1/2/2000#", "Date"),
        ("Empty", "Variant"),
        ("Null", "Variant"),
        ("5^", "LongLong"),
    ];
    for (expr, want) in cases {
        assert_eq!(type_of("", expr), *want, "literal `{expr}`");
    }
}

#[test]
fn hex_literal_values() {
    let (_, v) = bound_value("", "&HFFFF");
    assert_eq!(
        v.kind,
        BExprKind::Const(Constant::Value(vba_value::Variant::Integer(-1)))
    );
    let (_, v) = bound_value("", "&HFFFF&");
    assert_eq!(
        v.kind,
        BExprKind::Const(Constant::Value(vba_value::Variant::Long(65535)))
    );
}

fn head(e: &BExpr) -> String {
    match &e.kind {
        BExprKind::Call(c) => match &c.target {
            CallTarget::Proc(_) => {
                format!("proc{}", if c.receiver.is_some() { "+recv" } else { "" })
            }
            CallTarget::Declare(_) => "declare".into(),
            CallTarget::Intrinsic { name, dollar } => {
                format!("intrinsic:{name}{}", if *dollar { "$" } else { "" })
            }
        },
        BExprKind::Index { .. } => "index".into(),
        BExprKind::Late(l) => match &l.member {
            LateMember::Default => "late:default".into(),
            LateMember::Named(n) => format!("late:{n}"),
        },
        BExprKind::Const(_) => "const".into(),
        BExprKind::Local(_) => "local".into(),
        BExprKind::Global(_) => "global".into(),
        BExprKind::Field { .. } => "field".into(),
        BExprKind::UdtField { .. } => "udtfield".into(),
        BExprKind::Binary { .. } => "binary".into(),
        BExprKind::New(_) => "new".into(),
        BExprKind::LibGlobal(n) => format!("libglobal:{n}"),
        other => format!("{other:?}").chars().take(20).collect(),
    }
}

#[test]
fn call_classification() {
    let decls = "Dim a(3) As Long, v As Variant, o As Object, c As New Collection, n As Long, str As String\nDim d As New Scripting.Dictionary";
    let cases: &[(&str, &str)] = &[
        ("a(1)", "index"),
        ("v(1)", "late:default"),
        ("o(1)", "late:default"),
        ("o.Foo", "late:Foo"),
        ("o.Foo(1)", "late:Foo"),
        ("c(1)", "late:Item"),
        ("c.Count", "late:Count"),
        ("d(\"k\")", "late:Item"),
        ("d!k", "late:Item"),
        ("v!k", "late:default"),
        ("Len(str)", "intrinsic:Len"),
        ("Left$(str, 1)", "intrinsic:Left$"),
        ("VBA.Left(str, 1)", "intrinsic:Left"),
        ("Helper(1)", "proc"),
        ("Helper", "proc"),
        ("Ext(1)", "declare"),
        ("Err.Number", "late:Number"),
        ("Err", "late:Number"),
        ("vbCrLf", "const"),
        ("Erl", "intrinsic:Erl"),
        ("a(1) + 1", "binary"),
    ];
    for (expr, want) in cases {
        let src = format!(
            "Function Helper(Optional x As Long) As Long\nEnd Function\nPrivate Declare PtrSafe Function Ext Lib \"k\" (ByVal x As Long) As Long\nFunction T()\n{decls}\nT = {expr}\nEnd Function\n"
        );
        let a = analyze(&[bas("M", &src)]);
        assert_clean(&a);
        let p = proc_of(&a, "T");
        let v = p
            .body
            .iter()
            .rev()
            .find_map(|s| match &s.kind {
                BStmtKind::Assign { value, .. } => Some(value.clone()),
                _ => None,
            })
            .unwrap();
        let v = match v.kind {
            BExprKind::Convert(i) => *i,
            _ => v,
        };
        assert_eq!(head(&v), *want, "expression `{expr}`");
    }
}

#[test]
fn member_calls_use_receivers() {
    let a = analyze(&[
        cls(
            "C",
            "Public Function Get1() As Long\nEnd Function\nPublic Sub Run()\nGet1\nMe.Get1\nEnd Sub\n",
        ),
        bas("M", "Sub T()\nDim c As New C\nc.Get1\nc.Run\nEnd Sub\n"),
    ]);
    assert_clean(&a);
    let run = a.program.find_proc("C", "Run").unwrap();
    for s in &run.body {
        let BStmtKind::Call(e) = &s.kind else {
            panic!()
        };
        assert_eq!(head(e), "proc+recv");
    }
}

#[test]
fn byref_and_byval_arguments() {
    let src = "Sub P(ByRef a As Long, ByVal b As Long, c As Variant, Optional d As Long)\nEnd Sub\nSub T()\nDim x As Long, i As Integer, a(2) As Long\nP x, x, x\nP a(1), 5, i\nP (x), x + 1, 1, 2\nEnd Sub\n";
    let a = analyze(&[bas("M", src)]);
    assert_clean(&a);
    let p = proc_of(&a, "T");
    let calls: Vec<&BCall> = p
        .body
        .iter()
        .filter_map(|s| match &s.kind {
            BStmtKind::Call(BExpr {
                kind: BExprKind::Call(c),
                ..
            }) => Some(&**c),
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 3);
    // P x, x, x: ByRef Long gets the variable itself, ByVal a value, the
    // Variant parameter a Ref (same type is not required for Variant: a
    // Long variable is passed as a converted copy).
    assert!(matches!(calls[0].args[0], BArg::Ref(BPlace::Local(_))));
    assert!(matches!(calls[0].args[1], BArg::Value(_)));
    assert!(matches!(calls[0].args[2], BArg::Value(_)));
    assert!(matches!(calls[0].args[3], BArg::Missing));
    // Array element by reference; Integer to Variant ByRef is a copy.
    assert!(matches!(calls[1].args[0], BArg::Ref(BPlace::Index { .. })));
    // Parenthesised argument is by value.
    assert!(matches!(calls[2].args[0], BArg::Value(_)));
}

#[test]
fn static_locals_become_hoisted_variables() {
    let a = analyze(&[bas(
        "M",
        "Function Counter() As Long\nStatic n As Long\nn = n + 1\nCounter = n\nEnd Function\n",
    )]);
    assert_clean(&a);
    let statics: Vec<_> = a
        .program
        .globals
        .iter()
        .filter(|g| g.static_of.is_some())
        .collect();
    assert_eq!(statics.len(), 1);
    assert_eq!(statics[0].name.to_string(), "n");
}

#[test]
fn with_blocks() {
    let src = "Type P\nx As Long\nEnd Type\nSub T()\nDim p As P, c As New Collection\nWith p\n.x = 1\nEnd With\nWith c\n.Add 1\nEnd With\nEnd Sub\n";
    let a = analyze(&[bas("M", src)]);
    assert_clean(&a);
    let p = proc_of(&a, "T");
    assert!(matches!(p.body[0].kind, BStmtKind::Block(_)));
    assert!(matches!(p.body[1].kind, BStmtKind::With { .. }));
}

#[test]
fn function_result_slot() {
    let a = analyze(&[bas(
        "M",
        "Function F() As Long\nF = 1\nF = F + 1\nEnd Function\n",
    )]);
    assert_clean(&a);
    let p = proc_of(&a, "F");
    let r = p.result_local.unwrap();
    let BStmtKind::Assign { target, value, .. } = &p.body[1].kind else {
        panic!()
    };
    assert_eq!(*target, BPlace::Local(r));
    // The right-hand `F` is a recursive call.
    let BExprKind::Binary { lhs, .. } = &value.kind else {
        panic!()
    };
    assert_eq!(head(lhs), "proc");
}

#[test]
fn undescribed_library_members_are_flagged_not_guessed() {
    let a = analyze(&[bas(
        "M",
        "Sub T()\nDim fs As New Scripting.FileSystemObject\nfs.DeleteFile \"x\"\nEnd Sub\n",
    )]);
    assert_clean(&a);
    let infos: Vec<_> = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Info && d.code.0 == "SEM005")
        .collect();
    assert_eq!(infos.len(), 1, "{}", render(&a));
}

#[test]
fn implicit_declarations_use_def_types() {
    let a = analyze(&[bas(
        "M",
        "DefInt I-N\nSub T()\nk = 1\nx = 2\ns$ = \"a\"\nEnd Sub\n",
    )]);
    assert_clean(&a);
    let p = proc_of(&a, "T");
    let tys: Vec<String> = p
        .locals
        .iter()
        .map(|l| format!("{}:{}", l.name, a.program.type_name(&l.ty)))
        .collect();
    assert_eq!(tys, vec!["k:Integer", "x:Variant", "s:String"]);
}

#[test]
fn interface_call_resolves_to_interface_member() {
    let a = analyze(&[
        cls("I", "Public Sub Run()\nEnd Sub\n"),
        cls("Impl", "Implements I\nPrivate Sub I_Run()\nEnd Sub\n"),
        bas(
            "M",
            "Sub T()\nDim i As I, x As New Impl\nSet i = x\ni.Run\nEnd Sub\n",
        ),
    ]);
    assert_clean(&a);
    let imp = a.program.module_by_name("Impl").unwrap();
    assert_eq!(imp.implements.len(), 1);
    let (iface_proc, impl_proc) = imp.implements[0].members[0];
    assert_eq!(a.program.proc(iface_proc).name.to_string(), "Run");
    assert_eq!(a.program.proc(impl_proc).name.to_string(), "I_Run");
}
