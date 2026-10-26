' INV: type.collection, obj.collection
' SOURCE: spec ms-vbal 6.1.3.1 Collection Count, Item (default member) and Add
' EXPECT:  3 bc
Public Sub Main()
    Dim c As New Collection
    c.Add "a"
    c.Add "b"
    c.Add "c"
    Debug.Print c.Count; c.Item(2); c(3)
End Sub
