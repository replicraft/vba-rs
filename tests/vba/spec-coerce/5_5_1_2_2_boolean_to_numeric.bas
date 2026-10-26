' INV: vbal.5.5.1.2.2, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.2 Let-coercion to and from Boolean, Boolean to numeric (Byte gets 255)
' XFAIL: wrong pending
' EXPECT: int=-1,0
' EXPECT: long=-1,0
' EXPECT: byte=255,0
' EXPECT: dbl=-1,0
' EXPECT: sng=-1,0
' EXPECT: cur=-1,0
' EXPECT: ll=-1,0
Public Sub Main()
    Dim t As Boolean, f As Boolean
    t = True: f = False
    Dim i As Integer, i2 As Integer
    Dim l As Long, l2 As Long
    Dim b As Byte, b2 As Byte
    Dim d As Double, d2 As Double
    Dim s As Single, s2 As Single
    Dim c As Currency, c2 As Currency
    Dim q As LongLong, q2 As LongLong
    i = t: i2 = f
    Debug.Print "int=" & CStr(i) & "," & CStr(i2)
    l = t: l2 = f
    Debug.Print "long=" & CStr(l) & "," & CStr(l2)
    b = t: b2 = f
    Debug.Print "byte=" & CStr(b) & "," & CStr(b2)
    d = t: d2 = f
    Debug.Print "dbl=" & CStr(d) & "," & CStr(d2)
    s = t: s2 = f
    Debug.Print "sng=" & CStr(s) & "," & CStr(s2)
    c = t: c2 = f
    Debug.Print "cur=" & CStr(c) & "," & CStr(c2)
    q = t: q2 = f
    Debug.Print "ll=" & CStr(q) & "," & CStr(q2)
End Sub
