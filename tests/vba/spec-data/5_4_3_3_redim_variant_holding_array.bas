' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 Variant variable holding an array can be redimensioned
' EXPECT: 4
Public Sub Main()
    Dim v As Variant
    v = Array(1, 2)
    ReDim v(4)
    Debug.Print CStr(UBound(v))
End Sub
