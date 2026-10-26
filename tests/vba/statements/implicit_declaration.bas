' INV: stmt.dim, stmt.option-explicit
' SOURCE: spec ms-vbal 5.6.10 implicit declaration in implicit-mode, 5.2.3.1.5 implicit Variant type
' EXPECT: Integer 3 
Public Sub Main()
    y = 3
    Debug.Print TypeName(y); y
End Sub
