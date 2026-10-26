' INV: vbal.6.1.1.7
' SOURCE: spec ms-vbal 6.1.1.7 VbDayOfWeek values
' EXPECT: vbFriday=6
' EXPECT: vbMonday=2
' EXPECT: vbSaturday=7
' EXPECT: vbSunday=1
' EXPECT: vbThursday=5
' EXPECT: vbTuesday=3
' EXPECT: vbUseSystemDayOfWeek=0
' EXPECT: vbWednesday=4
Public Sub Main()
    Debug.Print "vbFriday=" & CStr(vbFriday)
    Debug.Print "vbMonday=" & CStr(vbMonday)
    Debug.Print "vbSaturday=" & CStr(vbSaturday)
    Debug.Print "vbSunday=" & CStr(vbSunday)
    Debug.Print "vbThursday=" & CStr(vbThursday)
    Debug.Print "vbTuesday=" & CStr(vbTuesday)
    Debug.Print "vbUseSystemDayOfWeek=" & CStr(vbUseSystemDayOfWeek)
    Debug.Print "vbWednesday=" & CStr(vbWednesday)
End Sub
