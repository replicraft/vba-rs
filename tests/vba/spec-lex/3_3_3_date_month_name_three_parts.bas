' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, month name plus two numbers in any order, separators space comma slash dash
' EXPECT: 2003-3-5
' EXPECT: 2003-3-5
' EXPECT: 2003-3-5
' EXPECT: 2003-3-5
' EXPECT: 2003-12-25
' EXPECT: 2003-1-4
Public Sub Main()
    Dim d As Date
    d = #March 5, 2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #5 March 2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #2003 Mar 5#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #mar/5/2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #DECEMBER 25 2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
    d = #4-JAN-2003#
    Debug.Print Year(d) & "-" & Month(d) & "-" & Day(d)
End Sub
