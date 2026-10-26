' INV: func.weekday
' SOURCE: derived the Learn Weekday page: 15 June 2024 is a Saturday; the result counts from the first day of the week argument
' EXPECT: 7 6 1 7
' EXPECT: 7
Public Sub Main()
    Dim d As Date
    d = DateSerial(2024, 6, 15)
    Debug.Print CStr(Weekday(d)) & " " & CStr(Weekday(d, vbMonday)) & " " & CStr(Weekday(d, vbSaturday)) & " " & CStr(Weekday(d, vbSunday))
    Debug.Print CStr(Weekday(d + 1) + 6)
End Sub
