' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, String to Date (en-US date, time, date/time, number)
' EXPECT: date=36527
' EXPECT: time=0.5
' EXPECT: datetime=36527.5
' EXPECT: number=1.5
' EXPECT: int=100
Public Sub Main()
    Dim d As Date
    d = "1/2/2000": Debug.Print "date=" & CStr(CDbl(d))
    d = "12:00:00": Debug.Print "time=" & CStr(CDbl(d))
    d = "1/2/2000 12:00:00": Debug.Print "datetime=" & CStr(CDbl(d))
    d = "1.5": Debug.Print "number=" & CStr(CDbl(d))
    d = "100": Debug.Print "int=" & CStr(CDbl(d))
End Sub
