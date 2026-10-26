' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, a date literal without a year takes the current year (the year of the host clock at run time)
' CLOCK: 2024-06-15
' EXPECT: 29
Public Sub Main()
    Debug.Print CStr(Day(#2/29#))
End Sub
