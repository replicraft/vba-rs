' INV: func.imestatus, vbal.6.1.2.7.1.1
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/imestatus-function
' METHOD: the page lists vbIMEModeNoControl = 0 for a host without an IME; VarType 2 is vbInteger; by hand
' EXPECT: Integer
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print TypeName(IMEStatus)
    Debug.Print IMEStatus = vbIMEModeNoControl
    Debug.Print IMEStatus() = 0
End Sub
