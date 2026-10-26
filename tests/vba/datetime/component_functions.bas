' INV: func.year, func.month, func.day, func.hour, func.minute, func.second
' SOURCE: spec ms-vbal 3.3.3 date tokens and 6.1.2.4.1.14 Year, 6.1.2.4.1.9 Month, 6.1.2.4.1.6 Day, 6.1.2.4.1.7 Hour, 6.1.2.4.1.8 Minute, 6.1.2.4.1.10 Second
' EXPECT:  2000  3  5  13  5  9 
Public Sub Main()
    Dim d As Date
    d = #3/5/2000 1:05:09 PM#
    Debug.Print Year(d); Month(d); Day(d); Hour(d); Minute(d); Second(d)
End Sub
