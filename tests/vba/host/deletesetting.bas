' INV: stmt.deletesetting, stmt.savesetting, func.getsetting
' SOURCE: spec ms-vbal 6.1.2.8.2.3 DeleteSetting (with 6.1.2.8.1.10 GetSetting, 6.1.2.8.2.4 SaveSetting)
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
