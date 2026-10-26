' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum member name differs from constant names in the module
' ERROR: load
Private Const Count As Long = 3
Private Enum E
    Count = 1
End Enum
Public Sub Main()
End Sub
