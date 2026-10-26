' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 public UDT name distinct from other public UDTs in the project
' XFAIL: wrong pending
' ERROR: load
Public Type Dup
    a As Long
End Type
Public Sub Main()
End Sub
'#MODULE Other standard
Public Type Dup
    b As Long
End Type
