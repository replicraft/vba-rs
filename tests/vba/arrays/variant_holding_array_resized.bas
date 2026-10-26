' INV: stmt.redim, type.variant
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: x 3 
Public Sub Main()
    Dim v As Variant
    ReDim v(1 To 2)
    v(1) = "x"
    ReDim Preserve v(1 To 3)
    Debug.Print v(1); UBound(v)
End Sub
