' INV: vbal.6.1.2.4.1.6, vbal.6.1.2.4.1.9, vbal.6.1.2.4.1.14, func.day, func.month, func.year
' SOURCE: spec ms-vbal 6.1.2.4.1.6 Day, 6.1.2.4.1.9 Month, 6.1.2.4.1.14 Year
' EXPECT: day=4
' EXPECT: month=3
' EXPECT: year=2020
' EXPECT: dayType=Integer
' EXPECT: monthType=Integer
' EXPECT: yearType=Integer
' EXPECT: dayStr=4
' EXPECT: yearMin=100
' EXPECT: yearMax=9999
' EXPECT: dayNum=30
' EXPECT: monthNum=12
' EXPECT: yearNum=1899
' EXPECT: dayFrac=31
' EXPECT: empty=1899-12-30
' EXPECT: dayNull=True
' EXPECT: monthNull=True
' EXPECT: yearNull=True
' EXPECT: lastday=31
Public Sub Main()
    Dim d As Date
    Dim e As Variant
    d = DateSerial(2020, 3, 4)
    Debug.Print "day=" & CStr(Day(d))
    Debug.Print "month=" & CStr(Month(d))
    Debug.Print "year=" & CStr(Year(d))
    Debug.Print "dayType=" & TypeName(Day(d))
    Debug.Print "monthType=" & TypeName(Month(d))
    Debug.Print "yearType=" & TypeName(Year(d))
    Debug.Print "dayStr=" & CStr(Day("March 4, 2020"))
    Debug.Print "yearMin=" & CStr(Year(DateSerial(100, 1, 1)))
    Debug.Print "yearMax=" & CStr(Year(DateSerial(9999, 12, 31)))
    Debug.Print "dayNum=" & CStr(Day(0))
    Debug.Print "monthNum=" & CStr(Month(0))
    Debug.Print "yearNum=" & CStr(Year(0))
    Debug.Print "dayFrac=" & CStr(Day(1.5))
    Debug.Print "empty=" & CStr(Year(e)) & "-" & CStr(Month(e)) & "-" & CStr(Day(e))
    Debug.Print "dayNull=" & CStr(IsNull(Day(Null)))
    Debug.Print "monthNull=" & CStr(IsNull(Month(Null)))
    Debug.Print "yearNull=" & CStr(IsNull(Year(Null)))
    Debug.Print "lastday=" & CStr(Day(DateSerial(2020, 12, 31)))
End Sub
