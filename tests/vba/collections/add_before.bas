' INV: type.collection, obj.collection
' SOURCE: spec ms-vbal 6.1.3.1.2.1 Collection Add Before argument
' EXPECT: abc
Public Sub Main()
    Dim c As New Collection
    c.Add "a"
    c.Add "c"
    c.Add "b", , 2
    Debug.Print c(1) & c(2) & c(3)
End Sub
