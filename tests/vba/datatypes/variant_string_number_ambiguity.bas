' INV: type.variant, op.add, expr.implicit-conv
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 12
' EXPECT:  3 
Public Sub Main()
    Dim a As Variant, b As Variant
    a = "1": b = "2"
    Debug.Print a + b
    a = 1
    Debug.Print a + b
End Sub
