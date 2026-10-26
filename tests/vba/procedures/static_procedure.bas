' INV: stmt.static, stmt.sub
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/static-statement
' EXPECT:  1 
' EXPECT:  2 
Static Sub Count()
    Dim n As Long
    n = n + 1
    Debug.Print n
End Sub
Public Sub Main()
    Count
    Count
End Sub
