' INV: vbal.5.4.3.2, stmt.const
' SOURCE: spec ms-vbal 5.4.3.2 constant name must differ from other constant
' ERROR: load
Public Sub Main()
    Const k = 1
    Const k = 2
End Sub
