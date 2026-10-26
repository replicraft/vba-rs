' INV: vbal.2.3
' SOURCE: spec ms-vbal 2.3 variables, the initial value of a Date variable is 30 December 1899 00:00:00
' EXPECT: 1899-12-30 0:0:0
Public Sub Main()
    Dim d As Date
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d) & " " & Hour(d) & ":" & Minute(d) & ":" & Second(d)
End Sub
