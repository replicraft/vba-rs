' INV: vbal.5.4.2.3
' SOURCE: spec ms-vbal 5.4.2.3 the counter must be of numeric value type or Variant
' ERROR: load
Public Sub Main()
    Dim s As String
    For s = 1 To 3
    Next s
End Sub
