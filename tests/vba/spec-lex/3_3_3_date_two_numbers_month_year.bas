' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, two numbers that are not a month/day: month and year, day 1
' XFAIL: wrong pending
' EXPECT: 2003-12-1
' EXPECT: 2003-12-1
Public Sub Main()
    Dim d As Date
    d = #12/2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #2003/12#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
End Sub
