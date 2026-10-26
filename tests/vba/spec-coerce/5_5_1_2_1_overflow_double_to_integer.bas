' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, out-of-range Double to Integer is Overflow
' EXPECT: before
' ERROR: 6
Public Sub Main()
    Dim i As Integer
    Dim d As Double
    d = 40000
    Debug.Print "before"
    i = d
    Debug.Print "after"
End Sub
