' INV: type.variant, lit.empty
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  1 
' EXPECT: []
' EXPECT: TrueTrue
Public Sub Main()
    Dim v As Variant
    Debug.Print v + 1
    Debug.Print "[" & v & "]"
    Debug.Print v = 0; v = ""
End Sub
