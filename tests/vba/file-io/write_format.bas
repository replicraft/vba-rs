' INV: stmt.write, stmt.open, stmt.close, stmt.line-input
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/writestatement
' EXPECT: "a",1,#TRUE#,#2000-01-02#
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim f As Integer, s As String
    f = FreeFile
    Open "/t/w.txt" For Output As #f
    Write #f, "a", 1, True, #1/2/2000#
    Close #f
    Open "/t/w.txt" For Input As #f
    Line Input #f, s
    Close #f
    Debug.Print s
End Sub
