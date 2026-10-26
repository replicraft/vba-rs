' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum constant-expression may not reference a following member
' XFAIL: wrong pending
' ERROR: load
Private Enum Bad
    P = Q + 1
    Q = 5
End Enum
Public Sub Main()
End Sub
