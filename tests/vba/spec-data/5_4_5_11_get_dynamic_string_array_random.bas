' INV: stmt.put, stmt.get
' SOURCE: derived ms-vbal 5.4.5.11 a dynamic String array in a Random record: the descriptor, then each string with its two-byte length
' EXPECT: ab|cde|
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a() As String, b() As String
    ReDim a(0 To 2)
    a(0) = "ab": a(1) = "cde"
    Open "/t/g.bin" For Random As #1 Len = 64
    Put #1, 1, a
    Get #1, 1, b
    Close #1
    Debug.Print b(0) & "|" & b(1) & "|" & b(2)
End Sub
