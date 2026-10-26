' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 Print without an output list writes the line terminator only
' EXPECT: [a]
' EXPECT: []
' EXPECT: [b]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Print #1, "a"
    Print #1,
    Print #1, "b"
    Close #1
    Open "/t/p.txt" For Input As #1
    Do While Not EOF(1)
        Line Input #1, s
        Debug.Print "[" & s & "]"
    Loop
    Close #1
End Sub
