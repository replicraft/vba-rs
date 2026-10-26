' INV: vbal.5.6.13.1
' SOURCE: spec ms-vbal 5.6.13.1 argument list grammar requires a required positional argument after the last comma
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: a trailing comma in an argument list is accepted
' ERROR: load
Private Function F(ByVal a As Long, Optional ByVal b As Long = 20) As Long
    F = a + b
End Function
Public Sub Main()
    Debug.Print CStr(F(1, ))
End Sub
