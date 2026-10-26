' INV: stmt.deletesetting
' SOURCE: unverified the page says a run-time error occurs for a non-existent section or key; error 5 is assumed
' ERROR: 5
Public Sub Main()
    DeleteSetting "NoSuchApp", "NoSection"
End Sub
