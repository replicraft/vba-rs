' INV: vbal.5.6.14
' SOURCE: spec ms-vbal 5.6.14 dictionary access invalid on a UDT declared type
' ERROR: load
Private Type P
    key As Long
End Type
Public Sub Main()
    Dim u As P
    Debug.Print CStr(u!key)
End Sub
