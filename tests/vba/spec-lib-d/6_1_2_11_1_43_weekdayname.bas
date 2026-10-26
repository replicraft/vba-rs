' INV: vbal.6.1.2.11.1.43, func.weekdayname
' SOURCE: spec ms-vbal 6.1.2.11.1.43 WeekdayName numbering depends on FirstDayOfWeek, Abbreviate
' EXPECT: a=Sunday
' EXPECT: b=Monday
' EXPECT: c=Sun
' EXPECT: d=Sat
' EXPECT: e=Sunday
' EXPECT: f=Saturday
' EXPECT: g=Wed
' EXPECT: k=1,2,7
Public Sub Main()
    Debug.Print "a=" & WeekdayName(1, False, vbSunday)
    Debug.Print "b=" & WeekdayName(2, False, vbSunday)
    Debug.Print "c=" & WeekdayName(1, True, vbSunday)
    Debug.Print "d=" & WeekdayName(7, True, vbSunday)
    Debug.Print "e=" & WeekdayName(7, False, vbMonday)
    Debug.Print "f=" & WeekdayName(1, False, vbSaturday)
    Debug.Print "g=" & WeekdayName(3, True, vbMonday)
    Debug.Print "k=" & CStr(vbSunday) & "," & CStr(vbMonday) & "," & CStr(vbSaturday)
End Sub
