' INV: vbal.5.6.13, expr.index
' SOURCE: spec ms-vbal 5.6.13 index of a Long-returning function that takes no parameters with arguments is invalid
' ERROR: load
Private Function F() As Long
    F = 1
End Function
Public Sub Main()
    Debug.Print CStr(F(1))
End Sub
