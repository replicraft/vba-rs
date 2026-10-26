' INV: type.collection, obj.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' EXPECT:  3 bc
Public Sub Main()
    Dim c As New Collection
    c.Add "a"
    c.Add "b"
    c.Add "c"
    Debug.Print c.Count; c.Item(2); c(3)
End Sub
