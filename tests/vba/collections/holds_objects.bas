' INV: type.collection, obj.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' EXPECT:  7 
' EXPECT:  1 
Public Sub Main()
    Dim outer As New Collection, inner As New Collection
    inner.Add 7
    outer.Add inner
    Debug.Print outer(1)(1)
    Debug.Print outer(1).Count
End Sub
