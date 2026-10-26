' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 declared type must be Variant or resizable array
' ERROR: load
Public Sub Main()
    Dim a(1 To 3) As Long
    ReDim a(5)
End Sub
