' INV: vbal.5.2.3.2
' SOURCE: spec ms-vbal 5.2.3.2 private constant accessible only within module
' ERROR: load
Public Sub Main()
    Debug.Print CStr(Other.Q)
End Sub
'#MODULE Other standard
Private Const Q As Long = 20
