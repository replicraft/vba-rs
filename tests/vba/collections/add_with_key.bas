' INV: type.collection, obj.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' EXPECT: pearapple
Public Sub Main()
    Dim c As New Collection
    c.Add "apple", "k1"
    c.Add "pear", "k2"
    Debug.Print c("k2"); c.Item("K1")
End Sub
