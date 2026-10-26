' INV: vbal.5.4.5.7, stmt.width, stmt.print
' SOURCE: spec ms-vbal 5.4.5.7 width 0 means no maximum line length
' EXPECT: 300
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/w.txt" For Output As #1
    Width #1, 10
    Width #1, 0
    Print #1, String(300, "x")
    Close #1
    Open "/t/w.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print CStr(Len(s))
End Sub
