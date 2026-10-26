' INV: stmt.redim
' SOURCE: unverified an unallocated dynamic array has no dimensions, so its descriptor is the two-byte count alone (the 2 + 8 * dimensions formula with zero dimensions); real VBA may fail instead
' EXPECT: 2 deallocated
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a() As Long, b() As Long
    ReDim b(1 To 3)
    Open "/t/g.bin" For Binary As #1
    Put #1, , a
    Dim s As String
    s = CStr(LOF(1))
    Seek #1, 1
    Get #1, , b
    Close #1
    On Error Resume Next
    Dim n As Long
    n = UBound(b)
    If Err.Number = 9 Then Debug.Print s & " deallocated" Else Debug.Print s & " allocated"
End Sub
