' INV: vbal.5.6.16.1
' SOURCE: spec ms-vbal 5.6.16.1 constant expression may reference only procedure constants of the enclosing procedure
' ERROR: load
Private Sub Other()
    Const Hidden As Long = 3
End Sub
Private Const C As Long = Hidden + 1
Public Sub Main()
    Debug.Print CStr(C)
End Sub
