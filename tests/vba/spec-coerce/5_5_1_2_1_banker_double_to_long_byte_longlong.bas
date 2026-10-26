' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, Double to Long, Byte and LongLong rounds half to even
' EXPECT: long 2.5=2
' EXPECT: long 3.5=4
' EXPECT: long -3.5=-4
' EXPECT: long -2.5=-2
' EXPECT: byte 0.5=0
' EXPECT: byte 1.5=2
' EXPECT: byte 2.5=2
' EXPECT: byte 254.5=254
' EXPECT: byte 253.5=254
' EXPECT: ll 2.5=2
' EXPECT: ll 3.5=4
' EXPECT: ll -2.5=-2
Public Sub Main()
    Dim l As Long
    Dim b As Byte
    Dim q As LongLong
    Dim d As Double
    d = 2.5: l = d: Debug.Print "long 2.5=" & CStr(l)
    d = 3.5: l = d: Debug.Print "long 3.5=" & CStr(l)
    d = -3.5: l = d: Debug.Print "long -3.5=" & CStr(l)
    d = -2.5: l = d: Debug.Print "long -2.5=" & CStr(l)
    d = 0.5: b = d: Debug.Print "byte 0.5=" & CStr(b)
    d = 1.5: b = d: Debug.Print "byte 1.5=" & CStr(b)
    d = 2.5: b = d: Debug.Print "byte 2.5=" & CStr(b)
    d = 254.5: b = d: Debug.Print "byte 254.5=" & CStr(b)
    d = 253.5: b = d: Debug.Print "byte 253.5=" & CStr(b)
    d = 2.5: q = d: Debug.Print "ll 2.5=" & CStr(q)
    d = 3.5: q = d: Debug.Print "ll 3.5=" & CStr(q)
    d = -2.5: q = d: Debug.Print "ll -2.5=" & CStr(q)
End Sub
