' INV: func.weekday
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  7  2  5 
Public Sub Main()
    Debug.Print Weekday(DateSerial(2000, 1, 1)); Weekday(DateSerial(1900, 1, 1)); Weekday(DateSerial(2024, 2, 29))
End Sub
