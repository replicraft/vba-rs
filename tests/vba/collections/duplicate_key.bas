' INV: type.collection, obj.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' ERROR: 457
Public Sub Main()
    Dim c As New Collection
    c.Add 1, "k"
    c.Add 2, "k"
End Sub
