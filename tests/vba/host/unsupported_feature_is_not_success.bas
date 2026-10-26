' INV: stmt.savesetting, func.getsetting
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/savesetting-statement
' EXPECT: v
' XFAIL: unsupported the registry-backed settings statements have no host service
Public Sub Main()
    SaveSetting "app", "sec", "key", "v"
    Debug.Print GetSetting("app", "sec", "key")
End Sub
