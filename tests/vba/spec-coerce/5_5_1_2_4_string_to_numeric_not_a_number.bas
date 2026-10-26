' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, a string that is not a number is Type mismatch
' ERROR: 13
Public Sub Main()
    Dim i As Integer
    i = "abc"
    Debug.Print "not reached"
End Sub
