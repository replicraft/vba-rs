' INV: stmt.for-each-next
' SOURCE: spec ms-vbal 5.4.2.4.1 array enumeration order
' EXPECT:  1 
' EXPECT:  4 
' EXPECT:  9 
Public Sub Main()
    Dim a(1 To 3) As Long, v As Variant, i As Long
    For i = 1 To 3: a(i) = i * i: Next
    For Each v In a
        Debug.Print v
    Next
End Sub
