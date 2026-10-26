' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, a date literal without a year takes the current year (the year of the host clock at run time)
' CLOCK: 2024-06-15
' EXPECT: 2024-3-4 10:30
Public Sub Main()
    Dim d As Date
    d = #3/4 10:30 AM#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d) & " " & Hour(d) & ":" & Minute(d)
End Sub
