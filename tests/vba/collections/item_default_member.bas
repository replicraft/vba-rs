' INV: type.collection, expr.default-member
' SOURCE: spec ms-vbal 6.1.3.1.1.2 Collection Item is the default member
' EXPECT: True
Public Sub Main()
    Dim c As New Collection
    c.Add "z"
    Debug.Print c(1) = c.Item(1)
End Sub
