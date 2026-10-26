' INV: vbal.6.1.2.7.1.9, func.isobject
' SOURCE: spec ms-vbal 6.1.2.7.1.9 IsObject true for object reference values
' EXPECT: collection=True
' EXPECT: variant_obj=True
' EXPECT: number=False
' EXPECT: string=False
Public Sub Main()
    Dim c As Collection
    Dim v As Variant
    Set c = New Collection
    Set v = c
    Debug.Print "collection=" & CStr(IsObject(c))
    Debug.Print "variant_obj=" & CStr(IsObject(v))
    Debug.Print "number=" & CStr(IsObject(5))
    Debug.Print "string=" & CStr(IsObject("x"))
End Sub
