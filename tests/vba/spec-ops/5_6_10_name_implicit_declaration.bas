' INV: vbal.5.6.10
' SOURCE: spec ms-vbal 5.6.10 no match under implicit mode declares a local Variant
' EXPECT: Empty
' EXPECT: 7
' EXPECT: Integer
Public Sub Main()
    Debug.Print TypeName(fresh)
    fresh = 7
    Debug.Print CStr(fresh)
    Debug.Print TypeName(fresh)
End Sub
