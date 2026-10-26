' INV: vbal.6.1.2.4.1.5, func.datevalue
' SOURCE: spec ms-vbal 6.1.2.4.1.5 DateValue with omitted year uses the current year
' XFAIL: wrong pending
' EXPECT: noYear=True
' EXPECT: month=12
' EXPECT: day=30
Public Sub Main()
    Dim d As Variant
    d = DateValue("December 30")
    Debug.Print "noYear=" & CStr(Year(d) = Year(Date))
    Debug.Print "month=" & CStr(Month(d))
    Debug.Print "day=" & CStr(Day(d))
End Sub
