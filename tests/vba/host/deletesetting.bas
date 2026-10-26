' INV: stmt.deletesetting, func.getsetting, func.getallsettings
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/deletesetting-statement
' EXPECT: ok
' XFAIL: unsupported the registry-backed settings statements have no host service
Public Sub Main()
    DeleteSetting "app"
    Debug.Print "ok"
End Sub
