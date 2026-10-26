' INV: vbal.6.1.1.9
' SOURCE: spec ms-vbal 6.1.1.9 VbFirstWeekOfYear values
' EXPECT: vbFirstFourDays=2
' EXPECT: vbFirstFullWeek=3
' EXPECT: vbFirstJan1=1
' EXPECT: vbUseSystem=0
Public Sub Main()
    Debug.Print "vbFirstFourDays=" & CStr(vbFirstFourDays)
    Debug.Print "vbFirstFullWeek=" & CStr(vbFirstFullWeek)
    Debug.Print "vbFirstJan1=" & CStr(vbFirstJan1)
    Debug.Print "vbUseSystem=" & CStr(vbUseSystem)
End Sub
