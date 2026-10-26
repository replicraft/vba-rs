' INV: vbal.6.1.2.4.1.2, func.datediff
' SOURCE: spec ms-vbal 6.1.2.4.1.2 DateDiff interval table, Long result, negative when Date1 is later
' EXPECT: yyyy=1
' EXPECT: yyyy2=3
' EXPECT: q=1
' EXPECT: m=1
' EXPECT: m12=14
' EXPECT: y=10
' EXPECT: d=10
' EXPECT: h=2
' EXPECT: n=90
' EXPECT: s=3661
' EXPECT: neg=-10
' EXPECT: negyyyy=-1
' EXPECT: same=0
' EXPECT: type=Long
Public Sub Main()
    Dim a As Date
    Dim b As Date
    a = DateSerial(2020, 3, 4)
    b = DateSerial(2020, 3, 14)
    Debug.Print "yyyy=" & CStr(DateDiff("yyyy", DateSerial(2019, 12, 31), DateSerial(2020, 1, 1)))
    Debug.Print "yyyy2=" & CStr(DateDiff("yyyy", DateSerial(2017, 6, 1), DateSerial(2020, 6, 1)))
    Debug.Print "q=" & CStr(DateDiff("q", DateSerial(2019, 12, 31), DateSerial(2020, 1, 1)))
    Debug.Print "m=" & CStr(DateDiff("m", DateSerial(2020, 1, 31), DateSerial(2020, 2, 1)))
    Debug.Print "m12=" & CStr(DateDiff("m", DateSerial(2019, 1, 15), DateSerial(2020, 3, 15)))
    Debug.Print "y=" & CStr(DateDiff("y", a, b))
    Debug.Print "d=" & CStr(DateDiff("d", a, b))
    Debug.Print "h=" & CStr(DateDiff("h", a + TimeSerial(23, 0, 0), a + 1 + TimeSerial(1, 0, 0)))
    Debug.Print "n=" & CStr(DateDiff("n", a, a + TimeSerial(1, 30, 0)))
    Debug.Print "s=" & CStr(DateDiff("s", a, a + TimeSerial(1, 1, 1)))
    Debug.Print "neg=" & CStr(DateDiff("d", b, a))
    Debug.Print "negyyyy=" & CStr(DateDiff("yyyy", DateSerial(2020, 1, 1), DateSerial(2019, 12, 31)))
    Debug.Print "same=" & CStr(DateDiff("d", a, a))
    Debug.Print "type=" & TypeName(DateDiff("d", a, b))
End Sub
