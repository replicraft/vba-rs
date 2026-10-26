' INV: type.collection, obj.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' EXPECT:  1 
Public Sub Main()
    Dim c As New Collection
    c.Add 1, "Key"
    Debug.Print c("KEY")
End Sub
