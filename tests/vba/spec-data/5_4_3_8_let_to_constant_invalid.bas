' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 constant is a value, not assignable
' ERROR: load
Public Sub Main()
    Const k As Long = 1
    k = 2
End Sub
