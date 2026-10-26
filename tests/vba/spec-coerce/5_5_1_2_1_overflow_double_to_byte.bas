' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, out-of-range value to Byte is Overflow
' EXPECT: ok=255
' ERROR: 6
Public Sub Main()
    Dim b As Byte
    Dim d As Double
    d = 255
    b = d
    Debug.Print "ok=" & CStr(b)
    d = 256
    b = d
    Debug.Print "not reached"
End Sub
