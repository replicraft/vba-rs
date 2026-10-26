' INV: vbal.5.6.13, expr.index
' SOURCE: spec ms-vbal 5.6.13 index on a declared array needs positional arguments equal to its rank
' ERROR: load
Public Sub Main()
    Dim a(1 To 3) As Long
    a(1) = 1
    Debug.Print CStr(a(1, 2))
End Sub
