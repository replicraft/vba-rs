' INV: vbal.5.5.1.2.11, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.11 Let-coercion from Empty, Empty to numeric, Boolean, Date, String and String * length
' EXPECT: int=0
' EXPECT: byte=0
' EXPECT: dbl=0
' EXPECT: cur=0
' EXPECT: ll=0
' EXPECT: bool=False
' EXPECT: date=0
' EXPECT: str=[]
' EXPECT: fixed=[   ]
Public Sub Main()
    Dim v As Variant
    Dim i As Integer
    Dim y As Byte
    Dim d As Double
    Dim c As Currency
    Dim q As LongLong
    Dim b As Boolean
    Dim dt As Date
    Dim s As String
    Dim f As String * 3
    i = 7: i = v: Debug.Print "int=" & CStr(i)
    y = 7: y = v: Debug.Print "byte=" & CStr(y)
    d = 7: d = v: Debug.Print "dbl=" & CStr(d)
    c = 7: c = v: Debug.Print "cur=" & CStr(c)
    q = 7: q = v: Debug.Print "ll=" & CStr(q)
    b = True: b = v: Debug.Print "bool=" & CStr(b)
    dt = 7: dt = v: Debug.Print "date=" & CStr(CDbl(dt))
    s = "x": s = v: Debug.Print "str=[" & s & "]"
    f = "abc": f = v: Debug.Print "fixed=[" & f & "]"
End Sub
