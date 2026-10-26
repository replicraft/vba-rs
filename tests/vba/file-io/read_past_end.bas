' INV: stmt.line-input, stmt.open, stmt.close, func.eof
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/line-inputstatement
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
