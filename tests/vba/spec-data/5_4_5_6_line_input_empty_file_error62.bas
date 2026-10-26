' INV: vbal.5.4.5.6, stmt.line-input
' SOURCE: spec ms-vbal 5.4.5.6 empty file gives error 62
' ERROR: 62
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/li.txt" For Output As #1
    Close #1
    Open "/t/li.txt" For Input As #1
    Line Input #1, s
End Sub
