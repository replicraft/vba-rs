' INV: vbal.6.1.2.4.1.3, func.datepart
' SOURCE: spec ms-vbal 6.1.2.4.1.3 DatePart interval table, Integer result
' XFAIL: wrong pending
' EXPECT: yyyy=2020
' EXPECT: q=1
' EXPECT: q4=4
' EXPECT: m=3
' EXPECT: y=64
' EXPECT: yLast=366
' EXPECT: d=4
' EXPECT: w=4
' EXPECT: wMon=3
' EXPECT: wSat=5
' EXPECT: ww=10
' EXPECT: wwJan1=1
' EXPECT: h=5
' EXPECT: n=6
' EXPECT: s=7
' EXPECT: upper=2020
' EXPECT: type=Integer
Public Sub Main()
    Dim t As Date
    t = DateSerial(2020, 3, 4) + TimeSerial(5, 6, 7)
    Debug.Print "yyyy=" & CStr(DatePart("yyyy", t))
    Debug.Print "q=" & CStr(DatePart("q", t))
    Debug.Print "q4=" & CStr(DatePart("q", DateSerial(2020, 12, 1)))
    Debug.Print "m=" & CStr(DatePart("m", t))
    Debug.Print "y=" & CStr(DatePart("y", t))
    Debug.Print "yLast=" & CStr(DatePart("y", DateSerial(2020, 12, 31)))
    Debug.Print "d=" & CStr(DatePart("d", t))
    Debug.Print "w=" & CStr(DatePart("w", t))
    Debug.Print "wMon=" & CStr(DatePart("w", t, vbMonday))
    Debug.Print "wSat=" & CStr(DatePart("w", t, vbSaturday))
    Debug.Print "ww=" & CStr(DatePart("ww", t))
    Debug.Print "wwJan1=" & CStr(DatePart("ww", DateSerial(2020, 1, 1)))
    Debug.Print "h=" & CStr(DatePart("h", t))
    Debug.Print "n=" & CStr(DatePart("n", t))
    Debug.Print "s=" & CStr(DatePart("s", t))
    Debug.Print "upper=" & CStr(DatePart("YYYY", t))
    Debug.Print "type=" & TypeName(DatePart("d", t))
End Sub
