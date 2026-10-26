' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 revision 3.0 grammar: Debug.Print #n, with no list writes a line terminator
' XFAIL: wrong pending
' EXPECT: [a]
' EXPECT: []
' EXPECT: [b]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Debug.Print #1, "a"
    Debug.Print #1,
    Debug.Print #1, "b"
    Close #1
    Open "/t/p.txt" For Input As #1
    Do While Not EOF(1)
        Line Input #1, s
        Debug.Print "[" & s & "]"
    Loop
    Close #1
End Sub
