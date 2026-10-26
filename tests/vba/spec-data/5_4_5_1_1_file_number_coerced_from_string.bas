' INV: vbal.5.4.5.1.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1.1 file number is Let-coerced to Integer
' EXPECT: seen
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String, r As String
    s = "4"
    Open "/t/f" For Output As #s
    Print #4, "seen"
    Close #4
    Open "/t/f" For Input As #4
    Line Input #4, r
    Close #4
    Debug.Print r
End Sub
