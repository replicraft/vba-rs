' INV: lit.date, type.date
' SOURCE: spec ms-vbal 3.3.3 date tokens (date, time and time-only literals)
' EXPECT:  1  2 
' EXPECT:  15  4  5 
' EXPECT:  17  30  1899 
Public Sub Main()
    Dim d As Date
    d = #1/2/2000#
    Debug.Print Month(d); Day(d)
    d = #1/2/2000 3:04:05 PM#
    Debug.Print Hour(d); Minute(d); Second(d)
    d = #5:30:00 PM#
    Debug.Print Hour(d); Minute(d); Year(d)
End Sub
