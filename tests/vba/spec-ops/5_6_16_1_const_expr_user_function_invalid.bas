' INV: vbal.5.6.16.1
' SOURCE: spec ms-vbal 5.6.16.1 constant expression invoking a user-defined function is invalid
' ERROR: load
Private Function F() As Long
    F = 1
End Function
Private Const C As Long = F()
Public Sub Main()
    Debug.Print CStr(C)
End Sub
