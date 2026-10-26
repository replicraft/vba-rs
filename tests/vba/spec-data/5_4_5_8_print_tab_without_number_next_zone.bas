' INV: vbal.5.4.5.8.1, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 Tab without a number advances to the next 14-column print zone
' EXPECT: a             b
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/p.txt" For Output As #1
    Print #1, "a"; Tab; "b"
    Close #1
    Open "/t/p.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print s
End Sub
