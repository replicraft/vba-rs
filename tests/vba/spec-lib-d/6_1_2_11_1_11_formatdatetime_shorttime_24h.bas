' INV: vbal.6.1.2.11.1.11, func.formatdatetime
' SOURCE: spec ms-vbal 6.1.2.11.1.11 FormatDateTime vbShortTime uses 24-hour hh:mm
' EXPECT: a=15:05
' EXPECT: b=09:07
' EXPECT: c=00:00
' EXPECT: k=4
Public Sub Main()
    Debug.Print "a=" & FormatDateTime(TimeSerial(15, 5, 30), vbShortTime)
    Debug.Print "b=" & FormatDateTime(TimeSerial(9, 7, 0), vbShortTime)
    Debug.Print "c=" & FormatDateTime(TimeSerial(0, 0, 0), vbShortTime)
    Debug.Print "k=" & CStr(vbShortTime)
End Sub
