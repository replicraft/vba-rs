' INV: type.variant, op.add
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: Long 60000 
Public Sub Main()
    Dim a As Variant, b As Variant
    a = 30000: b = 30000
    Debug.Print TypeName(a + b); a + b
End Sub
