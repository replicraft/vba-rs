' INV: stmt.exit, stmt.function
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/exit-statement
' EXPECT:  1 
Function F() As Long
    F = 1
    Exit Function
    F = 2
End Function
Public Sub Main()
    Debug.Print F()
End Sub
