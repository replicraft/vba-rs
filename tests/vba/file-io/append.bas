' INV: stmt.open, stmt.print, stmt.close, stmt.line-input
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/open-statement
' EXPECT: one
' EXPECT: two
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f As Integer, s As String
    f = FreeFile
    Open "/t/ap.txt" For Output As #f
    Print #f, "one"
    Close #f
    Open "/t/ap.txt" For Append As #f
    Print #f, "two"
    Close #f
    Open "/t/ap.txt" For Input As #f
    Do While Not EOF(f)
        Line Input #f, s: Debug.Print s
    Loop
    Close #f
End Sub
