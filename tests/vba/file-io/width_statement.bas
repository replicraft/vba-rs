' INV: stmt.width, stmt.print, stmt.open, stmt.line-input
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/widthstatement
' EXPECT: short
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim s As String
    Open "/wd.txt" For Output As #1
    Width #1, 80
    Print #1, "short"
    Close #1
    Open "/wd.txt" For Input As #1
    Line Input #1, s
    Debug.Print s
    Close #1
End Sub
