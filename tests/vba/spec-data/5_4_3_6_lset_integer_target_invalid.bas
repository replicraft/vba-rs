' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: spec ms-vbal 5.4.3.6 declared type must be String, Variant or UDT
' ERROR: load
Public Sub Main()
    Dim n As Long
    LSet n = "a"
End Sub
