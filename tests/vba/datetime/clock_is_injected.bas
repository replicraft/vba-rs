' INV: func.now, func.date, func.time
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  2000  1  1 
' EXPECT:  0  0 
' EXPECT: True
Public Sub Main()
    Dim d As Date
    d = Date
    Debug.Print Year(d); Month(d); Day(d)
    Debug.Print Hour(Now); Hour(Time)
    Debug.Print Timer >= 0
End Sub
