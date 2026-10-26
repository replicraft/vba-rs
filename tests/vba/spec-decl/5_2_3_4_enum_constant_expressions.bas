' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum member may use constant-expression incl. earlier members and constants
' EXPECT: 4 14 24 8 1 2
Private Const BaseV As Long = 4
Private Enum Calc
    First = BaseV
    Second = First + 10
    Third = Second + 10
    Shifted = 2 * BaseV
    Rem a comment between members
    Auto1 = 1
    Auto2 = Auto1 + 1
End Enum
Public Sub Main()
    Debug.Print CStr(First) & " " & CStr(Second) & " " & CStr(Third) & " " & CStr(Shifted) & " " & CStr(Auto1) & " " & CStr(Auto2)
End Sub
