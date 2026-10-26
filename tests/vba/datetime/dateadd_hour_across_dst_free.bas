' INV: func.dateadd
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 2000-04-01 13:00
Public Sub Main()
    Debug.Print Format(DateAdd("h", 25, DateSerial(2000, 3, 31) + TimeSerial(12, 0, 0)), "yyyy-mm-dd hh:nn")
End Sub
