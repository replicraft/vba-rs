' INV: type.collection
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/collection-object
' EXPECT:  0 
Public Sub Main()
    Dim c As Collection
    Set c = New Collection
    Debug.Print c.Count
End Sub
