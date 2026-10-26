' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, a time without a date-value uses the date 1899/12/30
' EXPECT: 1899-12-30 13:30:0
' EXPECT: 0:0:0
Public Sub Main()
    Dim d As Date
    d = #13:30#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d) & " " & Hour(d) & ":" & Minute(d) & ":" & Second(d)
    d = #1/2/2003#
    Debug.Print Hour(d) & ":" & Minute(d) & ":" & Second(d)
End Sub
