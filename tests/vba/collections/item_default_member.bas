' INV: type.collection, expr.default-member
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' EXPECT: True
Public Sub Main()
    Dim c As New Collection
    c.Add "z"
    Debug.Print c(1) = c.Item(1)
End Sub
