' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, an integral value outside the destination range is Overflow
' EXPECT: ok=0
' ERROR: 6
Public Sub Main()
    Dim b As Byte
    Dim i As Integer
    i = 0
    b = i
    Debug.Print "ok=" & CStr(b)
    i = -1
    b = i
    Debug.Print "not reached"
End Sub
