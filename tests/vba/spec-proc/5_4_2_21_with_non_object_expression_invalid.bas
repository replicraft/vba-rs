' INV: vbal.5.4.2.21, stmt.with
' SOURCE: spec ms-vbal 5.4.2.21 With requires an expression of UDT, class, Object or Variant type
' ERROR: load
Public Sub Main()
    Dim n As Long
    With n
    End With
End Sub
