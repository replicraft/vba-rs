' INV: vbal.6.1.2.8.1.7, func.getallsettings
' SOURCE: spec ms-vbal 6.1.2.8.1.7 GetAllSettings returns Empty for an unknown application or section
' EXPECT: app=True
' EXPECT: section=True
Public Sub Main()
    Debug.Print "app=" & IsEmpty(GetAllSettings("VbarsNoSuchApp9", "NoSection"))
    SaveSetting "VbarsConfApp1", "Real", "K", "V"
    Debug.Print "section=" & IsEmpty(GetAllSettings("VbarsConfApp1", "NoSuchSection"))
    DeleteSetting "VbarsConfApp1"
End Sub
