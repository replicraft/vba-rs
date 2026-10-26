' INV: vbal.5.4.3.4, stmt.erase
' SOURCE: spec ms-vbal 5.4.3.4 declared type must be array or Variant
' ERROR: load
Public Sub Main()
    Dim n As Long
    Erase n
End Sub
