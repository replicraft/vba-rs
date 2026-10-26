' INV: lit.nothing, lit.empty, lit.null
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
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
