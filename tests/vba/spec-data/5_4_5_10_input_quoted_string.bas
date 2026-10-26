' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 quoted string read up to the closing quote, quotes excluded
' EXPECT: [a,b c]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a As String
    Open "/t/i.txt" For Output As #1
    Write #1, "a,b c"
    Close #1
    Open "/t/i.txt" For Input As #1
    Input #1, a
    Close #1
    Debug.Print "[" & a & "]"
End Sub
