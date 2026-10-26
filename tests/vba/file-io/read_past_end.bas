' INV: stmt.line-input, stmt.open, stmt.close, func.eof
' SOURCE: spec ms-vbal 5.4.5.6 Line Input Statement error 62 on an empty file
' ERROR: 62
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f As Integer, s As String
    f = FreeFile
    Open "/t/e.txt" For Output As #f
    Close #f
    Open "/t/e.txt" For Input As #f
    Line Input #f, s
End Sub
