' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, out-of-range Currency to Byte is Overflow
' EXPECT: ok=255
' ERROR: 6
Public Sub Main()
    Dim b As Byte
    Dim c As Currency
    c = 255
    b = c
    Debug.Print "ok=" & CStr(b)
    c = 300
    b = c
    Debug.Print "not reached"
End Sub
