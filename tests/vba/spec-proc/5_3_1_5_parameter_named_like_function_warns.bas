' INV: vbal.5.3.1.5
' SOURCE: spec ms-vbal 5.3.1.5 parameter name differs from its function name (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Function F(F As Long) As Long
End Function
Public Sub Main()
    Debug.Print "ok"
End Sub
