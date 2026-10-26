' INV: vbal.5.4.3.2, stmt.const
' SOURCE: spec ms-vbal 5.4.3.2 constant name must differ from variable name
' ERROR: load
Public Sub Main()
    Dim k As Integer
    Const k = 1
End Sub
