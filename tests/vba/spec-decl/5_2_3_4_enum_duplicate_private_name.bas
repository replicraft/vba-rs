' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 private enum name distinct from other enum names in the module
' ERROR: load
Private Enum E
    A1 = 1
End Enum
Private Enum E
    B1 = 2
End Enum
Public Sub Main()
End Sub
