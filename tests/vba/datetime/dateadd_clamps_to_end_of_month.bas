' INV: func.dateadd, func.datepart
' SOURCE: derived the Learn DateAdd, DateDiff and DatePart pages: adding months to the 31st clamps to the last day of the target month; day differences count boundaries; June is in quarter 2
' EXPECT: 2024-02-29
' EXPECT: 2023-02-28
' EXPECT: 30
' EXPECT: 2
' EXPECT: 2024-06-22
Public Sub Main()
    Debug.Print Format(DateAdd("m", 1, DateSerial(2024, 1, 31)), "yyyy-mm-dd")
    Debug.Print Format(DateAdd("yyyy", -1, DateSerial(2024, 2, 29)), "yyyy-mm-dd")
    Debug.Print CStr(DateDiff("d", DateSerial(2024, 6, 1), DateSerial(2024, 7, 1)))
    Debug.Print CStr(DatePart("q", DateSerial(2024, 6, 15)))
    Debug.Print Format(DateAdd("ww", 1, DateSerial(2024, 6, 15)), "yyyy-mm-dd")
End Sub
