' INV: vbal.5.2.3.1
' SOURCE: spec ms-vbal 5.2.3.1 module variable name differs from enum members
' ERROR: load
Private Enum E
    Alpha = 1
End Enum
Private Alpha As Long
Public Sub Main()
End Sub
