' INV: func.input
' SOURCE: unverified the Binary read follows the Input page; error 54 (wrong mode) and 5 (FileAttr return type 2) are assumptions
' EXPECT: a,b
' EXPECT: 54
' EXPECT: 5
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/b.txt" For Output As #1
    Print #1, "a,b";
    Close #1
    Open "/b.txt" For Binary As #1
    Debug.Print Input(3, #1)
    Close #1
    On Error Resume Next
    Open "/b.txt" For Output As #1
    Dim s As String
    s = Input(1, #1)
    Debug.Print Err.Number & ""
    Close #1
    Open "/b.txt" For Input As #1
    Err.Clear
    s = FileAttr(1, 2)
    Debug.Print Err.Number & ""
    Close #1
End Sub
