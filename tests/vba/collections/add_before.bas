' INV: type.collection, obj.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' EXPECT: abc
Public Sub Main()
    Dim c As New Collection
    c.Add "a"
    c.Add "c"
    c.Add "b", , 2
    Debug.Print c(1) & c(2) & c(3)
End Sub
