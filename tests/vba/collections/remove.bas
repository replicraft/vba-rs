' INV: type.collection, obj.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
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
