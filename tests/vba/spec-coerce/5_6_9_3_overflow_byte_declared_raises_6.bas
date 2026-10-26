' INV: vbal.5.6.9.3, op.add
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, Byte + Byte has effective type Byte and overflow raises error 6
' EXPECT: ok=255
' ERROR: 6
Public Sub Main()
    Dim a As Byte, b As Byte
    Dim r As Variant
    a = 200: b = 55
    r = a + b
    Debug.Print "ok=" & CStr(r)
    b = 56
    r = a + b
    Debug.Print "not reached"
End Sub
