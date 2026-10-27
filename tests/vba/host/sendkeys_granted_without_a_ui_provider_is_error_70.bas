' INV: stmt.sendkeys, vbal.6.1.2.8.2.5
' SOURCE: derived the embedding application grants ui-automation but supplies no keyboard provider: the statement is refused with error 70 instead of reporting success
' METHOD: by hand
' GRANT: ui-automation
' ERROR: 70
Public Sub Main()
    SendKeys "{ENTER}", True
End Sub
