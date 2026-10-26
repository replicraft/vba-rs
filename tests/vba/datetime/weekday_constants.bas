' INV: func.weekday
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/weekday-function
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
