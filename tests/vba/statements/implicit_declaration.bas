' INV: stmt.dim, stmt.option-explicit
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/option-explicit-statement
' EXPECT: Integer 3 
Public Sub Main()
    y = 3
    Debug.Print TypeName(y); y
End Sub
