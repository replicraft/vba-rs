' INV: op.is, lit.nothing
' SOURCE: derived Is compares object references (is-operator page)
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Dim c As Collection
    Debug.Print CStr(c Is Nothing)
    Set c = New Collection
    Debug.Print CStr(c Is Nothing)
End Sub
