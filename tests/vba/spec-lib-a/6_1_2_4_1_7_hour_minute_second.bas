' INV: vbal.6.1.2.4.1.7, vbal.6.1.2.4.1.8, vbal.6.1.2.4.1.10, func.hour, func.minute, func.second
' SOURCE: spec ms-vbal 6.1.2.4.1.7 Hour, 6.1.2.4.1.8 Minute, 6.1.2.4.1.10 Second
' EXPECT: hour=13
' EXPECT: minute=45
' EXPECT: second=59
' EXPECT: hourType=Integer
' EXPECT: minuteType=Integer
' EXPECT: secondType=Integer
' EXPECT: noon=12
' EXPECT: zero=0:0:0
' EXPECT: str=15:45:10
' EXPECT: empty=0:0:0
' EXPECT: midnightMinus=23:59:59
' EXPECT: hourNull=True
' EXPECT: minuteNull=True
' EXPECT: secondNull=True
Public Sub Main()
    Dim t As Date
    Dim e As Variant
    t = TimeSerial(13, 45, 59)
    Debug.Print "hour=" & CStr(Hour(t))
    Debug.Print "minute=" & CStr(Minute(t))
    Debug.Print "second=" & CStr(Second(t))
    Debug.Print "hourType=" & TypeName(Hour(t))
    Debug.Print "minuteType=" & TypeName(Minute(t))
    Debug.Print "secondType=" & TypeName(Second(t))
    Debug.Print "noon=" & CStr(Hour(0.5))
    Debug.Print "zero=" & CStr(Hour(0)) & ":" & CStr(Minute(0)) & ":" & CStr(Second(0))
    Debug.Print "str=" & CStr(Hour("3:45:10 PM")) & ":" & CStr(Minute("3:45:10 PM")) & ":" & CStr(Second("3:45:10 PM"))
    Debug.Print "empty=" & CStr(Hour(e)) & ":" & CStr(Minute(e)) & ":" & CStr(Second(e))
    Debug.Print "midnightMinus=" & CStr(Hour(TimeSerial(23, 59, 59))) & ":" & CStr(Minute(TimeSerial(23, 59, 59))) & ":" & CStr(Second(TimeSerial(23, 59, 59)))
    Debug.Print "hourNull=" & CStr(IsNull(Hour(Null)))
    Debug.Print "minuteNull=" & CStr(IsNull(Minute(Null)))
    Debug.Print "secondNull=" & CStr(IsNull(Second(Null)))
End Sub
