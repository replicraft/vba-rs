' INV: vbal.5.2.3.4
' SOURCE: spec ms-vbal 5.2.3.4 private enum members accessible only within the module
' ERROR: load
Public Sub Main()
    Debug.Print CStr(Other.Hid1)
End Sub
'#MODULE Other standard
Private Enum Hid
    Hid1 = 1
End Enum
