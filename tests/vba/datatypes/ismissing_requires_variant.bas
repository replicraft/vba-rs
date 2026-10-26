' INV: func.ismissing, decl.optional
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/ismissing-function
' EXPECT: False
Function F(Optional n As Long) As Boolean
    F = IsMissing(n)
End Function
Public Sub Main()
    Debug.Print F()
End Sub
