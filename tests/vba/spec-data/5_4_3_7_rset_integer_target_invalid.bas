' INV: vbal.5.4.3.7, stmt.rset
' SOURCE: spec ms-vbal 5.4.3.7 declared type must be String or Variant
' ERROR: load
Public Sub Main()
    Dim n As Long
    RSet n = "a"
End Sub
