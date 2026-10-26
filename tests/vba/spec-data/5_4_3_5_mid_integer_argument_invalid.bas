' INV: vbal.5.4.3.5, stmt.mid
' SOURCE: spec ms-vbal 5.4.3.5 declared type of string-argument must be String or Variant
' ERROR: load
Public Sub Main()
    Dim n As Long
    n = 12345
    Mid(n, 1, 1) = "9"
End Sub
