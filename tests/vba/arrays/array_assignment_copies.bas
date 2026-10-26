' INV: decl.array-bounds
' SOURCE: spec ms-vbal 5.5.1.2.7 array Let-coercion makes a copy
' EXPECT:  1  99 
Public Sub Main()
    Dim a(1 To 2) As Long, b() As Long
    a(1) = 1
    b = a
    b(1) = 99
    Debug.Print a(1); b(1)
End Sub
