' INV: vbal.3.3.5.3
' SOURCE: spec ms-vbal 3.3.5.3 special identifier forms, object and [object] are used as the type Object
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim a As object
    Dim b As [object]
    Debug.Print CStr(a Is Nothing)
    Debug.Print CStr(b Is Nothing)
End Sub
