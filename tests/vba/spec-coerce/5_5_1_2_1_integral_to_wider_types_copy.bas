' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, an integral value within the destination range is copied unchanged
' EXPECT: byte->int=255
' EXPECT: int->long=-32768
' EXPECT: long->dbl=2147483647
' EXPECT: int->byte=200
' EXPECT: long->int=-32768
' EXPECT: int->cur=-5
' EXPECT: long->sng=1000
' EXPECT: ll->var=1234567890123
Public Sub Main()
    Dim y As Byte, i As Integer, l As Long, d As Double, c As Currency, s As Single
    Dim q As LongLong
    Dim v As Variant
    y = 255: i = y: Debug.Print "byte->int=" & CStr(i)
    i = -32768: l = i: Debug.Print "int->long=" & CStr(l)
    l = 2147483647: d = l: Debug.Print "long->dbl=" & CStr(d)
    i = 200: y = i: Debug.Print "int->byte=" & CStr(y)
    l = -32768: i = l: Debug.Print "long->int=" & CStr(i)
    i = -5: c = i: Debug.Print "int->cur=" & CStr(c)
    l = 1000: s = l: Debug.Print "long->sng=" & CStr(s)
    q = CLngLng("1234567890123"): v = q: Debug.Print "ll->var=" & CStr(v)
End Sub
