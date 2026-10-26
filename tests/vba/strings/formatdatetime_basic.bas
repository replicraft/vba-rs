' INV: func.formatdatetime, func.dateserial
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/formatdatetime-function vbLongDate uses the system long date format, dddd, MMMM d, yyyy for en-US (the day has no leading zero)
' EXPECT: 1/5/2000
' EXPECT: Wednesday, January 5, 2000
Public Sub Main()
    Debug.Print CStr(FormatDateTime(DateSerial(2000, 1, 5), vbShortDate))
    Debug.Print CStr(FormatDateTime(DateSerial(2000, 1, 5), vbLongDate))
End Sub
