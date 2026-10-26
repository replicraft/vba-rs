' INV: vbal.6.1.1.6
' SOURCE: spec ms-vbal 6.1.1.6 VbDateTimeFormat values
' EXPECT: vbGeneralDate=0
' EXPECT: vbLongDate=1
' EXPECT: vbLongTime=3
' EXPECT: vbShortDate=2
' EXPECT: vbShortTime=4
Public Sub Main()
    Debug.Print "vbGeneralDate=" & CStr(vbGeneralDate)
    Debug.Print "vbLongDate=" & CStr(vbLongDate)
    Debug.Print "vbLongTime=" & CStr(vbLongTime)
    Debug.Print "vbShortDate=" & CStr(vbShortDate)
    Debug.Print "vbShortTime=" & CStr(vbShortTime)
End Sub
