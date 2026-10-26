' INV: stmt.static, stmt.function
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/static-statement
' EXPECT:  3 
Function Depth() As Long
    Static n As Long
    n = n + 1
    If n < 3 Then Depth = Depth() Else Depth = n
End Function
Public Sub Main()
    Debug.Print Depth()
End Sub
