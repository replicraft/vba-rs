' INV: vbal.5.4.5.8.1, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8.1 missing char-position acts as semicolon
' EXPECT: [xyz]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Print #1, "x"; "y" "z"
    Close #1
    Open "/t/p.txt" For Input As #1
    Line Input #1, s
    Debug.Print "[" & s & "]"
    Close #1
End Sub
