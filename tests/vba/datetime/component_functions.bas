' INV: func.year, func.month, func.day, func.hour, func.minute, func.second
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  2000  3  5  13  5  9 
Public Sub Main()
    Dim d As Date
    d = #3/5/2000 1:05:09 PM#
    Debug.Print Year(d); Month(d); Day(d); Hour(d); Minute(d); Second(d)
End Sub
