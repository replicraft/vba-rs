' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 quoted text into a numeric variable sets the default value
' EXPECT: 0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim n As Long
    n = 7
    Open "/t/i.txt" For Output As #1
    Write #1, "abc"
    Close #1
    Open "/t/i.txt" For Input As #1
    Input #1, n
    Close #1
    Debug.Print CStr(n)
End Sub
