' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 closed file number can be reused
' EXPECT: reused
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/x1" For Output As #3
    Print #3, "one"
    Close #3
    Open "/t/x2" For Output As #3
    Print #3, "reused"
    Close #3
    Open "/t/x2" For Input As #3
    Line Input #3, s
    Close #3
    Debug.Print s
End Sub
