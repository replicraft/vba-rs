' INV: type.collection, obj.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' ERROR: 5
Public Sub Main()
    Dim c As New Collection
    Debug.Print c("nope")
End Sub
