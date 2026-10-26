' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 Null output string
' EXPECT: [Null]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String, v As Variant
    v = Null
    Open "/t/p.txt" For Output As #1
    Print #1, v
    Close #1
    Open "/t/p.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
