' INV: vbal.5.4.5.8.1, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 Tab(n) with n at or before the current column starts a new line
' EXPECT: [abcde]
' EXPECT: [  x]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Print #1, "abcde"; Tab(3); "x"
    Close #1
    Open "/t/p.txt" For Input As #1
    Do While Not EOF(1)
        Line Input #1, s
        Debug.Print "[" & s & "]"
    Loop
    Close #1
End Sub
