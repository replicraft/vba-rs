' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 public UDT name distinct from other public UDTs in the project (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Public Type Dup
    a As Long
End Type
Public Sub Main()
    Debug.Print "ok"
End Sub
'#MODULE Other standard
Public Type Dup
    b As Long
End Type
