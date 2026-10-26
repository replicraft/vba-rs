' INV: vbal.5.4.5.9, stmt.write
' SOURCE: spec ms-vbal 5.4.5.9 trailing char-position keeps the comma and suppresses the terminator
' XFAIL: wrong not implemented: Write with a trailing semicolon and Spc items
' EXPECT: ["a","b"]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/w.txt" For Output As #1
    Write #1, "a";
    Write #1, "b"
    Close #1
    Open "/t/w.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
