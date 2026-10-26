' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 a variable named twice keeps the last assigned value
' EXPECT: def
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/i.txt" For Output As #1
    Print #1, "abc,def"
    Close #1
    Open "/t/i.txt" For Input As #1
    Input #1, s, s
    Close #1
    Debug.Print s
End Sub
