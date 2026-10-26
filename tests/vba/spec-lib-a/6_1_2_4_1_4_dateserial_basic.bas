' INV: vbal.6.1.2.4.1.4, func.dateserial
' SOURCE: spec ms-vbal 6.1.2.4.1.4 DateSerial normal and overflowing arguments
' EXPECT: normal=1991-12-31
' EXPECT: days35=2020-2-4
' EXPECT: month13=2021-1-1
' EXPECT: month0=2019-12-1
' EXPECT: day0=2020-2-29
' EXPECT: dayNeg=2020-2-28
' EXPECT: monthNeg=2019-11-1
' EXPECT: leap=2020-2-29
' EXPECT: noLeap=2019-3-1
' EXPECT: y1800=1800-1-1
' EXPECT: y100=100-1-1
' EXPECT: y9999=9999-12-31
' EXPECT: type=Date
' EXPECT: time=0
Private Function F(d As Variant) As String
    F = CStr(Year(d)) & "-" & CStr(Month(d)) & "-" & CStr(Day(d))
End Function
Public Sub Main()
    Debug.Print "normal=" & F(DateSerial(1991, 12, 31))
    Debug.Print "days35=" & F(DateSerial(2020, 1, 35))
    Debug.Print "month13=" & F(DateSerial(2020, 13, 1))
    Debug.Print "month0=" & F(DateSerial(2020, 0, 1))
    Debug.Print "day0=" & F(DateSerial(2020, 3, 0))
    Debug.Print "dayNeg=" & F(DateSerial(2020, 3, -1))
    Debug.Print "monthNeg=" & F(DateSerial(2020, -1, 1))
    Debug.Print "leap=" & F(DateSerial(2020, 2, 29))
    Debug.Print "noLeap=" & F(DateSerial(2019, 2, 29))
    Debug.Print "y1800=" & F(DateSerial(1800, 1, 1))
    Debug.Print "y100=" & F(DateSerial(100, 1, 1))
    Debug.Print "y9999=" & F(DateSerial(9999, 12, 31))
    Debug.Print "type=" & TypeName(DateSerial(2020, 1, 1))
    Debug.Print "time=" & CStr(CDbl(DateSerial(2020, 1, 1)) - Int(CDbl(DateSerial(2020, 1, 1))))
End Sub
