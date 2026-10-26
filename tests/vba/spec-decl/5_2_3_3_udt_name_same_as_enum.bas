' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 private UDT name distinct from enum names in the module
' ERROR: load
Private Enum Same
    A1 = 1
End Enum
Private Type Same
    a As Long
End Type
Public Sub Main()
End Sub
