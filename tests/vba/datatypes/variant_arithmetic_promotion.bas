' INV: type.variant, op.add
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic overflow with Variant operands
' EXPECT: Long 60000 
Public Sub Main()
    Dim a As Variant, b As Variant
    a = 30000: b = 30000
    Debug.Print TypeName(a + b); a + b
End Sub
