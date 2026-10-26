' INV: vbal.6.1.2.8.2.4, stmt.savesetting, func.getsetting
' SOURCE: spec ms-vbal 6.1.2.8.2.4 SaveSetting saves or creates an entry; a later save replaces the value
' EXPECT: first
' EXPECT: second
Public Sub Main()
    SaveSetting "VbarsConfApp6", "Sec", "Key", "first"
    Debug.Print GetSetting("VbarsConfApp6", "Sec", "Key")
    SaveSetting "VbarsConfApp6", "Sec", "Key", "second"
    Debug.Print GetSetting("VbarsConfApp6", "Sec", "Key")
    DeleteSetting "VbarsConfApp6"
End Sub
