' INV: obj.err
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 5: Invalid procedure call or argument
' EXPECT: 6: Overflow
' EXPECT: 7: Out of memory
' EXPECT: 9: Subscript out of range
' EXPECT: 11: Division by zero
' EXPECT: 13: Type mismatch
' EXPECT: 53: File not found
' EXPECT: 91: Object variable or With block variable not set
Public Sub Main()
    Dim n As Variant
    On Error Resume Next
    For Each n In Array(5, 6, 7, 9, 11, 13, 53, 91)
        Err.Clear
        Err.Raise n
        Debug.Print n & ": " & Err.Description
    Next
End Sub
