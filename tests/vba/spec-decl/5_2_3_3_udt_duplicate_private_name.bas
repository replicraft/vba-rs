' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 private UDT name distinct from other UDTs in the module
' ERROR: load
Private Type T1
    a As Long
End Type
Private Type T1
    b As Long
End Type
Public Sub Main()
End Sub
