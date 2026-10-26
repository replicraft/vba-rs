' INV: stmt.open, stmt.close, stmt.mkdir
' SOURCE: derived a scoped grant covers its prefix
' EXPECT: written
' GRANT: file-write=/ok
' GRANT: file-read=/ok
Public Sub Main()
    MkDir "/ok"
    Open "/ok/a.txt" For Output As #1
    Close #1
    Debug.Print "written"
End Sub
