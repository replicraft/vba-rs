' INV: func.getallsettings
' SOURCE: unverified the array shape and the empty Variant are documented; the order of the keys (sorted by name) is an assumption
' EXPECT: 1|Left=50;Top=75
' EXPECT: True
Public Sub Main()
    Dim s As Variant, i As Long, t As String
    SaveSetting appname:="MyApp", section:="Startup", key:="Top", setting:=75
    SaveSetting "MyApp", "Startup", "Left", 50
    s = GetAllSettings(appname:="MyApp", section:="Startup")
    For i = LBound(s, 1) To UBound(s, 1)
        t = t & s(i, 0) & "=" & s(i, 1) & IIf(i < UBound(s, 1), ";", "")
    Next i
    Debug.Print UBound(s, 1) & "|" & t
    ' Uninitialized Variant when the application or section does not exist.
    Debug.Print IsEmpty(GetAllSettings("NoApp", "NoSection"))
End Sub
