' INV: vbal.5.2.3.2
' SOURCE: spec ms-vbal 5.2.3.2 constant name differs from enum member names
' ERROR: load
Private Enum E
    Alpha = 1
End Enum
Private Const Alpha As Long = 2
Public Sub Main()
End Sub
