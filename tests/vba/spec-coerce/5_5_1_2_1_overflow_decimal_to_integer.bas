' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, out-of-range fixed-point value to Integer is Overflow
' EXPECT: ok=-32768
' ERROR: 6
Public Sub Main()
    Dim i As Integer
    Dim v As Variant
    v = CDec("-32768")
    i = v
    Debug.Print "ok=" & CStr(i)
    v = CDec("40000")
    i = v
    Debug.Print "not reached"
End Sub
