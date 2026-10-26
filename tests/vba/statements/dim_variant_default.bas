' INV: stmt.dim, type.variant
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dim-statement
' EXPECT: Empty Long
Public Sub Main()
    Dim a, b As Long
    Debug.Print TypeName(a) & " " & TypeName(b)
End Sub
