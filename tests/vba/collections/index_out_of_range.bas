' INV: type.collection, obj.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' ERROR: 9
Public Sub Main()
    Dim c As New Collection
    c.Add 1
    Debug.Print c(2)
End Sub
