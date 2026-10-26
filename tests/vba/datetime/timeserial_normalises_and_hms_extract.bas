' INV: func.timeserial
' SOURCE: derived the Learn TimeSerial page: minutes and seconds outside 0-59 carry into the next larger unit; Hour, Minute and Second extract the parts
' EXPECT: 1 30 0
' EXPECT: 13 5 9
' EXPECT: 13:05:09
' EXPECT: 2 1 1
Public Sub Main()
    Dim t As Date
    t = TimeSerial(0, 90, 0)
    Debug.Print CStr(Hour(t)) & " " & CStr(Minute(t)) & " " & CStr(Second(t))
    t = TimeSerial(13, 5, 9)
    Debug.Print CStr(Hour(t)) & " " & CStr(Minute(t)) & " " & CStr(Second(t))
    Debug.Print Format(t, "hh:nn:ss")
    t = TimeSerial(1, 60, 61)
    Debug.Print CStr(Hour(t)) & " " & CStr(Minute(t)) & " " & CStr(Second(t))
End Sub
