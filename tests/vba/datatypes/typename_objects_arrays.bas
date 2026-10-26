' INV: func.typename, lit.nothing
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: NothingCollectionInteger()
' EXPECT: Variant()
' EXPECT: Error
Public Sub Main()
    Dim o As Object, a() As Long, c As New Collection, b(1) As Integer
    Debug.Print TypeName(o); TypeName(c); TypeName(b)
    Debug.Print TypeName(Array(1))
    Debug.Print TypeName(CVErr(5))
End Sub
