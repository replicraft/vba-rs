' INV: vbal.5.4.3.4, stmt.erase
' SOURCE: spec ms-vbal 5.4.3.4 Variant array value becomes an empty array of same element type
' EXPECT: isarray=True
' ERROR: 9
Public Sub Main()
    Dim v As Variant
    v = Array(1, 2, 3)
    Erase v
    Debug.Print "isarray=" & CStr(IsArray(v))
    Debug.Print CStr(UBound(v))
End Sub
