' INV: vbal.5.6.9.3.1, op.neg
' SOURCE: spec ms-vbal 5.6.9.3.1 Unary - operator, negating the minimum Integer overflows (error 6) when the operand is not a Variant
' EXPECT: ok=32767
' ERROR: 6
Public Sub Main()
    Dim i As Integer
    Dim r As Variant
    i = -32767
    r = -i
    Debug.Print "ok=" & CStr(r)
    i = -32768
    r = -i
    Debug.Print "not reached"
End Sub
