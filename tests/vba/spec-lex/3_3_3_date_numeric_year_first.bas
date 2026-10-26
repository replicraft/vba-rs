' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, three numbers: year first (left not a legal month)
' EXPECT: 2003-1-2
' EXPECT: 2003-3-5
Public Sub Main()
    Dim d As Date
    d = #2003/1/2#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #2003-03-05#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
End Sub
