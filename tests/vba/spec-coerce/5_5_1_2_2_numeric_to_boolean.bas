' INV: vbal.5.5.1.2.2, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.2 Let-coercion to and from Boolean, numeric to Boolean (0 is False, anything else True)
' EXPECT: 0=False
' EXPECT: 1=True
' EXPECT: -1=True
' EXPECT: 256=True
' EXPECT: 0.5=True
' EXPECT: 0.0=False
' EXPECT: byte255=True
' EXPECT: cur0.0001=True
' EXPECT: dec0=False
' EXPECT: long70000=True
Public Sub Main()
    Dim b As Boolean
    Dim d As Double
    Dim y As Byte
    Dim c As Currency
    Dim l As Long
    Dim v As Variant
    b = 0: Debug.Print "0=" & CStr(b)
    b = 1: Debug.Print "1=" & CStr(b)
    b = -1: Debug.Print "-1=" & CStr(b)
    b = 256: Debug.Print "256=" & CStr(b)
    d = 0.5: b = d: Debug.Print "0.5=" & CStr(b)
    d = 0: b = d: Debug.Print "0.0=" & CStr(b)
    y = 255: b = y: Debug.Print "byte255=" & CStr(b)
    c = 0.0001: b = c: Debug.Print "cur0.0001=" & CStr(b)
    v = CDec("0"): b = v: Debug.Print "dec0=" & CStr(b)
    l = 70000: b = l: Debug.Print "long70000=" & CStr(b)
End Sub
