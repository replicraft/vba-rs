' INV: type.variant, lit.null
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: TrueFalse
Public Sub Main()
    Dim v As Variant
    v = Null
    Debug.Print IsNull(v + 1); IsNull(v & "a")
End Sub
