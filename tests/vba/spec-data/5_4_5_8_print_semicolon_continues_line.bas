' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 trailing semicolon suppresses the line terminator
' EXPECT: [ab]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Print #1, "a";
    Print #1, "b"
    Close #1
    Open "/t/p.txt" For Input As #1
    Line Input #1, s
    Debug.Print "[" & s & "]"
    Close #1
End Sub
