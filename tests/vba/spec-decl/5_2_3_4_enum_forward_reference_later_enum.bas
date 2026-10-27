' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum constant-expression may not reference a member of a later enum
' STRICT_SPEC: true
' ERROR: load
Private Enum First
    P = Later1 + 1
End Enum
Private Enum Later
    Later1 = 5
End Enum
Public Sub Main()
End Sub
