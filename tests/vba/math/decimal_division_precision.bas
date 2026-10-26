' INV: type.decimal, op.div
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  0.3333333333333333333333333333 
' EXPECT: Decimal
Public Sub Main()
    Dim d As Variant
    d = CDec(1) / CDec(3)
    Debug.Print d
    Debug.Print TypeName(d)
End Sub
