' INV: func.format, func.timeserial
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 13:05:09
' EXPECT: 1:05 PM
Public Sub Main()
    Debug.Print CStr(Format(TimeSerial(13, 5, 9), "hh:nn:ss"))
    Debug.Print CStr(Format(TimeSerial(13, 5, 9), "h:nn AM/PM"))
End Sub
