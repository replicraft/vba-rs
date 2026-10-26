' INV: stmt.savesetting, func.getsetting
' SOURCE: spec ms-vbal 6.1.2.8.2.4 SaveSetting (with 6.1.2.8.1.10 GetSetting)
' EXPECT: v
Public Sub Main()
    SaveSetting "app", "sec", "key", "v"
    Debug.Print GetSetting("app", "sec", "key")
End Sub
