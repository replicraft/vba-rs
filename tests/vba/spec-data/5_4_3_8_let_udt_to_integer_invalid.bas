' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 invalid Let coercion from declared type
' ERROR: load
Private Type T
    a As Long
End Type
Public Sub Main()
    Dim x As T, n As Long
    n = x
End Sub
