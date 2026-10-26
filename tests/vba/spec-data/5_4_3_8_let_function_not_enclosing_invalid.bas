' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 function assigned outside itself with non-Variant type is invalid
' ERROR: load
Private Function F() As Long
    F = 1
End Function
Public Sub Main()
    F = 5
End Sub
