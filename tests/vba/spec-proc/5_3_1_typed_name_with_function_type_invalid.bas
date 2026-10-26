' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 typed function name may not also have As clause
' XFAIL: wrong pending
' ERROR: load
Function F%() As Integer
    F = 1
End Function
Public Sub Main()
End Sub
