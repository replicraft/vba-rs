' INV: vbal.5.4.5.7, stmt.width, stmt.print
' SOURCE: spec ms-vbal 5.4.5.7 Width sets the maximum line length; output wraps at it
' EXPECT: [abcde]
' EXPECT: [fghij]
' EXPECT: [kl]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/w.txt" For Output As #1
    Width #1, 5
    Print #1, "abcdefghijkl"
    Close #1
    Open "/t/w.txt" For Input As #1
    Do While Not EOF(1)
        Line Input #1, s
        Debug.Print "[" & s & "]"
    Loop
    Close #1
End Sub
