' INV: type.collection, obj.collection
' SOURCE: spec ms-vbal 6.1.3.1.2.2 Collection Remove by key and by position
' EXPECT:  1 b
' EXPECT:  0 
Public Sub Main()
    Dim c As New Collection
    c.Add "a", "ka"
    c.Add "b"
    c.Remove "ka"
    Debug.Print c.Count; c(1)
    c.Remove 1
    Debug.Print c.Count
End Sub
