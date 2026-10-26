' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum constant-expression may not reference its own member
' ERROR: load
Private Enum Bad
    P = P + 1
End Enum
Public Sub Main()
End Sub
