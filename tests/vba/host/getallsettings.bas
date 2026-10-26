' INV: func.getallsettings
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/getallsettings-function
' XFAIL: unsupported registry-backed settings have no host service
Public Sub Main()
    Dim v As Variant
    v = GetAllSettings("app", "sec")
End Sub
