' INV: vbal.5.4.5.9, stmt.write
' SOURCE: spec ms-vbal 5.4.5.9 Write without a list writes only a line terminator
' EXPECT: [1]
' EXPECT: []
' EXPECT: [2]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/w.txt" For Output As #1
    Write #1, 1
    Write #1,
    Write #1, 2
    Close #1
    Open "/t/w.txt" For Input As #1
    Do While Not EOF(1)
        Line Input #1, s
        Debug.Print "[" & s & "]"
    Loop
    Close #1
End Sub
