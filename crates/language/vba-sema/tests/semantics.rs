//! Positive semantic checks on the resulting program model.

mod common;

use common::*;
use vba_diagnostics::Severity;
use vba_project::ModuleKind;
use vba_sema::bound::*;
use vba_sema::{LibEvent, LibMember, LibParam, StaticLibrary, Type};

#[test]
fn option_base_sets_default_lower_bounds() {
    let a = analyze(&[bas(
        "M",
        "Option Base 1\nDim a(5) As Long\nDim b(0 To 2) As Long\n",
    )]);
    assert_clean(&a);
    let g = &a.program.globals;
    assert_eq!(g[0].fixed_bounds, Some(vec![(1, 5)]));
    assert_eq!(g[1].fixed_bounds, Some(vec![(0, 2)]));
}

#[test]
fn optional_defaults_are_folded_and_converted() {
    let a = analyze(&[bas(
        "M",
        "Const K = 2\nSub P(Optional x As Long = K + 3, Optional s As String = \"a\" & \"b\", Optional v As Variant)\nEnd Sub\n",
    )]);
    assert_clean(&a);
    let p = a.program.find_proc("M", "P").unwrap();
    assert_eq!(
        p.params[0].default,
        Some(Constant::Value(vba_value::Variant::Long(5)))
    );
    assert!(matches!(
        &p.params[1].default,
        Some(Constant::Value(vba_value::Variant::String(s))) if s.to_string_lossy() == "ab"
    ));
    assert_eq!(p.params[2].default, None);
}

#[test]
fn scope_precedence_local_module_project() {
    let a = analyze(&[
        bas("A", "Public Function V() As Long\nEnd Function\n"),
        bas(
            "B",
            "Private Function V() As String\nEnd Function\nSub Own()\nDim s As String\ns = V()\nEnd Sub\nSub Local()\nDim V As Long\nV = 1\nEnd Sub\n",
        ),
    ]);
    assert_clean(&a);
    // `Own` calls B.V (module scope beats the other module).
    let own = a.program.find_proc("B", "Own").unwrap();
    let BStmtKind::Assign { value, .. } = &own.body[0].kind else {
        panic!()
    };
    let BExprKind::Call(c) = &value.kind else {
        panic!("{value:?}")
    };
    let CallTarget::Proc(p) = c.target else {
        panic!()
    };
    assert_eq!(
        a.program.proc(p).module,
        a.program.module_by_name("B").unwrap().id
    );
    // `Local` assigns the local variable.
    let local = a.program.find_proc("B", "Local").unwrap();
    let BStmtKind::Assign { target, .. } = &local.body[0].kind else {
        panic!()
    };
    assert!(matches!(target, BPlace::Local(_)));
}

#[test]
fn qualified_names_reach_other_modules() {
    let a = analyze(&[
        bas(
            "Cfg",
            "Public Const K As Long = 7\nPublic Enum Mode\nFast = 1\nSlow\nEnd Enum\nPublic Function Twice(ByVal n As Long) As Long\nEnd Function\nPublic Type Pt\nx As Long\nEnd Type\n",
        ),
        bas(
            "M",
            "Sub P()\nDim n As Long, m As Cfg.Mode, p As Cfg.Pt\nn = Cfg.K + Cfg.Twice(2)\nm = Cfg.Mode.Slow\nm = Cfg.Slow\nn = Mode.Fast\np.x = VBAProject.Cfg.K\nEnd Sub\n",
        ),
    ]);
    assert_clean(&a);
}

#[test]
fn local_constants_fold_and_size_arrays() {
    let a = analyze(&[bas(
        "M",
        "Sub P()\nConst N = 3\nConst M As Long = N * 2\nDim a(1 To M) As Long\nDim s As String\ns = \"n\" & N\nEnd Sub\n",
    )]);
    assert_clean(&a);
    let p = a.program.find_proc("M", "P").unwrap();
    let arr = p.locals.iter().find(|l| l.name.to_string() == "a").unwrap();
    assert_eq!(arr.fixed_bounds, Some(vec![(1, 6)]));
}

#[test]
fn property_get_with_index_parameters() {
    let a = analyze(&[
        cls(
            "Grid",
            "Private cells(1 To 3, 1 To 3) As Long\nPublic Property Get Cell(ByVal r As Long, ByVal c As Long) As Long\nCell = cells(r, c)\nEnd Property\nPublic Property Let Cell(ByVal r As Long, ByVal c As Long, ByVal v As Long)\ncells(r, c) = v\nEnd Property\n",
        ),
        bas(
            "M",
            "Sub P()\nDim g As New Grid, n As Long\ng.Cell(1, 2) = 5\nn = g.Cell(1, 2)\nEnd Sub\n",
        ),
    ]);
    assert_clean(&a);
    let p = a.program.find_proc("M", "P").unwrap();
    let BStmtKind::Assign { target, .. } = &p.body[0].kind else {
        panic!()
    };
    let BPlace::Property { call, set, .. } = target else {
        panic!("{target:?}")
    };
    assert!(!set);
    assert_eq!(call.args.len(), 2);
}

#[test]
fn write_only_and_object_properties() {
    let a = analyze(&[
        cls(
            "C",
            "Private o As Object\nPublic Property Set Target(ByVal v As Object)\nSet o = v\nEnd Property\nPublic Property Let Level(ByVal v As Long)\nEnd Property\n",
        ),
        bas(
            "M",
            "Sub P(x As Object)\nDim c As New C\nSet c.Target = x\nc.Level = 3\nEnd Sub\n",
        ),
    ]);
    assert_clean(&a);
}

#[test]
fn set_versus_let_on_properties() {
    let a = analyze(&[
        cls(
            "C",
            "Public Property Set Target(ByVal v As Object)\nEnd Property\nPublic Property Let Level(ByVal v As Long)\nEnd Property\n",
        ),
        bas(
            "M",
            "Sub P(x As Object)\nDim c As New C\nc.Target = x\nSet c.Level = x\nEnd Sub\n",
        ),
    ]);
    assert_eq!(codes(&a), vec!["SEM007@3", "SEM007@4"]);
}

#[test]
fn nested_user_types() {
    let a = analyze(&[bas(
        "M",
        "Type In1\nv As Long\nEnd Type\nType Out\ninner As In1\narr(1 To 2) As In1\nEnd Type\nSub P()\nDim o As Out\no.inner.v = 1\no.arr(2).v = o.inner.v\nEnd Sub\n",
    )]);
    assert_clean(&a);
}

#[test]
fn friend_and_private_visibility() {
    let a = analyze(&[
        cls(
            "C",
            "Friend Sub Fr()\nEnd Sub\nPrivate Sub Pr()\nEnd Sub\nPublic Sub Run()\nPr\nMe.Pr\nEnd Sub\n",
        ),
        bas("M", "Sub P()\nDim c As New C\nc.Fr\nc.Run\nc.Pr\nEnd Sub\n"),
    ]);
    assert_eq!(codes(&a), vec!["SEM028@5"]);
}

#[test]
fn designer_modules_fall_back_to_host_members_explicitly() {
    let a = analyze(&[
        module(
            "Form1",
            ModuleKind::Designer,
            "Private Sub Command1_Click()\nText1.Text = \"a\"\nMe.Caption = \"x\"\nShow\nEnd Sub\n",
        ),
        bas(
            "M",
            "Sub P()\nForm1.Show\nForm1.Text1.Text = \"a\"\nEnd Sub\n",
        ),
    ]);
    assert_clean(&a);
    let infos = a
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Info && d.code.0 == "SEM005")
        .count();
    assert!(infos >= 4, "{}", render(&a));
}

#[test]
fn designer_modules_still_report_undeclared_variables() {
    let a = analyze(&[module(
        "Form1",
        ModuleKind::Designer,
        "Option Explicit\nPrivate Sub X()\ny = 1\nEnd Sub\n",
    )]);
    assert_eq!(codes(&a), vec!["SEM002@3"]);
}

#[test]
fn predeclared_class_instances() {
    let a = analyze(&[
        cls(
            "Singleton",
            "Attribute VB_PredeclaredId = True\nPublic Sub Hello()\nEnd Sub\n",
        ),
        cls("Plain", "Public Sub Hello()\nEnd Sub\n"),
        bas("M", "Sub P()\nSingleton.Hello\nPlain.Hello\nEnd Sub\n"),
    ]);
    assert_eq!(codes(&a), vec!["SEM028@3"]);
}

#[test]
fn library_events_and_interfaces() {
    let lib = StaticLibrary::new("Host")
        .with_type("Button", vec![LibMember::property("Caption", Type::String)])
        .with_events(
            "Button",
            vec![LibEvent {
                name: "Click".into(),
                params: vec![],
            }],
        )
        .with_type(
            "IWatcher",
            vec![LibMember::sub(
                "Notify",
                vec![LibParam::required("msg", Type::String)],
            )],
        )
        .as_interface("IWatcher");
    let a = analyze_with(
        &[
            cls(
                "C",
                "Implements Host.IWatcher\nPrivate WithEvents b As Host.Button\nPrivate Sub b_Click()\nEnd Sub\nPrivate Sub b_Nope()\nEnd Sub\n",
            ),
            cls(
                "D",
                "Implements Host.IWatcher\nPrivate Sub IWatcher_Notify(ByVal msg As String)\nEnd Sub\n",
            ),
        ],
        &profile64(),
        &lib,
    );
    assert_eq!(codes(&a), vec!["SEM011@1", "SEM017@5"]);
}

#[test]
fn implements_of_unlisted_library_interface_is_flagged_not_assumed() {
    let lib = StaticLibrary::new("Host").with_opaque_type("IThing");
    let a = analyze_with(&[cls("C", "Implements Host.IThing\n")], &profile64(), &lib);
    assert_clean(&a);
    assert!(
        a.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Info && d.code.0 == "SEM011")
    );
}

#[test]
fn longlong_and_ptrsafe_depend_on_the_profile() {
    let src = "Dim x As LongLong\nDeclare PtrSafe Sub S Lib \"k\" ()\n";
    let a = analyze_with(&[bas("M", src)], &profile32(), &test_library());
    // 32-bit targets reject LongLong; PtrSafe is optional there.
    assert!(a.diagnostics.iter().any(|d| d.severity == Severity::Error));
    let a64 = analyze_with(&[bas("M", src)], &profile64(), &test_library());
    assert_clean(&a64);
}

#[test]
fn string_comparison_modes_follow_option_compare() {
    let a = analyze(&[bas(
        "M",
        "Option Compare Text\nFunction F(a As String) As Boolean\nF = a = \"X\"\nEnd Function\n",
    )]);
    assert_clean(&a);
    let f = a.program.find_proc("M", "F").unwrap();
    let BStmtKind::Assign { value, .. } = &f.body[0].kind else {
        panic!()
    };
    let BExprKind::Binary { compare, .. } = &value.kind else {
        panic!("{value:?}")
    };
    assert_eq!(*compare, vba_ast::CompareMode::Text);
}

#[test]
fn analyses_a_project_with_conditional_compilation_constants() {
    use vba_project::{Module, VbaProject};
    let mut p = VbaProject::new("Demo");
    p.constants = "FLAG = 1 : OTHER = 0".to_owned();
    p.modules.push(Module::new(
        "Main",
        ModuleKind::Standard,
        "Option Explicit\n#If FLAG Then\nPublic Const Mode As Long = 1\n#Else\nPublic Const Mode As String = \"x\"\n#End If\n#If OTHER Then\nSub Broken()\nUndefinedThing\nEnd Sub\n#End If\nSub Run()\nDim n As Long\nn = Mode\nn = Demo.Main.Mode\nEnd Sub\n",
    ));
    let a = vba_sema::analyze_project(&p, &profile64(), &test_library());
    assert_clean(&a);
    let c = &a.program.consts[0];
    assert_eq!(c.ty, Type::Long);
    assert!(a.program.find_proc("Main", "Broken").is_none());
}
