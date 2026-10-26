' INV: vbal.5.4.3.2, stmt.const
' SOURCE: spec ms-vbal 5.4.3.2 constant must not have the name of an implicit variable
' ERROR: load
Public Sub Main()
    y = 1
    Const y = 2
End Sub
