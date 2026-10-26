' INV: stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 ReDim target must be a resizable array or Variant
' ERROR: load
Public Sub Main()
    Dim a(3) As Long
    ReDim a(5)
End Sub
