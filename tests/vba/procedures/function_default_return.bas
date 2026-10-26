' INV: stmt.function
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/function-statement
' EXPECT:  0 []True
Function F() As Long
End Function
Function G() As String
End Function
Function H()
End Function
Public Sub Main()
    Debug.Print F(); "[" & G() & "]"; IsEmpty(H())
End Sub
