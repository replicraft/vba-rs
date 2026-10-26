' INV: stmt.dim, type.variant
' SOURCE: spec ms-vbal 5.2.3.1.5 implicit Variant type, 2.3 Empty default
' EXPECT: Empty Long
Public Sub Main()
    Dim a, b As Long
    Debug.Print TypeName(a) & " " & TypeName(b)
End Sub
