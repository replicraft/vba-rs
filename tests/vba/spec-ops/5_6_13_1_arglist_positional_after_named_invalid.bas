' INV: vbal.5.6.13.1
' SOURCE: spec ms-vbal 5.6.13.1 argument list grammar: positional argument cannot follow a named argument
' ERROR: load
Private Function F(ByVal a As Long, Optional ByVal b As Long = 20) As Long
    F = a + b
End Function
Public Sub Main()
    Debug.Print CStr(F(a:=1, 2))
End Sub
