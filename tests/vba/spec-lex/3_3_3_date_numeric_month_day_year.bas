' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, three numbers: legal month first, then day, then year
' EXPECT: 2003-1-2
' EXPECT: 2004-2-29
' EXPECT: 2003-12-31
' EXPECT: Date
Public Sub Main()
    Dim d As Date
    d = #1/2/2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #2/29/2004#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #12-31-2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    Debug.Print TypeName(#1/2/2003#)
End Sub
