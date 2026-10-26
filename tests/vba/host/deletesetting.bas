' INV: stmt.deletesetting, stmt.savesetting, func.getsetting
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/deletesetting-statement
' EXPECT: 75
' EXPECT: 25
' EXPECT: 25
Public Sub Main()
    SaveSetting appname:="MyApp", section:="Startup", key:="Top", setting:=75
    SaveSetting "MyApp", "Startup", "Left", 50
    Debug.Print GetSetting("MyApp", "Startup", "Top", "25")
    DeleteSetting "MyApp", "Startup", "Top"
    Debug.Print GetSetting("MyApp", "Startup", "Top", "25")
    DeleteSetting "MyApp", "Startup"
    Debug.Print GetSetting(appname:="MyApp", section:="Startup", key:="Left", default:="25")
End Sub
