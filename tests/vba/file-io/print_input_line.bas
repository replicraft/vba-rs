' INV: stmt.open, stmt.close, stmt.print, stmt.line-input, func.eof, func.freefile
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/line-inputstatement
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
