' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 enum member name differs from variable names in the module
' ERROR: load
Private Count As Long
Private Enum E
    Count = 1
End Enum
Public Sub Main()
End Sub
