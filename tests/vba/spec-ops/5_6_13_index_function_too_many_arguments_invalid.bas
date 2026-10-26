' INV: vbal.5.6.13, expr.index
' SOURCE: spec ms-vbal 5.6.13 index on a function requires a compatible parameter list
' ERROR: load
Private Function F(ByVal a As Long) As Long
    F = a
End Function
Public Sub Main()
    Debug.Print CStr(F(1, 2))
End Sub
