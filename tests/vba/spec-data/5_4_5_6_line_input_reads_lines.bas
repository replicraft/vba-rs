' INV: vbal.5.4.5.6, stmt.line-input
' SOURCE: spec ms-vbal 5.4.5.6 Line Input reads one line without the terminator
' EXPECT: [one]
' EXPECT: [two words]
' EXPECT: []
' EXPECT: [last]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/li.txt" For Output As #1
    Print #1, "one"
    Print #1, "two words"
    Print #1, ""
    Print #1, "last"
    Close #1
    Open "/t/li.txt" For Input As #1
    Do While Not EOF(1)
        Line Input #1, s
        Debug.Print "[" & s & "]"
    Loop
    Close #1
End Sub
