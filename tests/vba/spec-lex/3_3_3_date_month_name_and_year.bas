' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, month name and a number that is not a legal day: day 1, number is the year
' EXPECT: 2003-3-1
' EXPECT: 2003-9-1
' EXPECT: 2003-5-1
Public Sub Main()
    Dim d As Date
    d = #Mar 2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #2003 September#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #may-2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
End Sub
