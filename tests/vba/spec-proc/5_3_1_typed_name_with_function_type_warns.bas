' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 typed function name may not also have As clause (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Function F%() As Integer
    F = 1
End Function
Public Sub Main()
    Debug.Print "ok"
End Sub
