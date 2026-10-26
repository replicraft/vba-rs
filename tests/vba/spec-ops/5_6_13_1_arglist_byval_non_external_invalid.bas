' INV: vbal.5.6.13.1
' SOURCE: spec ms-vbal 5.6.13.1 ByVal in an argument list is invalid unless calling an external procedure
' ERROR: load
Private Function F(ByVal a As Long) As Long
    F = a
End Function
Public Sub Main()
    Dim n As Long
    n = 1
    Debug.Print CStr(F(ByVal n))
End Sub
