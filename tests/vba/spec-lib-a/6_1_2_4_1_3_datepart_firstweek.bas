' INV: vbal.6.1.2.4.1.3, func.datepart
' SOURCE: spec ms-vbal 6.1.2.4.1.3 DatePart ww with FirstWeekOfYear = first four days
' EXPECT: jan1_fourdays=1
' EXPECT: jan4_fourdays=1
' EXPECT: jan5_fourdays=2
' EXPECT: jan1_jan1rule=1
' EXPECT: jan5_jan1rule=2
Public Sub Main()
    ' 2020-01-01 is a Wednesday: the week Sun Dec 29 .. Sat Jan 4 has four days in 2020
    Debug.Print "jan1_fourdays=" & CStr(DatePart("ww", DateSerial(2020, 1, 1), vbSunday, vbFirstFourDays))
    Debug.Print "jan4_fourdays=" & CStr(DatePart("ww", DateSerial(2020, 1, 4), vbSunday, vbFirstFourDays))
    Debug.Print "jan5_fourdays=" & CStr(DatePart("ww", DateSerial(2020, 1, 5), vbSunday, vbFirstFourDays))
    Debug.Print "jan1_jan1rule=" & CStr(DatePart("ww", DateSerial(2020, 1, 1), vbSunday, vbFirstJan1))
    Debug.Print "jan5_jan1rule=" & CStr(DatePart("ww", DateSerial(2020, 1, 5), vbSunday, vbFirstJan1))
End Sub
