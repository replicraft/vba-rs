' INV: vbal.6.1.2.7.1.2, func.isarray
' SOURCE: spec ms-vbal 6.1.2.7.1.2 IsArray true only for array data values
' EXPECT: fixed=True
' EXPECT: dynamic=True
' EXPECT: variant_holding_array=True
' EXPECT: scalar=False
' EXPECT: string=False
Public Sub Main()
    Dim a(2) As Long
    Dim d() As String
    Dim v As Variant
    ReDim d(1)
    v = Array(1, 2)
    Debug.Print "fixed=" & CStr(IsArray(a))
    Debug.Print "dynamic=" & CStr(IsArray(d))
    Debug.Print "variant_holding_array=" & CStr(IsArray(v))
    Debug.Print "scalar=" & CStr(IsArray(5))
    Debug.Print "string=" & CStr(IsArray("abc"))
End Sub
