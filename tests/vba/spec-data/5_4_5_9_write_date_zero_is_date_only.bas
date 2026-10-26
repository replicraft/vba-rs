' INV: vbal.5.4.5.9, stmt.write
' SOURCE: spec ms-vbal 5.4.5.9 date 1899-12-30 at 00:00:00 prints only the date part
' EXPECT: [#1899-12-30#]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String, d As Date
    d = 0
    Open "/t/w.txt" For Output As #1
    Write #1, d
    Close #1
    Open "/t/w.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "[" & s & "]"
End Sub
