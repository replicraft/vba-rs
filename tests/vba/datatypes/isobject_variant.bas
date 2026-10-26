' INV: func.isobject
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/isobject-function
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Dim v As Variant
    Set v = New Collection
    Debug.Print IsObject(v)
    v = 5
    Debug.Print IsObject(v)
End Sub
