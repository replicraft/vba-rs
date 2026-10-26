' INV: stmt.open, stmt.close, stmt.print, stmt.line-input, func.eof, func.freefile
' SOURCE: spec ms-vbal 5.4.5.6 Line Input Statement (with 5.4.5.8 Print, 6.1.2.5.1.3 EOF)
' EXPECT: line one
' EXPECT: line two
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f As Integer, s As String
    f = FreeFile
    Open "/t/a.txt" For Output As #f
    Print #f, "line one"
    Print #f, "line two"
    Close #f
    f = FreeFile
    Open "/t/a.txt" For Input As #f
    Do While Not EOF(f)
        Line Input #f, s
        Debug.Print s
    Loop
    Close #f
End Sub
