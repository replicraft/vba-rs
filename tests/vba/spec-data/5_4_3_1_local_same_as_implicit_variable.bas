' INV: vbal.5.4.3.1, stmt.dim
' SOURCE: spec ms-vbal 5.4.3.1 variable must not have the name of an implicitly declared variable
' XFAIL: wrong not implemented: conflict between a local declaration and an earlier implicit variable
' ERROR: load
Public Sub Main()
    y = 1
    Dim y As Integer
End Sub
