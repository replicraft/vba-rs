' INV: stmt.const
' SOURCE: spec ms-vbal 5.4.3.8 Let statement invalid on a constant
' ERROR: load
Public Sub Main()
    Const A = 1
    A = 2
End Sub
