' INV: func.formatdatetime, func.dateserial
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 1/5/2000
' EXPECT: Wednesday, January 05, 2000
Public Sub Main()
    Debug.Print CStr(FormatDateTime(DateSerial(2000, 1, 5), vbShortDate))
    Debug.Print CStr(FormatDateTime(DateSerial(2000, 1, 5), vbLongDate))
End Sub
