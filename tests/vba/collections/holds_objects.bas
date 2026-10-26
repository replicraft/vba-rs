' INV: type.collection, obj.collection
' SOURCE: spec ms-vbal 6.1.3.1 Collection Add accepts any type and Item default member
' EXPECT:  7 
' EXPECT:  1 
Public Sub Main()
    Dim outer As New Collection, inner As New Collection
    inner.Add 7
    outer.Add inner
    Debug.Print outer(1)(1)
    Debug.Print outer(1).Count
End Sub
