' INV: stmt.open
' SOURCE: derived paths are normalized before the policy is asked, so .. cannot escape a scope
' ERROR: 70
' GRANT: file-write=/ok/
Public Sub Main()
    Open "/ok/../other/a.txt" For Output As #1
End Sub
