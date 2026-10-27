//! Table-driven tests: one row per grammar production (MS-VBAL §4 and §5),
//! valid forms that must parse without diagnostics and invalid forms that
//! must report a specific error while the tree stays lossless.

mod common;

use common::*;

/// Module-level snippets that must parse with no diagnostics.
const VALID_MODULE: &[&str] = &[
    // Options and attributes
    "Option Explicit",
    "Option Base 1",
    "Option Compare Text",
    "Option Compare Binary",
    "Option Compare Database",
    "Option Private Module",
    "Attribute VB_Name = \"Module1\"",
    "Attribute VB_Description = \"x\"",
    "Attribute Foo.VB_Description = \"hello\"",
    "Attribute VB_Exposed = False",
    "Attribute Foo.VB_MemberFlags = \"200\"",
    // Def statements
    "DefInt A-Z",
    "DefLng A, B, C-E",
    "DefStr S",
    "DefBool B-C, X",
    "DefVar V",
    // Implements
    "Implements IFoo",
    "Implements Excel.IFoo",
    // Variables
    "Dim x",
    "Dim x As Integer",
    "Dim x As Integer, y As String",
    "Dim x%, y$, z#",
    "Dim a(1 To 3) As Integer",
    "Dim a(1 To 3, 0 To 4) As Long",
    "Dim a() As Variant",
    "Dim a(10)",
    "Dim a As New Collection",
    "Dim s As String * 10",
    "Dim WithEvents w As Excel.Application",
    "Private x As Long",
    "Public x As Long, y As Long",
    "Global g As Integer",
    "Public Static x As Long",
    "Private p As LongPtr",
    "Dim v As Variant",
    "Dim r As Excel.Range",
    "Dim a(1 To 3) As Integer: Dim b As Long",
    // Constants
    "Const A = 1",
    "Public Const B As Long = 2",
    "Private Const C = \"x\", D = 4",
    "Const E = &HFF",
    "Const F = 1 + 2 * 3",
    "Global Const G = 10",
    // Types
    "Type Point\n    x As Long\n    y As Long\nEnd Type",
    "Private Type T\n    name As String * 20\n    arr(1 To 3) As Integer\nEnd Type",
    "Public Type Empty1\nEnd Type",
    // Enums
    "Enum Color\n    Red\n    Green = 2\n    Blue\nEnd Enum",
    "Public Enum E\n    [_Hidden] = 0\n    A = 1\nEnd Enum",
    // Events
    "Event Changed()",
    "Public Event Changed(ByVal x As Long, Optional y As String)",
    // Declares
    "Declare PtrSafe Sub Beep Lib \"kernel32\" ()",
    "Private Declare PtrSafe Function GetTickCount Lib \"kernel32\" () As Long",
    "Public Declare PtrSafe Function GetTickCount Lib \"kernel32\" () As LongLong",
    "Private Declare PtrSafe Sub Sleep Lib \"kernel32\" (ByVal ms As Long)",
    "Declare PtrSafe Function MessageBoxA Lib \"user32\" Alias \"MessageBoxA\" (ByVal h As LongPtr, ByVal t As String, ByVal c As String, ByVal f As Long) As Long",
    "Declare PtrSafe Function Foo Lib \"x\" (ByRef a As Any, b() As Byte) As Long",
    // Procedures
    "Sub A()\nEnd Sub",
    "Sub A\nEnd Sub",
    "Public Sub A(ByVal x As Long, Optional ByRef y As String = \"a\", ParamArray z() As Variant)\nEnd Sub",
    "Private Function F() As Long\nF = 1\nEnd Function",
    "Friend Function F(x As Long) As Variant\nEnd Function",
    "Static Sub S()\nEnd Sub",
    "Public Static Function F()\nEnd Function",
    "Property Get P() As Long\nEnd Property",
    "Public Property Let P(ByVal v As Long)\nEnd Property",
    "Property Set P(ByVal v As Object)\nEnd Property",
    "Function F() As String()\nEnd Function",
    "Sub A(): End Sub",
    "Sub A(x() As Long, ByVal y As Variant)\nEnd Sub",
    "Sub A()\nAttribute A.VB_Description = \"d\"\nEnd Sub",
    // Several items and comments
    "' just a comment",
    "Rem a remark",
    "Option Explicit ' trailing",
    "\n\n   \n",
    "",
];

#[test]
fn valid_module_level_productions() {
    let mut failures = Vec::new();
    for src in VALID_MODULE {
        for text in [src.to_string(), format!("{src}\n"), format!("{src}\r\n")] {
            let p = parse_str(&text);
            if !p.diagnostics.is_empty() {
                failures.push(format!("{text:?}: {}", diag_lines(&p).join("; ")));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Statement snippets (inside a `Sub`) that must parse cleanly.
const VALID_STMT: &[&str] = &[
    // Assignment and calls
    "x = 1",
    "Let x = 1",
    "Set o = New Collection",
    "Set o = Nothing",
    "a.b.c = 5",
    "a(1) = 2",
    "a(1, 2) = 3",
    ".Value = 3",
    "With x\n.Value = 3\nEnd With",
    "rs!Name = \"x\"",
    "x = rs!Name",
    "Call Foo",
    "Call Foo(1, 2)",
    "Call obj.Method(1)",
    "Foo",
    "Foo 1, 2",
    "Foo a, , b",
    "Foo x:=1, y:=2",
    "obj.Method 1, \"a\"",
    "obj.Method(1)",
    "Foo (1)",
    "Foo (1), 2",
    "Foo.Bar.Baz 1",
    "MsgBox \"a\", vbOK + vbCritical, \"T\"",
    "x = MsgBox(\"a\", vbOK)",
    "Mid$(s, 1, 2) = \"ab\"",
    "Mid(s, 1, 2) = \"ab\"",
    "Date = #1/2/2020#",
    "Debug.Print x",
    "Debug.Print x; y, z",
    "Debug.Print",
    "Debug.Print \"a\"; Tab(5); \"b\"; Spc(2)",
    "Debug.Assert x > 0",
    "DoEvents",
    "Randomize",
    "Kill \"a.txt\"",
    "LSet a = b",
    "RSet a = b",
    "x = Me.Name",
    "Me.Foo = 1",
    "x = a.b(1).c(2, 3)",
    "x = a(1)(2)",
    "x = Array(1, 2, 3)(1)",
    "x = (1 + 2) * 3",
    "x = -y",
    "x = Not y",
    "x = a And b Or c Xor d Eqv e Imp f",
    "x = a Like \"*b\"",
    "x = a Is Nothing",
    "x = TypeOf a Is Foo",
    "x = TypeOf a Is Excel.Range",
    "x = AddressOf Foo",
    "x = New Foo",
    "x = a & b & c",
    "x = a \\ b Mod c",
    "x = 2 ^ 3 ^ 4",
    "x = &H1F + &O17 + 1.5e3 + 12.5# + 3@",
    "x = \"a\"\"b\"",
    "x = #1/2/2020 3:04 PM#",
    "x = True: y = False",
    "x = Empty",
    "x = Null",
    "x = [Foreign Name]",
    "x = a![b]",
    "x = Foo(, 1)",
    "x = Foo(1, , 3)",
    "x = Foo(a:=1, b:=2)",
    "x = Len(s)",
    "x = String(5, \"a\")",
    "x = String$(5, \"a\")",
    "x = Input(5, #1)",
    "x = LBound(a)",
    "x = UBound(a, 2)",
    "x = CInt(1.5)",
    "x = Date",
    "x = Date$",
    "x = Format$(Now, \"yyyy\")",
    // Declarations in procedures
    "Dim x As Integer",
    "Static n As Long",
    "Const K = 3",
    "ReDim a(1 To 10)",
    "ReDim Preserve a(1 To 10) As Long",
    "ReDim a(5), b(1 To 3, 2) As Integer",
    "ReDim Preserve a(UBound(a) + 1)",
    "Erase a",
    "Erase a, b",
    // If forms
    "If a Then b = 1",
    "If a Then b = 1 Else c = 2",
    "If a Then b = 1: c = 2 Else d = 3: e = 4",
    "If a Then Else b = 1",
    "If a Then GoTo done",
    "If a Then 100",
    "If a Then 100 Else 200",
    "If a Then If b Then c = 1 Else d = 2",
    "If a Then\nb = 1\nEnd If",
    "If a Then\nb = 1\nElse\nc = 2\nEnd If",
    "If a Then\nb = 1\nElseIf c Then\nd = 2\nElseIf e Then\nf = 3\nElse\ng = 4\nEnd If",
    "If a Then\nIf b Then\nc = 1\nEnd If\nEnd If",
    "If a Then ' comment\nb = 1\nEnd If",
    "If a Then\nb = 1\nElse\nIf c Then\nd = 2\nEnd If\nEnd If",
    "If a Then\nEndIf",
    // Select
    "Select Case x\nCase 1\ny = 1\nCase 2, 3\ny = 2\nCase 4 To 6\ny = 3\nCase Is > 6\ny = 4\nCase Else\ny = 5\nEnd Select",
    "Select Case x\nCase \"a\", \"b\"\nEnd Select",
    "Select Case True\nCase a > 1, b < 2\nEnd Select",
    "Select Case x\nEnd Select",
    "Select Case x\nCase 1: y = 1\nCase Else: y = 2\nEnd Select",
    // Loops
    "For i = 1 To 10\nNext",
    "For i = 1 To 10\nNext i",
    "For i = 1 To 10 Step 2\nx = x + i\nNext i",
    "For i = 10 To 1 Step -1\nNext",
    "For Each c In coll\nNext c",
    "For Each c In a.b\nDebug.Print c\nNext",
    "For i = 1 To 3\nFor j = 1 To 3\nNext j, i",
    "For i = 1 To 3: For j = 1 To 3: Next j, i",
    "For i = 1 To 3\nFor j = 1 To 3\nNext j\nNext i",
    "Do\nLoop",
    "Do While x < 3\nx = x + 1\nLoop",
    "Do Until x > 3\nx = x + 1\nLoop",
    "Do\nx = x + 1\nLoop While x < 3",
    "Do\nx = x + 1\nLoop Until x > 3",
    "While x < 3\nx = x + 1\nWend",
    "With a.b\n.c = 1\nWith .d\n.e = 2\nEnd With\nEnd With",
    // Jumps and labels
    "GoTo done\ndone:",
    "GoSub sub1\nExit Sub\nsub1:\nReturn",
    "On Error GoTo handler\nhandler:",
    "On Error GoTo 0",
    "On Error GoTo -1",
    "On Error Resume Next",
    "On x GoTo l1, l2, l3\nl1:\nl2:\nl3:",
    "On x GoSub l1, l2\nl1:\nl2:",
    "Resume",
    "Resume Next",
    "Resume handler\nhandler:",
    "Exit Sub",
    "Exit Function",
    "Exit Property",
    "Exit For",
    "Exit Do",
    "Stop",
    "End",
    "10 x = 1",
    "10 x = 1\n20 y = 2",
    "100\nx = 1",
    "10: x = 1",
    "lbl: x = 1",
    "lbl:\nx = 1",
    "x = 1: y = 2: z = 3",
    "x = 1 : y = 2",
    ": x = 1",
    "x = 1 ' comment",
    "x = 1 _\n+ 2",
    "Foo a, _\nb",
    "RaiseEvent Changed",
    "RaiseEvent Changed(1, 2)",
    // File I/O
    "Open \"f.txt\" For Input As #1",
    "Open \"f.txt\" For Output As #1",
    "Open \"f.txt\" For Append As #fn",
    "Open \"f.dat\" For Binary Access Read Write As #1",
    "Open \"f.dat\" For Random As #1 Len = 100",
    "Open \"f.dat\" For Binary Access Read Lock Write As #1",
    "Open p & \"x\" For Input Shared As #1",
    "Open \"f\" As #1",
    "Close #1",
    "Close #1, #2",
    "Close",
    "Close 1",
    "Print #1, x",
    "Print #1, x; y",
    "Print #1, x, y",
    "Print #1,",
    "Print #1, Tab(5); x; Spc(3); y",
    "Print #1, \"a\" \"b\"",
    "Write #1, x, y",
    "Write #1,",
    "Input #1, a, b",
    "Line Input #1, s",
    "Get #1, , x",
    "Get #1, pos, x",
    "Put #1, , x",
    "Put #1, 5, x",
    "Seek #1, 10",
    "Lock #1",
    "Lock #1, 1 To 5",
    "Unlock #1, 3",
    "Width #1, 80",
    "Name \"a\" As \"b\"",
    "Name old As newname",
    "Name = 3",
    "Name.Foo = 3",
];

#[test]
fn valid_statement_productions() {
    let mut failures = Vec::new();
    for src in VALID_STMT {
        let full = format!("Sub T()\n{src}\nEnd Sub\n");
        let p = parse_str(&full);
        if !p.diagnostics.is_empty() {
            failures.push(format!("{src:?}: {}", diag_lines(&p).join("; ")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// (source, expected error code) for erroneous input. Parsing must stay
/// lossless and report at least that code.
const INVALID: &[(&str, &str)] = &[
    ("Option", "SYN002"),
    ("Option Foo", "SYN002"),
    ("Option Base 2", "SYN002"),
    ("Option Compare Foo", "SYN002"),
    ("Dim", "SYN002"),
    ("Dim x As", "SYN002"),
    ("Dim 5 As Integer", "SYN002"),
    ("Const A", "SYN002"),
    ("Const = 1", "SYN002"),
    ("Type\nEnd Type", "SYN002"),
    ("Type T\nx As Long", "SYN003"),
    ("Enum E\nA\n", "SYN003"),
    ("Event", "SYN002"),
    ("Declare Function F Lib", "SYN002"),
    ("Declare Function F (", "SYN002"),
    ("Sub A()\nx = \nEnd Sub", "SYN002"),
    ("Sub A()", "SYN003"),
    ("Sub A()\nEnd Function", "SYN004"),
    ("Sub A(", "SYN002"),
    ("Sub", "SYN002"),
    ("Function F() As\nEnd Function", "SYN002"),
    ("x = 1", "SYN004"),
    ("Foo 1", "SYN004"),
    ("Sub A()\nIf x Then\nEnd Sub", "SYN003"),
    ("Sub A()\nEnd If\nEnd Sub", "SYN004"),
    ("Sub A()\nFor i = 1 To 3\nEnd Sub", "SYN003"),
    ("Sub A()\nNext\nEnd Sub", "SYN004"),
    ("Sub A()\nDo\nEnd Sub", "SYN003"),
    ("Sub A()\nLoop\nEnd Sub", "SYN004"),
    ("Sub A()\nWhile x\nEnd Sub", "SYN003"),
    ("Sub A()\nWith x\nEnd Sub", "SYN003"),
    ("Sub A()\nSelect Case x\nEnd Sub", "SYN003"),
    (
        "Sub A()\nSelect Case x\ny = 1\nEnd Select\nEnd Sub",
        "SYN002",
    ),
    ("Sub A()\nx = (1 + \nEnd Sub", "SYN002"),
    ("Sub A()\nx = Foo(1\nEnd Sub", "SYN002"),
    ("Sub A()\nx = 1 +\nEnd Sub", "SYN002"),
    ("Sub A()\nx = * 2\nEnd Sub", "SYN002"),
    ("Sub A()\nIf Then\nEnd If\nEnd Sub", "SYN002"),
    ("Sub A()\nFor i = 1\nNext\nEnd Sub", "SYN002"),
    ("Sub A()\nFor Each x\nNext\nEnd Sub", "SYN002"),
    ("Sub A()\nGoTo\nEnd Sub", "SYN002"),
    ("Sub A()\nOn Error\nEnd Sub", "SYN002"),
    ("Sub A()\nExit\nEnd Sub", "SYN002"),
    ("Sub A()\nx = 1 2\nEnd Sub", "SYN002"),
    ("Sub A()\nx = )\nEnd Sub", "SYN002"),
    ("Sub A()\nOpen \"f\" For\nEnd Sub", "SYN002"),
    ("Sub A()\nPrint x\nEnd Sub", "SYN002"),
    ("Sub A()\nSet x\nEnd Sub", "SYN002"),
    ("Sub A()\nDim x As Integer, \nEnd Sub", "SYN002"),
    ("Sub A()\nDeclare Sub X Lib \"y\" ()\nEnd Sub", "SYN004"),
    ("Sub A()\nx = \"unterminated\nEnd Sub", "LEX001"),
    ("Sub A()\nx = ~\nEnd Sub", "LEX003"),
    ("Sub A()\nTypeOf\nEnd Sub", "SYN002"),
    ("Sub A()\nPublic x As Long\nEnd Sub", "SYN004"),
    ("DefInt AB", "SYN002"),
    ("DefInt", "SYN002"),
    ("Implements", "SYN002"),
    ("Attribute", "SYN002"),
    ("Attribute x", "SYN002"),
    ("Property Get", "SYN002"),
    ("End Sub", "SYN004"),
    ("End If", "SYN004"),
    (")", "SYN004"),
];

#[test]
fn invalid_productions_report_errors_and_stay_lossless() {
    for (src, code) in INVALID {
        for text in [src.to_string(), format!("{src}\n")] {
            let p = parse_str(&text);
            assert!(
                p.diagnostics.iter().any(|d| d.code.0 == *code),
                "{text:?}: expected {code}, got:\n{}",
                diag_lines(&p).join("\n")
            );
            assert!(p.has_errors(), "{text:?}: not an error");
        }
    }
}

#[test]
fn member_attribute_names_may_contain_a_dot() {
    let p = parse_clean("Sub T()\nAttribute T.VB_ProcData.VB_Invoke_Func = \"x\\n14\"\nEnd Sub\n");
    assert!(sexp(&p).contains("AttributeStmt"), "{}", sexp(&p));
    parse_clean("Attribute T.VB_ProcData.VB_Invoke_Func = \" \\n20\"\n");
}

#[test]
fn a_reserved_word_can_name_the_first_argument_of_a_call_statement() {
    assert_eq!(
        stmts("obj.Add Type:=1, Operator:=2"),
        "(CallStmt (MemberExpr (NameExpr obj) . Add) (ArgList (Arg Type := (LiteralExpr 1)) , (Arg Operator := (LiteralExpr 2))))"
    );
    assert_eq!(
        stmts("With x\n.Add Type:=1\nEnd With"),
        "(WithStmt With (NameExpr x) (Block (CallStmt (MemberExpr . Add) (ArgList (Arg Type := (LiteralExpr 1))))) End With)"
    );
}

#[test]
fn a_dot_after_blanks_in_a_call_statement_starts_an_argument() {
    assert_eq!(
        stmts("MsgBox .Name"),
        "(CallStmt (NameExpr MsgBox) (ArgList (Arg (MemberExpr . Name))))"
    );
    assert_eq!(
        stmts("Err.Raise .Number"),
        "(CallStmt (MemberExpr (NameExpr Err) . Raise) (ArgList (Arg (MemberExpr . Number))))"
    );
    assert_eq!(
        stmts("Foo .A, .B"),
        "(CallStmt (NameExpr Foo) (ArgList (Arg (MemberExpr . A)) , (Arg (MemberExpr . B))))"
    );
    // After a line continuation the dot continues the chain, and in an
    // expression a spaced dot is still a member access.
    assert_eq!(
        stmts("x.Foo _\n    .Bar = 1"),
        "(AssignStmt (MemberExpr (MemberExpr (NameExpr x) . Foo) . Bar) = (LiteralExpr 1))"
    );
    assert_eq!(
        stmts("y = x .Foo"),
        "(AssignStmt (NameExpr y) = (MemberExpr (NameExpr x) . Foo))"
    );
}
