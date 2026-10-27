' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum member referring to a following member (spec-strict: a warning by default, the program runs)
' EXPECT: 6
Private Enum Fwd
    P = Q + 1
    Q = 5
End Enum
Public Sub Main()
    Debug.Print CStr(P)
End Sub
