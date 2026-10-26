' INV: func.dateserial
' SOURCE: derived the Learn DateSerial page: a month or day outside its range carries into the next larger unit, day 0 is the last day of the previous month
' EXPECT: 2025-02-01
' EXPECT: 2024-02-29
' EXPECT: 2023-12-31
' EXPECT: 2024-03-01
Public Sub Main()
    Debug.Print Format(DateSerial(2024, 14, 1), "yyyy-mm-dd")
    Debug.Print Format(DateSerial(2024, 3, 0), "yyyy-mm-dd")
    Debug.Print Format(DateSerial(2024, 1, 0), "yyyy-mm-dd")
    Debug.Print Format(DateSerial(2024, 2, 30), "yyyy-mm-dd")
End Sub
