' INV: vbal.5.4.5.8.1, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 Tab(n) writes spaces up to column n
' EXPECT: [a   b]
' EXPECT: [a        b]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Print #1, "a"; Tab(5); "b"
    Print #1, "a"; Tab(10); "b"
    Close #1
    Open "/t/p.txt" For Input As #1
    Do While Not EOF(1)
        Line Input #1, s
        Debug.Print "[" & s & "]"
    Loop
    Close #1
End Sub
