' INV: vbal.5.6.16.1
' SOURCE: spec ms-vbal 5.6.16.1 constant expression referencing a variable is invalid
' ERROR: load
Private v As Long
Private Const C As Long = v + 1
Public Sub Main()
    Debug.Print CStr(C)
End Sub
