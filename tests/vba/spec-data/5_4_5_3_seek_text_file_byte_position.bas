' INV: vbal.5.4.5.3, stmt.seek
' SOURCE: spec ms-vbal 5.4.5.3 Seek on a non-Random file refers to a byte; positions start at 1
' EXPECT: cdef
' EXPECT: abcdef
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/s.txt" For Output As #1
    Print #1, "abcdef"
    Close #1
    Open "/t/s.txt" For Input As #1
    Seek #1, 3
    Line Input #1, s
    Debug.Print s
    Seek #1, 1
    Line Input #1, s
    Debug.Print s
    Close #1
End Sub
