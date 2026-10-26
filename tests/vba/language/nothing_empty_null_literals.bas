' INV: lit.nothing, lit.empty, lit.null
' SOURCE: spec ms-vbal 6.1.2.7.1.4 IsEmpty and 6.1.2.7.1.7 IsNull, Is Nothing
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim v As Variant
    v = Empty
    Debug.Print IsEmpty(v)
    v = Null
    Debug.Print IsNull(v)
    Dim o As Object
    Set o = Nothing
    Debug.Print o Is Nothing
End Sub
