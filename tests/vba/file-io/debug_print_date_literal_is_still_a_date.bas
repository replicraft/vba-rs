' INV: stmt.print, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, a date literal is recognised where an expression operand is allowed, also after Debug.Print
' EXPECT:  2020 
' EXPECT: 5
Public Sub Main()
    Debug.Print Year(#3/4/2020#)
    Debug.Print CStr(Day(#1/5/2020#))
End Sub
