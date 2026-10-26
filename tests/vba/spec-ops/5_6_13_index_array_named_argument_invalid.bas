' INV: vbal.5.6.13, expr.index
' SOURCE: spec ms-vbal 5.6.13 index on a declared array allows no named arguments
' ERROR: load
Public Sub Main()
    Dim a(1 To 3) As Long
    a(1) = 1
    Debug.Print CStr(a(i:=1))
End Sub
