' INV: stmt.static
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/static-statement
' EXPECT:  1 
' EXPECT:  2 
' EXPECT:  3 
Sub Count()
    Static n As Long
    n = n + 1
    Debug.Print n
End Sub
Public Sub Main()
    Count
    Count
    Count
End Sub
