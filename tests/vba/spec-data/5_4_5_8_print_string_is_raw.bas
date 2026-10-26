' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 other values are written as their String coercion
' EXPECT: [say "hi"]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Print #1, "say " & Chr(34) & "hi" & Chr(34)
    Close #1
    Open "/t/p.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
