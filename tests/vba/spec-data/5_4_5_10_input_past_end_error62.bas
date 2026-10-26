' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 reading beyond the end of the file is error 62
' ERROR: 62
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/i.txt" For Output As #1
    Close #1
    Open "/t/i.txt" For Input As #1
    Input #1, s
End Sub
