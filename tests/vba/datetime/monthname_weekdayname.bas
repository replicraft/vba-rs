' INV: func.monthname, func.weekdayname
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: March
' EXPECT: Mar
' EXPECT: December
' EXPECT: Sunday
' EXPECT: Sun
' EXPECT: Monday
' EXPECT: Saturday
Public Sub Main()
    Debug.Print CStr(MonthName(3))
    Debug.Print CStr(MonthName(3, True))
    Debug.Print CStr(MonthName(12))
    Debug.Print CStr(WeekdayName(1))
    Debug.Print CStr(WeekdayName(1, True))
    Debug.Print CStr(WeekdayName(1, False, vbMonday))
    Debug.Print CStr(WeekdayName(7))
End Sub
