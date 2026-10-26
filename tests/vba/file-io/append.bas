' INV: stmt.open, stmt.print, stmt.close, stmt.line-input
' SOURCE: spec ms-vbal 5.4.5.1 Open Statement append mode (with 5.4.5.6 Line Input)
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
