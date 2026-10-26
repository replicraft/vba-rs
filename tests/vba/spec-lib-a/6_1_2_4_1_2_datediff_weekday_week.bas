' INV: vbal.6.1.2.4.1.2, func.datediff
' SOURCE: spec ms-vbal 6.1.2.4.1.2 DateDiff w and ww count first-day-of-week occurrences
' EXPECT: w_mon_to_mon7=1
' EXPECT: w_mon_to_mon14=2
' EXPECT: w_mon_to_sun=0
' EXPECT: w_sun_to_sun=1
' EXPECT: ww_sun_start=0
' EXPECT: ww_mon_to_sun=1
' EXPECT: ww_two=2
' EXPECT: ww_firstMonday=1
' EXPECT: ww_sunday_default=0
Public Sub Main()
    ' 2023-01-01 is a Sunday, 2023-01-02 a Monday
    Debug.Print "w_mon_to_mon7=" & CStr(DateDiff("w", DateSerial(2023, 1, 2), DateSerial(2023, 1, 9)))
    Debug.Print "w_mon_to_mon14=" & CStr(DateDiff("w", DateSerial(2023, 1, 2), DateSerial(2023, 1, 16)))
    Debug.Print "w_mon_to_sun=" & CStr(DateDiff("w", DateSerial(2023, 1, 2), DateSerial(2023, 1, 8)))
    Debug.Print "w_sun_to_sun=" & CStr(DateDiff("w", DateSerial(2023, 1, 1), DateSerial(2023, 1, 8)))
    Debug.Print "ww_sun_start=" & CStr(DateDiff("ww", DateSerial(2023, 1, 1), DateSerial(2023, 1, 7)))
    Debug.Print "ww_mon_to_sun=" & CStr(DateDiff("ww", DateSerial(2023, 1, 2), DateSerial(2023, 1, 8)))
    Debug.Print "ww_two=" & CStr(DateDiff("ww", DateSerial(2023, 1, 2), DateSerial(2023, 1, 15)))
    Debug.Print "ww_firstMonday=" & CStr(DateDiff("ww", DateSerial(2023, 1, 1), DateSerial(2023, 1, 2), vbMonday))
    Debug.Print "ww_sunday_default=" & CStr(DateDiff("ww", DateSerial(2023, 1, 1), DateSerial(2023, 1, 2)))
End Sub
