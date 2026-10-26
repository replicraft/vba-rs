' INV: vbal.5.4.5.2, stmt.close
' SOURCE: spec ms-vbal 5.4.5.2 Close completes previously executed file statements
' EXPECT: data
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/a" For Output As #1
    Print #1, "data";
    Close #1
    Open "/t/a" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print s
End Sub
