' INV: vbal.5.5.1.2.3, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.3 Let-coercion to and from Date, a Double below the Date range is Overflow
' EXPECT: ok
' ERROR: 6
Public Sub Main()
    Dim d As Date
    Dim x As Double
    x = -657434
    d = x
    Debug.Print "ok"
    x = -700000
    d = x
    Debug.Print "not reached"
End Sub
