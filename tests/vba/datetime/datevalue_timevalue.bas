' INV: func.datevalue, func.timevalue
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  2000  3  5 
' EXPECT:  13  5  9 
' EXPECT: True
Public Sub Main()
    Dim d As Date
    d = DateValue("March 5, 2000")
    Debug.Print Year(d); Month(d); Day(d)
    d = TimeValue("1:05:09 PM")
    Debug.Print Hour(d); Minute(d); Second(d)
    Debug.Print CDbl(DateValue("March 5, 2000 6:00 PM")) = CDbl(DateSerial(2000, 3, 5))
End Sub
