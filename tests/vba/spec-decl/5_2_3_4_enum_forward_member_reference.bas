' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum constant-expression may not reference a following member
' XFAIL: wrong not implemented: forward references between Enum members and Global Enum in class modules
' ERROR: load
Private Enum Bad
    P = Q + 1
    Q = 5
End Enum
Public Sub Main()
End Sub
