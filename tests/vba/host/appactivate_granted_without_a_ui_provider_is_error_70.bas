' INV: stmt.appactivate, vbal.6.1.2.8.2.1
' SOURCE: derived the embedding application grants ui-automation but supplies no window provider: the statement is refused with error 70 instead of reporting success
' METHOD: by hand
' GRANT: ui-automation
' ERROR: 70
Public Sub Main()
    AppActivate "Untitled - Notepad", True
End Sub
