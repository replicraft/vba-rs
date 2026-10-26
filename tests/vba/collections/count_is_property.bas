' INV: type.collection
' SOURCE: spec ms-vbal 6.1.3.1.1.1 Collection Count
' EXPECT:  0 
Public Sub Main()
    Dim c As Collection
    Set c = New Collection
    Debug.Print c.Count
End Sub
