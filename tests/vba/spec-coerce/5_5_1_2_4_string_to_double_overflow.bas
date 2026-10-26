' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, a number string beyond the Double magnitude range is Overflow
' ERROR: 6
Public Sub Main()
    Dim x As Double
    x = "1E400"
    Debug.Print "not reached"
End Sub
