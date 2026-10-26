' INV: vbal.6.1.2.4.1.13, func.weekday
' SOURCE: spec ms-vbal 6.1.2.4.1.13 Weekday values 1..7 and FirstDayOfWeek
' EXPECT: sun=1
' EXPECT: mon=2
' EXPECT: tue=3
' EXPECT: wed=4
' EXPECT: thu=5
' EXPECT: fri=6
' EXPECT: sat=7
' EXPECT: sunFirstMon=7
' EXPECT: monFirstMon=1
' EXPECT: satFirstMon=6
' EXPECT: sunFirstSat=2
' EXPECT: satFirstSat=1
' EXPECT: wedFirstWed=1
' EXPECT: epoch=7
' EXPECT: type=Integer
' EXPECT: const=True
' EXPECT: null=True
Public Sub Main()
    ' 2023-01-01 is a Sunday
    Debug.Print "sun=" & CStr(Weekday(DateSerial(2023, 1, 1)))
    Debug.Print "mon=" & CStr(Weekday(DateSerial(2023, 1, 2)))
    Debug.Print "tue=" & CStr(Weekday(DateSerial(2023, 1, 3)))
    Debug.Print "wed=" & CStr(Weekday(DateSerial(2023, 1, 4)))
    Debug.Print "thu=" & CStr(Weekday(DateSerial(2023, 1, 5)))
    Debug.Print "fri=" & CStr(Weekday(DateSerial(2023, 1, 6)))
    Debug.Print "sat=" & CStr(Weekday(DateSerial(2023, 1, 7)))
    Debug.Print "sunFirstMon=" & CStr(Weekday(DateSerial(2023, 1, 1), vbMonday))
    Debug.Print "monFirstMon=" & CStr(Weekday(DateSerial(2023, 1, 2), vbMonday))
    Debug.Print "satFirstMon=" & CStr(Weekday(DateSerial(2023, 1, 7), vbMonday))
    Debug.Print "sunFirstSat=" & CStr(Weekday(DateSerial(2023, 1, 1), vbSaturday))
    Debug.Print "satFirstSat=" & CStr(Weekday(DateSerial(2023, 1, 7), vbSaturday))
    Debug.Print "wedFirstWed=" & CStr(Weekday(DateSerial(2023, 1, 4), vbWednesday))
    Debug.Print "epoch=" & CStr(Weekday(0))
    Debug.Print "type=" & TypeName(Weekday(DateSerial(2023, 1, 1)))
    Debug.Print "const=" & CStr(Weekday(DateSerial(2023, 1, 1)) = vbSunday)
    Debug.Print "null=" & CStr(IsNull(Weekday(Null)))
End Sub
