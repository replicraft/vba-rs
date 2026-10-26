' INV: func.year, func.month, func.day, func.hour, lit.null
' SOURCE: spec ms-vbal 6.1.2.4.1.14 Year, Month, Day, Hour return Null for Null
' EXPECT: TrueTrueTrueTrue
Public Sub Main()
    Debug.Print IsNull(Year(Null)); IsNull(Month(Null)); IsNull(Day(Null)); IsNull(Hour(Null))
End Sub
