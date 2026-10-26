' INV: vbal.5.4.3.1, stmt.dim
' SOURCE: spec ms-vbal 5.4.3.1 variable name must differ from constant name
' ERROR: load
Public Sub Main()
    Const k = 1
    Dim k As Integer
End Sub
