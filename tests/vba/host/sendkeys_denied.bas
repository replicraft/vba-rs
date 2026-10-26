' INV: stmt.sendkeys
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/sendkeys-statement
' ERROR: 70
' XFAIL: unsupported SendKeys is not defined ('Sub or Function not defined'); with UiAutomation as the gating capability it should be error 70
Public Sub Main()
    SendKeys "abc"
End Sub
