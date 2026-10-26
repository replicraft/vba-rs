' INV: type.object, stmt.set
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT: Collection
' EXPECT: Nothing
Public Sub Main()
    Dim o As Object
    Set o = New Collection
    Debug.Print TypeName(o)
    Set o = Nothing
    Debug.Print TypeName(o)
End Sub
