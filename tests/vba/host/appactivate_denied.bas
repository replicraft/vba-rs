' INV: stmt.appactivate
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/appactivate-statement
' ERROR: 70
' XFAIL: unsupported AppActivate is not defined ('Sub or Function not defined')
Public Sub Main()
    AppActivate "Notepad"
End Sub
