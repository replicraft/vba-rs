' INV: func.cdate
' SOURCE: derived a number converts to a date, fraction is the time (type-conversion-functions page); serial 36526 is 2000-01-01
' EXPECT:  2000  1  1  12 
Public Sub Main()
    Dim d As Date
    d = CDate(36526.5)
    Debug.Print Year(d); Month(d); Day(d); Hour(d)
End Sub
