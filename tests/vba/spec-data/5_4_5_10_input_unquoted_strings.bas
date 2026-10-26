' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 unquoted string read up to a comma; leading white space skipped
' EXPECT: [abc][def]
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a As String, b As String
    Open "/t/i.txt" For Output As #1
    Print #1, "  abc,def"
    Close #1
    Open "/t/i.txt" For Input As #1
    Input #1, a, b
    Close #1
    Debug.Print "[" & a & "][" & b & "]"
End Sub
