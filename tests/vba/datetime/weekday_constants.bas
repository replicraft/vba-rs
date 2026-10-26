' INV: func.weekday
' SOURCE: spec ms-vbal 6.1.1.7 VbDayOfWeek constant values
' EXPECT: 1
' EXPECT: 2
' EXPECT: 3
' EXPECT: 4
' EXPECT: 5
' EXPECT: 6
' EXPECT: 7
Public Sub Main()
    Debug.Print CStr(vbSunday)
    Debug.Print CStr(vbMonday)
    Debug.Print CStr(vbTuesday)
    Debug.Print CStr(vbWednesday)
    Debug.Print CStr(vbThursday)
    Debug.Print CStr(vbFriday)
    Debug.Print CStr(vbSaturday)
End Sub
