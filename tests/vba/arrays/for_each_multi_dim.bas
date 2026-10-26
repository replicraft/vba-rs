' INV: stmt.for-each-next, decl.array-bounds
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/for-eachnext-statement
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
