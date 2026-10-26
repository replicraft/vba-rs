' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, three numbers: left not a legal month, middle is month, left is day
' EXPECT: 2003-2-13
' EXPECT: 2003-12-25
Public Sub Main()
    Dim d As Date
    d = #13/2/2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #25/12/2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
End Sub
