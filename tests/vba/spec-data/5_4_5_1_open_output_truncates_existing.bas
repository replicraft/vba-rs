' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Output replaces old data
' EXPECT: only
' EXPECT: done
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/tr.txt" For Output As #1
    Print #1, "old line one"
    Print #1, "old line two"
    Close #1
    Open "/t/tr.txt" For Output As #1
    Print #1, "only"
    Close #1
    Open "/t/tr.txt" For Input As #1
    Line Input #1, s
    Debug.Print s
    Debug.Print IIf(EOF(1), "done", "more")
    Close #1
End Sub
