' INV: stmt.for-each-next, decl.array-bounds
' SOURCE: spec ms-vbal 5.4.2.4.1 array enumeration order (leftmost index varies fastest)
' EXPECT:  11 
' EXPECT:  21 
' EXPECT:  12 
' EXPECT:  22 
Public Sub Main()
    Dim m(1 To 2, 1 To 2) As Long, v As Variant, i As Long, j As Long
    For i = 1 To 2: For j = 1 To 2: m(i, j) = i * 10 + j: Next: Next
    For Each v In m
        Debug.Print v
    Next
End Sub
