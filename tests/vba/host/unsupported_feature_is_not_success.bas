' INV: stmt.savesetting, func.getsetting
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/savesetting-statement
' EXPECT: v
Public Sub Main()
    SaveSetting "app", "sec", "key", "v"
    Debug.Print GetSetting("app", "sec", "key")
End Sub
