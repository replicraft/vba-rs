' INV: vbal.5.6.12
' SOURCE: spec ms-vbal 5.6.12 member access on a UDT without that field is invalid
' ERROR: load
Private Type P
    X As Long
End Type
Public Sub Main()
    Dim u As P
    Debug.Print CStr(u.Z)
End Sub
