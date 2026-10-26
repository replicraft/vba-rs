' INV: op.is, type.collection
' SOURCE: derived two references to one object are Is-equal
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Dim a As Collection, b As Collection, c As Collection
    Set a = New Collection
    Set b = a
    Set c = New Collection
    Debug.Print CStr(a Is b)
    Debug.Print CStr(a Is c)
End Sub
