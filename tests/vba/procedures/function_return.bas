' INV: stmt.function
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/function-statement
' EXPECT:  5 
Function Add(a As Long, b As Long) As Long
    Add = a + b
End Function
Public Sub Main()
    Debug.Print Add(2, 3)
End Sub
