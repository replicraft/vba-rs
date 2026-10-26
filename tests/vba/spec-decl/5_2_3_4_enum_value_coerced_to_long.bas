' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum member constant-expression coerced to Long
' EXPECT: Long 7
' EXPECT: 8
Private Enum Coerced
    P = "7"
    Q
End Enum
Public Sub Main()
    Debug.Print TypeName(P) & " " & CStr(P)
    Debug.Print CStr(Q)
End Sub
