' INV: func.year, func.month, func.day, func.hour, lit.null
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: TrueTrueTrueTrue
Public Sub Main()
    Debug.Print IsNull(Year(Null)); IsNull(Month(Null)); IsNull(Day(Null)); IsNull(Hour(Null))
End Sub
