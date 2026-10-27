//! Table-driven tests: each case is a small project and the exact list of
//! `CODE@line` diagnostics (errors and warnings) it must produce.

mod common;

use common::*;
use vba_project::ModuleKind;

struct Case {
    name: &'static str,
    mods: &'static [(&'static str, ModuleKind, &'static str)],
    expect: &'static [&'static str],
}

use ModuleKind::{Class, Standard};

const CASES: &[Case] = &[
    // ---- declarations -------------------------------------------------
    Case {
        name: "duplicate module variable",
        mods: &[("M", Standard, "Dim x As Long\nDim x As String\n")],
        expect: &["SEM001@2"],
    },
    Case {
        name: "duplicate procedure differs only by case",
        mods: &[("M", Standard, "Sub Foo()\nEnd Sub\nSub FOO()\nEnd Sub\n")],
        expect: &["SEM001@3"],
    },
    Case {
        name: "duplicate local and parameter",
        mods: &[(
            "M",
            Standard,
            "Sub Foo(a As Long)\nDim A As Long\nDim b, B\nEnd Sub\n",
        )],
        expect: &["SEM001@2", "SEM001@3"],
    },
    Case {
        name: "property group does not clash with itself",
        mods: &[(
            "C",
            Class,
            "Private v As Long\nPublic Property Get P() As Long\nP = v\nEnd Property\nPublic Property Let P(x As Long)\nv = x\nEnd Property\n",
        )],
        expect: &[],
    },
    Case {
        name: "duplicate property getter",
        mods: &[(
            "C",
            Class,
            "Public Property Get P() As Long\nEnd Property\nPublic Property Get P() As Long\nEnd Property\n",
        )],
        expect: &["SEM001@3"],
    },
    Case {
        name: "option explicit rejects undeclared names",
        mods: &[(
            "M",
            Standard,
            "Option Explicit\nSub Foo()\nx = 1\nEnd Sub\n",
        )],
        expect: &["SEM002@3"],
    },
    Case {
        name: "implicit declaration without option explicit",
        mods: &[(
            "M",
            Standard,
            "Sub Foo()\nx = 1\ny$ = \"a\"\nz = x + 1\nEnd Sub\n",
        )],
        expect: &[],
    },
    Case {
        name: "undefined procedure is always an error",
        mods: &[("M", Standard, "Sub Foo()\nBar 1\nx = Baz(2)\nEnd Sub\n")],
        expect: &["SEM003@2", "SEM003@3"],
    },
    Case {
        name: "unresolved library type",
        mods: &[("M", Standard, "Dim r As Excel.Range\nDim w As Widget\n")],
        expect: &["SEM004@1", "SEM004@2"],
    },
    Case {
        name: "provider types resolve qualified and unqualified",
        mods: &[(
            "M",
            Standard,
            "Dim d As Scripting.Dictionary\nDim e As New Dictionary\nDim f As FileSystemObject\n",
        )],
        expect: &[],
    },
    Case {
        name: "option statements: duplicate and misplaced",
        mods: &[(
            "M",
            Standard,
            "Option Explicit\nOption Explicit\nDim x As Long\nOption Base 1\n",
        )],
        expect: &["SEM016@2", "SEM016@4"],
    },
    Case {
        name: "option private module only in standard modules",
        mods: &[("C", Class, "Option Private Module\n")],
        expect: &["SEM016@1"],
    },
    Case {
        name: "def type governs implicit types",
        mods: &[(
            "M",
            Standard,
            "DefLng A-C\nSub F()\nDim abc\nDim xyz\nabc = 1\nxyz = \"s\"\nEnd Sub\n",
        )],
        expect: &[],
    },
    Case {
        name: "constants: circular, non constant, function call",
        mods: &[(
            "M",
            Standard,
            "Const A = B + 1\nConst B = A\nDim v As Long\nConst C = v\nConst D = Len(\"x\")\nConst E = 1 / 0\n",
        )],
        expect: &["SEM010@1", "SEM010@4", "SEM010@5", "SEM010@6"],
    },
    Case {
        name: "constants fold with operators and enums",
        mods: &[(
            "M",
            Standard,
            "Public Enum E\nA = 1\nB = A * 4\nEnd Enum\nConst K As Long = E.B + 2\nConst S = \"a\" & \"b\"\nDim arr(1 To K) As Long\n",
        )],
        expect: &[],
    },
    Case {
        name: "enum value out of range",
        mods: &[("M", Standard, "Enum E\nA = 3000000000#\nEnd Enum\n")],
        expect: &["SEM027@2"],
    },
    Case {
        name: "recursive user type",
        mods: &[("M", Standard, "Type T\nx As T\nEnd Type\n")],
        expect: &["SEM027@1"],
    },
    Case {
        name: "array bounds must be ordered constants",
        mods: &[(
            "M",
            Standard,
            "Dim a(5 To 1) As Long\nDim n As Long\nDim b(n) As Long\n",
        )],
        expect: &["SEM021@1", "SEM010@3"],
    },
    Case {
        name: "a type containing itself, directly and through others",
        mods: &[(
            "M",
            Standard,
            "Private Type A\nx As A\nEnd Type\nPrivate Type B\ny As C\nEnd Type\nPrivate Type C\nz As B\nEnd Type\nPrivate Type D\nw As B\nEnd Type\nPrivate Type E\nv As Long\nEnd Type\n",
        )],
        expect: &["SEM027@1", "SEM027@4", "SEM027@7"],
    },
    // ---- parameters ----------------------------------------------------
    Case {
        name: "parameter rules",
        mods: &[(
            "M",
            Standard,
            "Sub A(Optional x As Long, y As Long)\nEnd Sub\nSub B(ParamArray p() As Variant, z As Long)\nEnd Sub\nSub C(ParamArray p() As Long)\nEnd Sub\nSub D(x As Long, x As Long)\nEnd Sub\n",
        )],
        expect: &["SEM024@1", "SEM024@3", "SEM024@5", "SEM001@7"],
    },
    // ---- properties ----------------------------------------------------
    Case {
        name: "property type pairing",
        mods: &[(
            "C",
            Class,
            "Public Property Get P() As Long\nEnd Property\nPublic Property Let P(ByVal v As String)\nEnd Property\nPublic Property Get Q(i As Long) As Long\nEnd Property\nPublic Property Let Q(ByVal v As Long)\nEnd Property\nPublic Property Set R(ByVal v As Long)\nEnd Property\n",
        )],
        expect: &["SEM012@3", "SEM012@7", "SEM012@9"],
    },
    // ---- classes -------------------------------------------------------
    Case {
        name: "class lifetime procedures",
        mods: &[(
            "C",
            Class,
            "Private Sub Class_Initialize(x As Long)\nEnd Sub\nPrivate Function Class_Terminate()\nEnd Function\n",
        )],
        expect: &["SEM023@1", "SEM023@3"],
    },
    Case {
        name: "implements: missing and mismatched members",
        mods: &[
            (
                "I",
                Class,
                "Public Sub Run(ByVal n As Long)\nEnd Sub\nPublic Function Calc() As Long\nEnd Function\nPublic Property Get Name() As String\nEnd Property\n",
            ),
            (
                "Impl",
                Class,
                "Implements I\nPrivate Sub I_Run(ByVal n As String)\nEnd Sub\nPrivate Function I_Calc() As Long\nEnd Function\n",
            ),
        ],
        expect: &["SEM011@1", "SEM011@2"],
    },
    Case {
        name: "implements: complete implementation is clean",
        mods: &[
            ("I", Class, "Public Sub Run()\nEnd Sub\n"),
            (
                "Impl",
                Class,
                "Implements I\nPrivate Sub I_Run()\nEnd Sub\n",
            ),
        ],
        expect: &[],
    },
    Case {
        name: "implements only in classes, not self, not twice, not UDT",
        mods: &[
            ("M", Standard, "Implements I\n"),
            ("I", Class, "Public Sub Run()\nEnd Sub\n"),
            (
                "S",
                Class,
                "Implements S\nImplements I\nImplements I\nPrivate Sub I_Run()\nEnd Sub\n",
            ),
        ],
        expect: &["SEM011@1", "SEM011@1", "SEM011@3"],
    },
    Case {
        name: "events and handlers",
        mods: &[
            (
                "Src",
                Class,
                "Public Event Changed(ByVal n As Long)\nPublic Event Done()\nPublic Sub Fire()\nRaiseEvent Changed(1)\nRaiseEvent Done\nRaiseEvent Nope\nEnd Sub\n",
            ),
            (
                "Sink",
                Class,
                "Private WithEvents s As Src\nPrivate WithEvents bad As Long\nPrivate Sub s_Changed(ByVal n As Long)\nEnd Sub\nPrivate Sub s_Done(ByVal x As Long)\nEnd Sub\nPrivate Sub s_Missing()\nEnd Sub\n",
            ),
        ],
        expect: &["SEM017@6", "SEM017@2", "SEM017@5", "SEM017@7"],
    },
    Case {
        name: "withevents and events only in classes",
        mods: &[(
            "M",
            Standard,
            "Private WithEvents x As Collection\nEvent E()\n",
        )],
        expect: &["SEM017@1", "SEM017@2"],
    },
    Case {
        name: "declare in a class module must be private",
        mods: &[(
            "C",
            Class,
            "Public Declare PtrSafe Sub Sleep Lib \"kernel32\" (ByVal ms As Long)\n",
        )],
        expect: &["SEM022@1"],
    },
    // ---- types and assignments ----------------------------------------
    Case {
        name: "assignment type rules",
        mods: &[(
            "M",
            Standard,
            "Type P\nx As Long\nEnd Type\nSub F()\nDim p As P, q As P, n As Long, s As String, c As Collection, v As Variant\nn = p\np = q\nn = s\ns = n\nc = New Collection\nSet n = Nothing\nSet c = Nothing\nSet v = c\nSet c = n\nv = c\nEnd Sub\n",
        )],
        expect: &["SEM006@6", "SEM007@10", "SEM007@11", "SEM006@14"],
    },
    Case {
        name: "default member lets object participate in value contexts",
        mods: &[
            (
                "Box",
                Class,
                "Public Property Get Value() As Long\nAttribute Value.VB_UserMemId = 0\nEnd Property\nPublic Property Let Value(ByVal v As Long)\nEnd Property\n",
            ),
            (
                "M",
                Standard,
                "Sub F()\nDim b As Box, n As Long\nSet b = New Box\nn = b + 1\nn = b\nb = 5\nEnd Sub\n",
            ),
        ],
        // `b = 5` assigns the default member (`Property Let Value`).
        expect: &[],
    },
    Case {
        name: "let to an object whose default member is read-only",
        mods: &[
            (
                "Box",
                Class,
                "Public Property Get Value() As Long\nAttribute Value.VB_UserMemId = 0\nEnd Property\n",
            ),
            (
                "M",
                Standard,
                "Sub F()\nDim b As Box\nSet b = New Box\nb = 5\nEnd Sub\n",
            ),
        ],
        expect: &["SEM007@4"],
    },
    Case {
        name: "class without default member",
        mods: &[
            ("Plain", Class, "Public X As Long\n"),
            (
                "M",
                Standard,
                "Sub F()\nDim p As New Plain, n As Long\nn = p\nEnd Sub\n",
            ),
        ],
        expect: &["SEM006@3"],
    },
    Case {
        name: "assignment targets",
        mods: &[(
            "M",
            Standard,
            "Const K = 1\nFunction G() As Long\nEnd Function\nSub F()\nK = 2\nG = 3\nG() = 4\nEnd Sub\n",
        )],
        expect: &["SEM025@5", "SEM025@6", "SEM025@7"],
    },
    Case {
        name: "for loop control variables",
        mods: &[(
            "M",
            Standard,
            "Sub F()\nDim s As String, i As Long, j As Long, a(3) As Long\nFor s = 1 To 3\nNext s\nFor i = 1 To 3\nNext j\nFor a(1) = 1 To 2\nNext\nFor Each i In a\nNext\nFor Each s In 5\nNext\nEnd Sub\n",
        )],
        expect: &["SEM018@3", "SEM018@6", "SEM018@7", "SEM018@9", "SEM018@11"],
    },
    // ---- calls ---------------------------------------------------------
    Case {
        name: "argument matching",
        mods: &[(
            "M",
            Standard,
            "Sub P(a As Long, Optional b As Long, Optional c As String)\nEnd Sub\nFunction Q(a As Long) As Long\nEnd Function\nSub F()\nP 1\nP 1, 2, \"x\", 4\nP\nP 1, d:=2\nP 1, a:=2\nP c:=\"x\", a:=1\nP , 2\nDim v As Long\nv = Q(1, 2)\nv = Q()\nEnd Sub\n",
        )],
        expect: &[
            "SEM008@7",
            "SEM008@8",
            "SEM008@9",
            "SEM008@10",
            "SEM008@12",
            "SEM008@14",
            "SEM008@15",
        ],
    },
    Case {
        name: "byref type mismatch",
        mods: &[(
            "M",
            Standard,
            "Sub P(ByRef a As Long)\nEnd Sub\nSub V(ByRef a As Variant)\nEnd Sub\nSub F()\nDim i As Integer, l As Long, s As String\nP i\nP l\nP (i)\nP 5\nV i\nV s\nEnd Sub\n",
        )],
        expect: &["SEM026@7"],
    },
    Case {
        name: "calling things",
        mods: &[(
            "M",
            Standard,
            "Sub S()\nEnd Sub\nSub F()\nDim n As Long, x As Long\nn = S()\nn(1) = 2\nx = n(1)\nS 1\nEnd Sub\n",
        )],
        expect: &["SEM009@5", "SEM009@6", "SEM009@7", "SEM008@8"],
    },
    Case {
        name: "late bound access on Variant and Object is allowed",
        mods: &[(
            "M",
            Standard,
            "Sub F()\nDim v As Variant, o As Object\nv = o.Foo(1, 2).Bar\no.Baz = 3\nSet o = v.Child\nv.Run\no.Items(\"a\") = 1\nv = o!Name\nEnd Sub\n",
        )],
        expect: &[],
    },
    Case {
        name: "members of project classes",
        mods: &[
            (
                "C",
                Class,
                "Public X As Long\nPrivate Y As Long\nPublic Sub Go(ByVal n As Long)\nEnd Sub\nFriend Sub Fr()\nEnd Sub\nPrivate Sub Hidden()\nEnd Sub\n",
            ),
            (
                "M",
                Standard,
                "Sub F()\nDim c As New C\nc.X = 1\nc.Y = 2\nc.Go 1\nc.Fr\nc.Hidden\nc.Nope\nEnd Sub\n",
            ),
        ],
        expect: &["SEM028@4", "SEM028@7", "SEM019@8"],
    },
    Case {
        name: "library members: described, undescribed and missing",
        mods: &[(
            "M",
            Standard,
            "Sub F()\nDim d As New Scripting.Dictionary\nDim n As Long\nd.Add \"k\", 1\nn = d.Count\nn = d(\"k\")\nd.Remove \"k\"\nd.Add 1\nDim fs As New Scripting.FileSystemObject\nfs.DeleteFile \"x\"\nEnd Sub\n",
        )],
        expect: &["SEM019@7", "SEM008@8"],
    },
    Case {
        name: "mid statement",
        mods: &[(
            "M",
            Standard,
            "Sub F()\nDim s As String, n As Long\nMid(s, 1, 2) = \"ab\"\nMid$(s, 2) = \"x\"\nMid(n, 1) = \"a\"\nMid(s) = \"x\"\nEnd Sub\n",
        )],
        expect: &["SEM006@5", "SEM008@6"],
    },
    // ---- labels, exit, with ---------------------------------------------
    Case {
        name: "labels",
        mods: &[(
            "M",
            Standard,
            "Sub F()\nGoTo Done\nlbl:\nlbl:\nOn Error GoTo Nowhere\nResume lbl\nDone:\nEnd Sub\nSub G()\nGoTo Done\nEnd Sub\n",
        )],
        expect: &["SEM013@4", "SEM013@5", "SEM013@10"],
    },
    Case {
        name: "line numbers are labels",
        mods: &[(
            "M",
            Standard,
            "Sub F()\n10 Debug.Print 1\nGoTo 10\nGoTo 20\nEnd Sub\n",
        )],
        expect: &["SEM013@4"],
    },
    Case {
        name: "exit statements",
        mods: &[(
            "M",
            Standard,
            "Sub S()\nExit Function\nExit Property\nExit For\nExit Do\nFor i = 1 To 2\nExit For\nExit Do\nNext\nDo\nExit Do\nLoop\nWhile True\nExit Do\nWend\nEnd Sub\nFunction F() As Long\nExit Sub\nExit Function\nEnd Function\n",
        )],
        expect: &[
            "SEM014@2",
            "SEM014@3",
            "SEM014@4",
            "SEM014@5",
            "SEM014@8",
            "SEM014@14",
            "SEM014@18",
        ],
    },
    Case {
        name: "with, me and bang access outside objects",
        mods: &[(
            "M",
            Standard,
            "Sub F()\nDim n As Long\nn = .X\nn = Me.Y\nn = !k\nWith n\nEnd With\nEnd Sub\n",
        )],
        expect: &["SEM020@3", "SEM020@4", "SEM020@5", "SEM020@6"],
    },
    // ---- arrays -------------------------------------------------------
    Case {
        name: "array use",
        mods: &[(
            "M",
            Standard,
            "Sub F()\nDim a(1 To 3) As Long, d() As Long, m(1, 2) As Long, n As Long\nn = a(1)\nn = a(1, 2)\nn = m(1)\nn = m(1, 1)\nReDim a(5)\nReDim d(2, 3)\nReDim d(4)\nReDim n(3)\nErase a, d\nErase n\nEnd Sub\n",
        )],
        expect: &["SEM021@4", "SEM021@5", "SEM021@7", "SEM021@10", "SEM021@12"],
    },
    // ---- cross-module scope -------------------------------------------
    Case {
        name: "ambiguous public names across modules",
        mods: &[
            (
                "A",
                Standard,
                "Public Function Dup() As Long\nEnd Function\nPublic Function OnlyA() As Long\nEnd Function\n",
            ),
            (
                "B",
                Standard,
                "Public Function Dup() As Long\nEnd Function\nPrivate Function Hidden() As Long\nEnd Function\n",
            ),
            (
                "C",
                Standard,
                "Sub F()\nDim n As Long\nn = Dup()\nn = A.Dup()\nn = B.Dup()\nn = OnlyA()\nn = Hidden()\nn = B.Hidden()\nEnd Sub\n",
            ),
        ],
        expect: &["SEM015@3", "SEM003@7", "SEM028@8"],
    },
    Case {
        name: "local and module names shadow other modules",
        mods: &[
            (
                "A",
                Standard,
                "Public Function Dup() As Long\nEnd Function\n",
            ),
            (
                "B",
                Standard,
                "Public Function Dup() As Long\nEnd Function\n",
            ),
            (
                "C",
                Standard,
                "Private Function Dup() As Long\nEnd Function\nSub F()\nDim n As Long\nn = Dup()\nEnd Sub\n",
            ),
        ],
        expect: &[],
    },
    Case {
        name: "duplicate module names",
        mods: &[("A", Standard, ""), ("a", Standard, "")],
        expect: &["SEM001@1"],
    },
    // ---- AddressOf (MS-VBAL 5.6.16.8) ---------------------------------
    Case {
        name: "AddressOf a Sub or Function of a standard module",
        mods: &[
            (
                "M",
                Standard,
                "Declare PtrSafe Function Cb Lib \"k\" (ByVal f As LongPtr) As Long\nPublic Function Tgt(ByVal a As Long, ByRef b As Long) As Long\nEnd Function\nSub S()\nEnd Sub\nSub F()\nDim n As Long, p As LongPtr\nn = Cb(AddressOf Tgt)\np = AddressOf S\np = AddressOf Other.Hi\nEnd Sub\n",
            ),
            ("Other", Standard, "Public Sub Hi()\nEnd Sub\n"),
        ],
        expect: &[],
    },
    Case {
        name: "AddressOf a private procedure of another module",
        mods: &[
            ("Other", Standard, "Private Sub Hi()\nEnd Sub\n"),
            (
                "M",
                Standard,
                "Sub F()\nDim p As LongPtr\np = AddressOf Other.Hi\nEnd Sub\n",
            ),
        ],
        expect: &["SEM028@3"],
    },
    Case {
        name: "AddressOf a procedure of a class module",
        mods: &[
            ("C", Class, "Public Sub Hi()\nEnd Sub\n"),
            (
                "M",
                Standard,
                "Sub F()\nDim p As LongPtr\nDim o As New C\np = AddressOf o.Hi\nEnd Sub\n",
            ),
        ],
        expect: &["SEM009@4"],
    },
    Case {
        name: "AddressOf inside a class module for its own procedure",
        mods: &[(
            "C",
            Class,
            "Public Sub Hi()\nEnd Sub\nPublic Sub F()\nDim p As LongPtr\np = AddressOf Hi\nEnd Sub\n",
        )],
        expect: &["SEM009@5"],
    },
    Case {
        name: "AddressOf a Declare, a property, a variable, an expression",
        mods: &[(
            "M",
            Standard,
            "Declare PtrSafe Function Cb Lib \"k\" () As Long\nDim v As Long\nProperty Get Pr() As Long\nEnd Property\nSub F()\nDim p As LongPtr\np = AddressOf Cb\np = AddressOf Pr\np = AddressOf v\np = AddressOf (F)\nEnd Sub\n",
        )],
        expect: &["SEM009@7", "SEM009@8", "SEM009@9", "SEM009@10"],
    },
    Case {
        name: "AddressOf a procedure with Optional or ParamArray parameters",
        mods: &[(
            "M",
            Standard,
            "Sub A(Optional x As Long)\nEnd Sub\nSub B(ParamArray x())\nEnd Sub\nSub F()\nDim p As LongPtr\np = AddressOf A\np = AddressOf B\nEnd Sub\n",
        )],
        expect: &["SEM009@7", "SEM009@8"],
    },
    // ---- qualified names found in real projects --------------------------
    Case {
        name: "the project name qualifies modules, enums, types and public procedures",
        mods: &[
            (
                "Cls",
                Class,
                "Attribute VB_PredeclaredId = True\nPublic Enum Mode\nA\nB\nEnd Enum\nPublic Type Pt\nx As Long\nEnd Type\n",
            ),
            (
                "N",
                Standard,
                "Public Function Helper() As Long\nHelper = 1\nEnd Function\n",
            ),
            (
                "M",
                Standard,
                "Sub F()\nDim o As Object\nDim m As VBAProject.Mode\nDim p As VBAProject.Pt\nSet o = VBAProject.Cls\nm = VBAProject.Mode.B\nm = VBAProject.Cls.Mode.A\np.x = VBAProject.Helper()\nEnd Sub\n",
            ),
        ],
        expect: &[],
    },
    Case {
        name: "the project name does not reach private or missing names",
        mods: &[
            (
                "N",
                Standard,
                "Private Function Hidden() As Long\nEnd Function\n",
            ),
            (
                "M",
                Standard,
                "Sub F()\nDim x As Long\nx = VBAProject.Hidden()\nx = VBAProject.Nothing1\nEnd Sub\n",
            ),
        ],
        expect: &["SEM019@3", "SEM019@4"],
    },
    Case {
        name: "the VBA library name qualifies functions, constants, modules and enums",
        mods: &[(
            "M",
            Standard,
            "Sub F()\nDim s As String\ns = VBA.vbNullString\ns = VBA.Left$(\"abc\", 1)\ns = VBA.Strings.Mid(\"abc\", 2)\nIf VarType(s) = VbVarType.vbString Then s = \"\"\nIf VarType(s) = VBA.VbVarType.vbString Then s = \"\"\nVBA.Interaction.MsgBox s\nWith Err\n.Raise .Number\nEnd With\nEnd Sub\n",
        )],
        expect: &[],
    },
    Case {
        name: "a name the VBA library does not have",
        mods: &[(
            "M",
            Standard,
            "Sub F()\nDim s As String\ns = VBA.NoSuchFunction(1)\nEnd Sub\n",
        )],
        expect: &["SEM019@3"],
    },
    Case {
        name: "Implements may precede Option statements",
        mods: &[
            ("I", Class, "Public Sub M()\nEnd Sub\n"),
            (
                "C",
                Class,
                "Implements I\nOption Explicit\nPrivate Sub I_M()\nEnd Sub\n",
            ),
        ],
        expect: &[],
    },
    Case {
        name: "a property with a private Get and a public Let is writable from outside",
        mods: &[
            (
                "C",
                Class,
                "Private mV As Long\nPrivate Property Get V() As Long\nV = mV\nEnd Property\nPublic Property Let V(ByVal x As Long)\nmV = x\nEnd Property\n",
            ),
            (
                "M",
                Standard,
                "Sub F()\nDim c As C\nSet c = New C\nc.V = 3\nEnd Sub\n",
            ),
        ],
        expect: &[],
    },
    Case {
        name: "Set on a write-only property through a With block",
        mods: &[
            (
                "W",
                Class,
                "Private mO As Object\nPublic Property Set Thing(ByVal v As Object)\nSet mO = v\nEnd Property\nPublic Function Make(ByVal v As Object) As W\nWith New W\nSet .Thing = v\nSet Make = .Self\nEnd With\nEnd Function\nPublic Function Self() As W\nSet Self = Me\nEnd Function\n",
            ),
            (
                "M",
                Standard,
                "Sub F(ByVal v As Object)\nDim w As W\nSet w = New W\nWith w\nSet .Thing = v\nEnd With\nEnd Sub\n",
            ),
        ],
        expect: &[],
    },
];

#[test]
fn diagnostics_table() {
    let mut failures = Vec::new();
    for c in CASES {
        let mods: Vec<_> = c.mods.iter().map(|(n, k, s)| module(n, *k, s)).collect();
        let a = analyze(&mods);
        let got = codes(&a);
        // Syntax diagnostics would mask sema problems in the table.
        let syntax: Vec<_> = a
            .diagnostics
            .iter()
            .filter(|d| {
                !d.code.0.starts_with("SEM") && d.severity == vba_diagnostics::Severity::Error
            })
            .collect();
        let expected: Vec<String> = c.expect.iter().map(|s| (*s).to_owned()).collect();
        let mut g = got.clone();
        g.sort();
        let mut e = expected.clone();
        e.sort();
        if !syntax.is_empty() || g != e {
            failures.push(format!(
                "case `{}`:\n  expected {:?}\n  got      {:?}\n{}",
                c.name,
                expected,
                got,
                render(&a)
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

#[test]
fn a_chain_of_thousands_of_nested_types_is_analysed_in_linear_time() {
    // T0 contains T1 contains ... T(n-1): no cycle, and the check must not
    // walk the whole chain once per type.
    let n = 20_000;
    let mut src = String::new();
    for k in 0..n {
        src.push_str(&format!("Private Type T{k}\nx As Long\n"));
        if k + 1 < n {
            src.push_str(&format!("y As T{}\n", k + 1));
        }
        src.push_str("End Type\n");
    }
    let a = vba_sema::analyze_sources(
        &[bas("M", &src)],
        &profile64(),
        &vba_sema::NoLibraries,
        &vba_sema::Options::default(),
    );
    assert!(
        a.diagnostics
            .iter()
            .all(|d| d.severity != vba_diagnostics::Severity::Error),
        "{:?}",
        a.diagnostics.first()
    );
}
