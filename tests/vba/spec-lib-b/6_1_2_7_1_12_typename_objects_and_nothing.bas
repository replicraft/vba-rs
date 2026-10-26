' INV: vbal.6.1.2.7.1.12, func.typename
' SOURCE: spec ms-vbal 6.1.2.7.1.12 TypeName for Nothing and for an object of a named type
' EXPECT: Nothing
' EXPECT: Collection
Public Sub Main()
    Dim o As Object
    Dim c As Collection
    Set c = New Collection
    Debug.Print TypeName(o)
    Debug.Print TypeName(c)
End Sub
