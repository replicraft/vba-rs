' INV: vbal.5.4.2.5, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.5 Exit For must be lexically inside a For or For Each
' ERROR: load
Public Sub Main()
    Exit For
End Sub
