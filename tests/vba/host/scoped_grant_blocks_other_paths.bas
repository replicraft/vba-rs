' INV: stmt.open
' SOURCE: derived a scoped grant does not cover other paths
' ERROR: 70
' GRANT: file-write=/ok
Public Sub Main()
    Open "/other/a.txt" For Output As #1
End Sub
