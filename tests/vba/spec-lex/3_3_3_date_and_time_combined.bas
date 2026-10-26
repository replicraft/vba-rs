' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, date-value and time-value separated by whitespace; dot time separator
' EXPECT: 2003-1-2 15:4:5
' EXPECT: 2003-3-5 8:15:0
Public Sub Main()
    Dim d As Date
    d = #1/2/2003 3:04:05 PM#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d) & " " & Hour(d) & ":" & Minute(d) & ":" & Second(d)
    d = #March 5, 2003 8:15#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d) & " " & Hour(d) & ":" & Minute(d) & ":" & Second(d)
End Sub
