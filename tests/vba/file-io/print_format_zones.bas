' INV: stmt.print, stmt.open, stmt.close, stmt.line-input
' SOURCE: spec ms-vbal 5.4.5.8 Print Statement print zones and numeric output
' EXPECT: [a             b]
' EXPECT: [ 5  6 ]
' EXPECT: [xy]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f As Integer, s As String
    f = FreeFile
    Open "/t/p.txt" For Output As #f
    Print #f, "a", "b"
    Print #f, 5; 6
    Print #f, "x";
    Print #f, "y"
    Close #f
    Open "/t/p.txt" For Input As #f
    Do While Not EOF(f)
        Line Input #f, s
        Debug.Print "[" & s & "]"
    Loop
    Close #f
End Sub
