' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, scaled value out of the destination range is Overflow
' ERROR: 6
Public Sub Main()
    Dim i As Integer
    i = "1E5"
    Debug.Print "not reached"
End Sub
